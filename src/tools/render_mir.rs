use std::env;
use std::error::Error as StdError;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

use compiler::compiler::span::Span;
use compiler::compiler::{ast, format_air, hir, lexer::Lexer, parser::Parser};
use compiler::debug_tools::test_helpers::generate_air_functions;

fn main() -> Result<(), Box<dyn StdError>> {
    let mut args = env::args().skip(1);
    let input = args.next();
    let target = args.next();
    let output = args.next();
    if args.next().is_some() {
        return Err("expected exactly three arguments: <input> <target> <output>".into());
    }

    let (input_path, target, output_path) = match (input, target, output) {
        (Some(input), Some(target), Some(output)) => (input, target, output),
        _ => return Err("render_mir requires <input> <target> <output>".into()),
    };

    let file = File::open(&input_path)?;
    let reader = BufReader::new(file);
    let rendered = render_mir(reader, &target)?;

    let output = File::create(output_path)?;
    let mut writer = BufWriter::new(output);
    writer.write_all(rendered.as_bytes())?;
    Ok(())
}

fn render_mir<R: std::io::BufRead>(reader: R, target: &str) -> Result<String, Box<dyn StdError>> {
    let lexer = Lexer::new(reader);
    let mut parser = Parser::new(lexer);
    let mut ctx = hir::Context::new();
    let mut lowerer = hir::Lowerer::new();
    let mut hir_block_items = Vec::new();

    while let Some(item) = parser.next_block_item()? {
        reject_root_execution(&item)?;
        lowerer.consume(&mut ctx, item)?;
        while let Some(lowered) = lowerer.produce() {
            hir_block_items.push(lowered);
        }
    }

    lowerer.consume(&mut ctx, target_exec(target))?;
    while let Some(lowered) = lowerer.produce() {
        hir_block_items.push(lowered);
    }

    let air_functions = generate_air_functions(&hir_block_items)
        .map_err(|err| Box::new(err) as Box<dyn StdError>)?;
    Ok(format_air::render_air_functions(&air_functions))
}

fn target_exec(target: &str) -> ast::BlockItem {
    ast::BlockItem::Ident(ast::Ident {
        name: target.to_string(),
        args: Vec::new(),
        span: Span::unknown(),
    })
}

fn reject_root_execution(item: &ast::BlockItem) -> Result<(), Box<dyn StdError>> {
    match item {
        ast::BlockItem::Ident(_)
        | ast::BlockItem::Lambda(_)
        | ast::BlockItem::ScopeCapture { .. } => {
            Err("root-level invocation is not supported; choose a target function".into())
        }
        _ => Ok(()),
    }
}
