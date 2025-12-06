use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

use compiler::compiler::{
    ast::{Item, TypeRef},
    compile,
    hir::{
        lower_entry, lower_function, normalize_type_alias, ConstantValue, Env, EnvEntry, Function,
    },
    lexer::Lexer,
    mir::MirModule,
    parser::Parser,
    span::Span,
    symbol::SymbolRegistry,
    test_utils::render_normalized_rgo,
};

const GENERATED_DIR: &str = "tests/generated";

#[test]
fn golden_test() {
    generate_golden_snapshots();
    verify_expected_outputs();
}

fn generate_golden_snapshots() {
    let src_dir = Path::new("tests");
    let out_dir = Path::new(GENERATED_DIR);
    fs::create_dir_all(out_dir).expect("failed to create generated output directory");

    let mut rgo_paths: Vec<PathBuf> = fs::read_dir(src_dir)
        .expect("tests directory should exist")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("rgo") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    rgo_paths.sort();

    for path in rgo_paths {
        if let Err(err) = build_reference_for_path(&path, out_dir) {
            panic!("{}: {err}", path.display());
        }
    }
}

fn collect_items(source: &str) -> Result<(Vec<Item>, SymbolRegistry), String> {
    let cursor = Cursor::new(source.as_bytes());
    let lexer = Lexer::new(cursor);
    let mut parser = Parser::new(lexer);
    let mut symbols = SymbolRegistry::new();
    let mut items = Vec::new();

    while let Some(item) = parser
        .next(&mut symbols)
        .map_err(|err| format!("parsing failed: {err}"))?
    {
        items.push(item);
    }

    Ok((items, symbols))
}

fn lower_items(items: Vec<Item>, symbols: &mut SymbolRegistry) -> Result<Vec<Function>, String> {
    let mut functions = Vec::new();
    let mut env = Env::new();
    let mut entry_prelude: Vec<Item> = Vec::new();
    let mut entry_items: Vec<Item> = Vec::new();

    for item in items {
        match item {
            Item::FunctionDef { .. } => {
                let (function, nested) = lower_function(item, symbols, &env)
                    .map_err(|err| format!("HIR lowering failed: {err}"))?;
                functions.push(function);
                functions.extend(nested);
            }
            Item::Ident(ident) => {
                entry_items.push(Item::Ident(ident));
            }
            Item::Lambda(lambda) => {
                entry_items.push(Item::Lambda(lambda));
            }
            Item::ScopeCapture {
                params,
                term,
                span: capture_span,
            } => {
                entry_items.push(Item::ScopeCapture {
                    params,
                    term,
                    span: capture_span,
                });
            }
            Item::StrDef {
                name,
                literal,
                span,
            } => {
                entry_prelude.push(Item::StrDef {
                    name: name.clone(),
                    literal: literal.clone(),
                    span,
                });
                env.insert(
                    name,
                    EnvEntry {
                        ty: TypeRef::Str,
                        span,
                        constant: Some(ConstantValue::Str(literal.value.clone())),
                    },
                );
            }
            Item::IntDef {
                name,
                literal,
                span,
            } => {
                entry_prelude.push(Item::IntDef {
                    name: name.clone(),
                    literal: literal.clone(),
                    span,
                });
                env.insert(
                    name,
                    EnvEntry {
                        ty: TypeRef::Int,
                        span,
                        constant: Some(ConstantValue::Int(literal.value)),
                    },
                );
            }
            Item::TypeDef { name, .. } => {
                normalize_type_alias(&name, symbols)
                    .map_err(|err| format!("type normalization failed: {err}"))?;
            }
            Item::IdentDef { name, ident, span } => {
                entry_prelude.push(Item::IdentDef { name, ident, span });
            }
            Item::Import { .. } => {}
        }
    }

    if !entry_items.is_empty() {
        let span = entry_item_span(entry_items.last().unwrap());
        if let Some(entry_funcs) = lower_entry(entry_prelude.clone(), entry_items, span, symbols)
            .map_err(|err| format!("HIR lowering failed: {err}"))?
        {
            for entry in entry_funcs {
                let (function, nested) = lower_function(entry, symbols, &env)
                    .map_err(|err| format!("HIR lowering failed: {err}"))?;
                functions.push(function);
                functions.extend(nested);
            }
        }
    }

    Ok(functions)
}

