use std::collections::HashMap;
use std::io::{BufRead, Write};

pub mod air;
pub mod air_ast;
pub mod ast;
pub mod builtins;
pub mod codegen;
pub mod error;
pub mod format_air;
pub mod format_hir;
pub mod hir;
pub mod hir_ast;
pub mod hir_context;
pub mod lexer;
pub mod parser;
pub mod runtime;
pub mod signature;
pub mod span;
pub mod symbol;
pub mod token;

#[cfg(test)]
mod codegen_test;
#[cfg(test)]
mod lexer_test;
#[cfg(test)]
mod parser_test;

use error::Error;
use error::{Code, Error as CompilerError};
use hir::Lowerer;
use lexer::Lexer;
use parser::Parser;
use span::Span;
use symbol::SymbolRegistry;

pub fn compile<R: BufRead, W: Write>(input: R, target: &str, out: &mut W) -> Result<(), Error> {
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let mut symbols = SymbolRegistry::new();
    let mut hir_ctx = hir::Context::new();
    let mut air_functions: Vec<air::AirFunction> = Vec::new();
    let mut hir_functions: HashMap<String, hir::Function> = HashMap::new();

    // Emit preamble (globals, default labels, etc.).
    codegen::write_preamble(out)?;

    let mut lowerer = Lowerer::new();
    let mut entry_items: Vec<hir::BlockItem> = Vec::new();

    while let Some(item) = parser.next()? {
        reject_root_execution(&item)?;
        lowerer.consume(&mut hir_ctx, item)?; // consume one function/item

        // produce many functions/types etc (hoisted)
        while let Some(lowered) = lowerer.produce() {
            match lowered {
                hir::BlockItem::Import { label, path } => {
                    symbol::register_builtin_import(&label, &path, &mut symbols)?;
                }
                hir::BlockItem::SigDef { name, sig } => {
                    symbols.install_type(name.to_string(), air::SigKind::Sig(sig.clone()))?;
                }
                hir::BlockItem::FunctionDef(function) => {
                    let sig = air::function_sig_from_hir(&function);
                    symbols.declare_function(sig)?;
                    hir_functions.insert(function.name.clone(), function);
                }
                other => entry_items.push(other),
            }
        }
    }

    let target_exec = ast::BlockItem::Ident(ast::Ident {
        name: target.to_string(),
        args: Vec::new(),
        span: Span::unknown(),
    });
    lowerer.consume(&mut hir_ctx, target_exec)?;
    while let Some(lowered) = lowerer.produce() {
        match lowered {
            hir::BlockItem::Import { .. }
            | hir::BlockItem::SigDef { .. }
            | hir::BlockItem::FunctionDef(_) => {
                return Err(CompilerError::new(
                    Code::Internal,
                    "entry target lowering produced a declaration",
                    Span::unknown(),
                ));
            }
            other => entry_items.push(other),
        }
    }

    let mut function_lowerer = air::FunctionLowerer::new(hir_functions);
    let entry_funcs = air::entry_function(entry_items, &mut symbols, &mut function_lowerer)?;
    let mut generated = function_lowerer.take_generated_functions();
    generated.extend(entry_funcs);
    air_functions.extend(generated);

    let mut artifacts = codegen::Artifacts::collect(&air_functions);
    for func in air_functions {
        codegen::function(func, &mut artifacts, out)?;
    }
    codegen::emit_externs(&artifacts.externs, out)?;
    codegen::emit_data(artifacts.string_literals(), out)?;
    Ok(())
}

fn reject_root_execution(item: &ast::BlockItem) -> Result<(), Error> {
    match item {
        ast::BlockItem::Ident(_)
        | ast::BlockItem::Lambda(_)
        | ast::BlockItem::ScopeCapture { .. } => Err(CompilerError::new(
            Code::Parse,
            "root-level invocation is not supported; choose a target function",
            item.span(),
        )),
        _ => Ok(()),
    }
}
