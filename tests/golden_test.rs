use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

use compiler::compiler::error::{self, Code, Error};
use compiler::compiler::hir;
use compiler::compiler::span::Span;
use compiler::compiler::{
    compile, format_hir::render_normalized_rgo, format_mir::render_mir_functions, lexer::Lexer,
    parser::Parser,
};
use compiler::debug_tools::test_helpers::generate_mir_functions;

const GENERATED_DIR: &str = "tests/generated";

#[test]
fn golden_test() {
    generate_golden_snapshots();
    let bin_dir = Path::new("bin");
    fs::create_dir_all(bin_dir).expect("failed to create bin directory");
    verify_expected_runtime_outputs(Path::new("tests/golden"), bin_dir);
}

#[test]
fn failing_test() {
    generate_failure_snapshots();
    let bin_dir = Path::new("bin");
    fs::create_dir_all(bin_dir).expect("failed to create bin directory");
    verify_expected_compile_errors(Path::new("tests/failing"), bin_dir);
}

fn generate_golden_snapshots() {
    let golden_dir = Path::new("tests/golden");
    let out_dir = Path::new(GENERATED_DIR);
    fs::create_dir_all(out_dir).expect("failed to create generated output directory");

    process_snapshot_directory(golden_dir, out_dir, SnapshotKind::Success);
}

fn generate_failure_snapshots() {
    let failing_dir = Path::new("tests/failing");
    let out_dir = Path::new(GENERATED_DIR);
    fs::create_dir_all(out_dir).expect("failed to create generated output directory");

    process_snapshot_directory(failing_dir, out_dir, SnapshotKind::Failure);
}

fn process_snapshot_directory(src_dir: &Path, out_dir: &Path, kind: SnapshotKind) {
    if !src_dir.exists() {
        return;
    }

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
        if let Err(err) = build_reference_for_path(&path, out_dir, kind) {
            panic!("{}: {err}", path.display());
        }
    }
}

fn compile_source(source: &str) -> Result<String, Error> {
    let mut output = Vec::new();
    let cursor = Cursor::new(source.as_bytes());
    compile(cursor, &mut output)?;
    let asm = String::from_utf8(output).map_err(|err| {
        error::new(
            Code::Codegen,
            format!("assembly is not valid UTF-8: {err}"),
            Span::unknown(),
        )
    })?;
    Ok(asm) // TODO: This mir_module can be done better
}

fn build_reference_for_path(
    path: &Path,
    out_dir: &Path,
    kind: SnapshotKind,
) -> Result<(), Error> {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| {
            error::new(
                Code::Internal,
                "filename should be valid UTF-8",
                Span::unknown(),
            )
        })?;
    let source = fs::read_to_string(path)?;
    match kind {
        SnapshotKind::Success => {
            let artifacts = generate_artifacts(&source)?;
            let actual_err_path = out_dir.join(format!("{stem}.actual.err"));
            if actual_err_path.exists() {
                fs::remove_file(&actual_err_path)?;
            }
            write_artifacts(out_dir, stem, &artifacts)?;
            Ok(())
        }
        SnapshotKind::Failure => match generate_artifacts(&source) {
            Ok(_) => Err(error::new(
                Code::Internal,
                "expected compilation failure but succeeded",
                Span::unknown(),
            )),
            Err(err) => {
                let actual_err_path = out_dir.join(format!("{stem}.actual.err"));
                fs::write(actual_err_path, format!("{err}\n"))?;
                Ok(())
            }
        },
    }
}

struct GeneratedArtifacts {
    parser_output: String,
    normalized_hir: String,
    mir: String,
    asm: String,
}

#[derive(Copy, Clone)]
enum SnapshotKind {
    Success,
    Failure,
}

fn generate_artifacts(source: &str) -> Result<GeneratedArtifacts, Error> {
    let cursor = Cursor::new(source.as_bytes());
    let lexer = Lexer::new(cursor);
    let mut parser = Parser::new(lexer);
    let mut ctx = hir::Context::new();

    let mut block_items = Vec::new();
    let mut lowerer = hir::Lowerer::new();
    let mut hir_block_items = Vec::new();

    while let Some(item) = parser.next()? {
        block_items.push(item.clone());
        lowerer.consume(&mut ctx, item)?;
        while let Some(lowered) = lowerer.produce() {
            hir_block_items.push(lowered.clone());
        }
    }

    let normalized_hir = render_normalized_rgo(&hir_block_items);
    let parser_output = format!("{:#?}", block_items);

    let mir_functions = generate_mir_functions(&hir_block_items)?;
    let mir = render_mir_functions(&mir_functions);

    let asm = compile_source(source)?;
    Ok(GeneratedArtifacts {
        parser_output,
        normalized_hir,
        mir,
        asm,
    })
}

fn write_artifacts(
    out_dir: &Path,
    stem: &str,
    artifacts: &GeneratedArtifacts,
) -> Result<(), Error> {
    fs::write(
        out_dir.join(format!("{stem}.txt")),
        &artifacts.parser_output,
    )?;
    fs::write(
        out_dir.join(format!("{stem}.hir.rgo")),
        &artifacts.normalized_hir,
    )?;
    fs::write(out_dir.join(format!("{stem}.mir")), &artifacts.mir)?;
    fs::write(out_dir.join(format!("{stem}.asm")), &artifacts.asm)?;
    Ok(())
}

fn verify_expected_runtime_outputs(tests_dir: &Path, bin_dir: &Path) {
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
        if !rgo_path.exists() {
            continue;
        }

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

fn verify_expected_compile_errors(tests_dir: &Path, bin_dir: &Path) {
    if !tests_dir.exists() {
        return;
    }

    let mut expected_paths: Vec<PathBuf> = fs::read_dir(tests_dir)
        .expect("tests directory should exist")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            if path.is_file() && name.ends_with(".expected.err") {
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
            .strip_suffix(".expected.err")
            .expect("expected error files should end with .expected.err");
        let rgo_path = tests_dir.join(format!("{base}.rgo"));
        if !rgo_path.exists() {
            continue;
        }

        let expected_error = fs::read_to_string(&expected_path)
            .expect("expected error file should be readable")
            .trim_end()
            .to_string();
        let asm_path = bin_dir.join(format!("{base}.err.asm"));
        let mut cmd = Command::new("cargo");
        cmd.arg("run").arg("--").arg(&rgo_path).arg(&asm_path);
        let actual_error =
            capture_compile_failure_output(&mut cmd, &format!("cargo run -- {}", rgo_path.display()));

        assert!(
            actual_error.contains(&expected_error),
            "expected error substring \"{}\" not found for {}: actual error:\n{}",
            expected_error,
            expected_name,
            actual_error
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

fn capture_compile_failure_output(cmd: &mut Command, description: &str) -> String {
    let output = cmd
        .output()
        .unwrap_or_else(|err| panic!("{description} failed to start: {err}"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        panic!(
            "{} unexpectedly succeeded:\nstdout:\n{}\nstderr:\n{}",
            description, stdout, stderr
        );
    }

    format!("{stdout}{stderr}")
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