fn entry_item_span(item: &Item) -> Span {
    match item {
        Item::Ident(ident) => ident.span,
        Item::Lambda(lambda) => lambda.span,
        Item::ScopeCapture { span, .. } => *span,
        _ => Span::unknown(),
    }
}

fn compile_source(source: &str) -> Result<(String, MirModule), String> {
    let mut output = Vec::new();
    let cursor = Cursor::new(source.as_bytes());
    let metadata =
        compile(cursor, &mut output).map_err(|err| format!("assembly generation failed: {err}"))?;
    let asm =
        String::from_utf8(output).map_err(|err| format!("assembly is not valid UTF-8: {err}"))?;
    Ok((asm, metadata.mir_module))
}

fn build_reference_for_path(path: &Path, out_dir: &Path) -> Result<(), String> {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| "filename should be valid UTF-8".to_string())?;
    let source = fs::read_to_string(path).map_err(|err| format!("reading source failed: {err}"))?;
    let err_path = out_dir.join(format!("{stem}.err"));
    let expect_error = err_path.exists();

    match generate_artifacts(&source) {
        Ok(artifacts) => {
            if expect_error {
                return Err(format!(
                    "{}: expected compilation failure but succeeded",
                    path.display()
                ));
            }
            write_artifacts(out_dir, stem, &artifacts)?;
            if err_path.exists() {
                fs::remove_file(&err_path)
                    .map_err(|err| format!("removing stale error snapshot failed: {err}"))?;
            }
            Ok(())
        }
        Err(err) => {
            if expect_error {
                cleanup_generated_artifacts(out_dir, stem)?;
                fs::write(&err_path, format!("{err}\n"))
                    .map_err(|write_err| format!(
                        "writing expected error snapshot failed: {write_err}"
                    ))?;
                Ok(())
            } else {
                Err(err)
            }
        }
    }
}

struct GeneratedArtifacts {
    parser_output: String,
    normalized_hir: String,
    hir_debug: String,
    asm: String,
    mir_pretty: String,
    mir_debug: String,
}

fn generate_artifacts(source: &str) -> Result<GeneratedArtifacts, String> {
    let (items, mut symbols) =
        collect_items(source).map_err(|err| format!("parser step failed: {err}"))?;
    let parser_output = format!("{:#?}", items);

    let functions =
        lower_items(items, &mut symbols).map_err(|err| format!("HIR lowering failed: {err}"))?;

    let normalized_hir = render_normalized_rgo(&functions);
    let hir_debug = format!("{:#?}", functions);

    let (asm, mir_model) =
        compile_source(source).map_err(|err| format!("codegen step failed: {err}"))?;
    Ok(GeneratedArtifacts {
        parser_output,
        normalized_hir,
        hir_debug,
        asm,
        mir_pretty: format!("{mir_model}"),
        mir_debug: format!("{:#?}", mir_model),
    })
}

fn write_artifacts(out_dir: &Path, stem: &str, artifacts: &GeneratedArtifacts) -> Result<(), String> {
    fs::write(out_dir.join(format!("{stem}.txt")), &artifacts.parser_output)
        .map_err(|err| format!("writing parser output failed: {err}"))?;
    fs::write(out_dir.join(format!("{stem}.hir.rgo")), &artifacts.normalized_hir)
        .map_err(|err| format!("writing normalized HIR failed: {err}"))?;
    fs::write(out_dir.join(format!("{stem}.hir.debug.txt")), &artifacts.hir_debug)
        .map_err(|err| format!("writing HIR debug failed: {err}"))?;
    fs::write(out_dir.join(format!("{stem}.asm")), &artifacts.asm)
        .map_err(|err| format!("writing assembly failed: {err}"))?;
    fs::write(out_dir.join(format!("{stem}.mir")), &artifacts.mir_pretty)
        .map_err(|err| format!("writing mir failed: {err}"))?;
    fs::write(out_dir.join(format!("{stem}.mir.debug.txt")), &artifacts.mir_debug)
        .map_err(|err| format!("writing mir debug failed: {err}"))?;
    Ok(())
}

