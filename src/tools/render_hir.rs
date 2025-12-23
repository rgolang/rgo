use std::env;
use std::error::Error as StdError;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

use compiler::compiler::hir;
use compiler::compiler::{lexer::Lexer, parser::Parser};
use compiler::debug_tools::format_hir::render_normalized_rgo;

fn main() -> Result<(), Box<dyn StdError>> {
    let mut args = env::args().skip(1);
    let input = args.next();
    let output = args.next();
    if args.next().is_some() {
        return Err("expected exactly two arguments: <input> <output>".into());
    }

    let (input_path, output_path) = match (input, output) {
        (Some(input), Some(output)) => (input, output),
        _ => return Err("render_hir requires an input path and an output path".into()),
    };

    let file = File::open(&input_path)?;
    let reader = BufReader::new(file);
    let rendered = render_normalized_hir(reader)?;

    let output = File::create(output_path)?;
    let mut writer = BufWriter::new(output);
    writer.write_all(rendered.as_bytes())?;
    Ok(())
}

fn render_normalized_hir<R: std::io::BufRead>(reader: R) -> Result<String, Box<dyn StdError>> {
    let lexer = Lexer::new(reader);
    let mut parser = Parser::new(lexer);
    let mut ctx = hir::Context::new();
    let mut lowerer = hir::Lowerer::new();
    let mut hir_block_items = Vec::new();

    while let Some(item) = parser.next()? {
        lowerer.consume(&mut ctx, item)?;
        while let Some(lowered) = lowerer.produce() {
            hir_block_items.push(lowered);
        }
    }

    Ok(render_normalized_rgo(&hir_block_items))
}
