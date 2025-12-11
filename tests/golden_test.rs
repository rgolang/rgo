use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

mod test_utils;
use compiler::compiler::hir;
use compiler::compiler::{compile, lexer::Lexer, parser::Parser};
use test_utils::render_normalized_rgo;

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

fn compile_source(source: &str) -> Result<String, String> {
    let mut output = Vec::new();
    let cursor = Cursor::new(source.as_bytes());
    compile(cursor, &mut output).map_err(|err| format!("assembly generation failed: {err}"))?;
    let asm =
        String::from_utf8(output).map_err(|err| format!("assembly is not valid UTF-8: {err}"))?;
    Ok(asm) // TODO: This mir_module can be done better
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
                fs::write(&err_path, format!("{err}\n")).map_err(|write_err| {
                    format!("writing expected error snapshot failed: {write_err}")
                })?;
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
    asm: String,
}

fn generate_artifacts(source: &str) -> Result<GeneratedArtifacts, String> {
    let cursor = Cursor::new(source.as_bytes());
    let lexer = Lexer::new(cursor);
    let mut parser = Parser::new(lexer);
    let mut scope = hir::Scope::new();

    let mut block_items = Vec::new();
    let mut lowerer = hir::Lowerer::new();
    let mut hir_block_items = Vec::new();

    while let Some(item) = parser.next().map_err(|e| e.to_string())? {
        block_items.push(item.clone());
        lowerer
            .consume(item, &mut scope)
            .map_err(|e| e.to_string())?;
        while let Some(lowered) = lowerer.produce() {
            hir_block_items.push(lowered.clone());
        }
    }

    lowerer.finish().map_err(|e| e.to_string())?;
    while let Some(lowered) = lowerer.produce() {
        hir_block_items.push(lowered.clone());
    }

    let normalized_hir = render_normalized_rgo(&hir_block_items);
    let parser_output = format!("{:#?}", block_items);

    let asm = compile_source(source).map_err(|err| format!("codegen step failed: {err}"))?;
    Ok(GeneratedArtifacts {
        parser_output,
        normalized_hir,
        asm,
    })
}

fn write_artifacts(
    out_dir: &Path,
    stem: &str,
    artifacts: &GeneratedArtifacts,
) -> Result<(), String> {
    fs::write(
        out_dir.join(format!("{stem}.txt")),
        &artifacts.parser_output,
    )
    .map_err(|err| format!("writing parser output failed: {err}"))?;
    fs::write(
        out_dir.join(format!("{stem}.hir.rgo")),
        &artifacts.normalized_hir,
    )
    .map_err(|err| format!("writing normalized HIR failed: {err}"))?;
    fs::write(out_dir.join(format!("{stem}.asm")), &artifacts.asm)
        .map_err(|err| format!("writing assembly failed: {err}"))?;
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
            fs::remove_file(&path).map_err(|err| {
                format!("removing stale snapshot {} failed: {err}", path.display())
            })?;
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