fn cleanup_generated_artifacts(out_dir: &Path, stem: &str) -> Result<(), String> {
    const SUFFIXES: &[&str] = &[
        "txt",
        "hir.rgo",
        "hir.debug.txt",
        "asm",
        "mir",
        "mir.debug.txt",
    ];
    for suffix in SUFFIXES {
        let path = out_dir.join(format!("{stem}.{suffix}"));
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|err| format!("removing stale snapshot {} failed: {err}", path.display()))?;
        }
    }
    Ok(())
}

fn verify_expected_outputs() {
    let tests_dir = Path::new("tests");
    let bin_dir = Path::new("bin");
    fs::create_dir_all(bin_dir).expect("failed to create bin directory");

    let mut expected_paths: Vec<PathBuf> = fs::read_dir(tests_dir)
        .expect("tests directory should exist")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            if path.is_file() && name.ends_with(".expected.out") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    expected_paths.sort();

    for expected_path in expected_paths {
        let expected_name = expected_path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("filename should be valid UTF-8");
        let base = expected_name
            .strip_suffix(".expected.out")
            .expect("expected output files should end with .expected.out");
        let rgo_path = tests_dir.join(format!("{base}.rgo"));
        assert!(
            rgo_path.exists(),
            "source file is missing for expected output {}",
            expected_name
        );

        let asm_path = bin_dir.join(format!("{base}.asm"));
        compile_rgo_source(&rgo_path, &asm_path);

        let obj_path = bin_dir.join(format!("{base}.o"));
        let mut nasm_cmd = Command::new("nasm");
        nasm_cmd
            .arg("-felf64")
            .arg(&asm_path)
            .arg("-o")
            .arg(&obj_path);
        run_command(
            &mut nasm_cmd,
            &format!(
                "nasm -felf64 {} -o {}",
                asm_path.display(),
                obj_path.display()
            ),
        );

        let bin_path = bin_dir.join(base);
        let mut ld_cmd = Command::new("ld");
        ld_cmd
            .arg("-dynamic-linker")
            .arg("/lib64/ld-linux-x86-64.so.2")
            .arg("-lc")
            .arg(&obj_path)
            .arg("-o")
            .arg(&bin_path);
        run_command(
            &mut ld_cmd,
            &format!(
                "ld -dynamic-linker /lib64/ld-linux-x86-64.so.2 -lc {} -o {}",
                obj_path.display(),
                bin_path.display()
            ),
        );

        let mut run_cmd = Command::new(&bin_path);
        let actual_output =
            capture_command_output(&mut run_cmd, &format!("running {}", bin_path.display()));
        let expected_output =
            fs::read_to_string(&expected_path).expect("expected output file should be readable");

        // The console output is part of the golden snapshot and should only change when the source intentionally changes.
        assert_eq!(
            actual_output, expected_output,
            "unexpected runtime output for {}",
            expected_name
        );
    }
}

fn compile_rgo_source(rgo_path: &Path, asm_path: &Path) {
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--").arg(rgo_path).arg(asm_path);
    run_command(
        &mut cmd,
        &format!("cargo run -- {} {}", rgo_path.display(), asm_path.display()),
    );
}

fn run_command(cmd: &mut Command, description: &str) {
    let output = cmd
        .output()
        .unwrap_or_else(|err| panic!("{description} failed to start: {err}"));
    if !output.status.success() {
        panic!(
            "{} failed (status: {}):\nstdout:\n{}\nstderr:\n{}",
            description,
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn capture_command_output(cmd: &mut Command, description: &str) -> String {
    let output = cmd
        .output()
        .unwrap_or_else(|err| panic!("{description} failed to start: {err}"));
    if !output.status.success() {
        panic!(
            "{} failed (status: {}):\nstdout:\n{}\nstderr:\n{}",
            description,
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8_lossy(&output.stdout).to_string()
}
