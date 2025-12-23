use std::fs;
use std::io::Cursor;
use std::path::Path;

use super::compile;

fn assert_codegen_snapshot(actual: &str, expected: &str, snapshot_name: &str, message: &str) {
    if actual != expected {
        let actual_path = Path::new("src/compiler").join(format!("{snapshot_name}.actual"));
        fs::write(&actual_path, actual)
            .unwrap_or_else(|err| panic!("failed to write {}: {err}", actual_path.display()));
    }

    assert_eq!(actual, expected, "{}", message);
}

#[test]
#[ignore]
fn codegen_test() {
    let source = include_bytes!("codegen_test.rgo");
    let cursor = Cursor::new(&source[..]);
    let mut output = Vec::new();

    compile(cursor, &mut output).expect("codegen should accept codegen_test.rgo");

    let asm = String::from_utf8(output).expect("assembly should be UTF-8");
    let expected = include_str!("codegen_test.expected.asm");

    assert_codegen_snapshot(
        &asm,
        expected,
        "codegen_test",
        "codegen should lower foo(a:int, b:int) -> int and tail-jump to foo(1, 2)",
    );
}

#[test]
#[ignore]
fn codegen_curry_test() {
    let source = include_bytes!("codegen_curry_test.rgo");
    let cursor = Cursor::new(&source[..]);
    let mut output = Vec::new();

    compile(cursor, &mut output).expect("codegen should accept codegen_curry_test.rgo");

    let asm = String::from_utf8(output).expect("assembly should be UTF-8");
    let expected = include_str!("codegen_curry_test.expected.asm");

    assert_codegen_snapshot(
        &asm,
        expected,
        "codegen_curry_test",
        "codegen should lower properly",
    );
}
