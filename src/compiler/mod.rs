use std::io::{BufRead, Write};

pub mod ast;
pub mod builtins;
pub mod codegen;
pub mod error;
pub mod format_hir;
pub mod format_mir;
pub mod hir;
pub mod hir_ast;
pub mod hir_context;
pub mod lexer;
pub mod mir;
pub mod mir_ast;
pub mod parser;
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

use ast::SigKind;
use error::Error;
use hir::Lowerer;
use lexer::Lexer;
use parser::Parser;
use symbol::SymbolRegistry;

pub fn compile<R: BufRead, W: Write>(input: R, out: &mut W) -> Result<(), Error> {
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let mut symbols = SymbolRegistry::new();
    let mut hir_ctx = hir::Context::new();

    // Codegen context holds global data + extern references.
    let mut ctx = codegen::CodegenContext::new();

    // Emit preamble (globals, default labels, etc.).
    codegen::write_preamble(out)?;

    let mut lowerer = Lowerer::new();
    let mut entry_items: Vec<hir::BlockItem> = Vec::new();

    while let Some(item) = parser.next()? {
        lowerer.consume(&mut hir_ctx, item)?; // consume one function/item

        // produce many functions/types etc (hoisted)
        while let Some(lowered) = lowerer.produce() {
            match lowered.clone() {
                hir::BlockItem::Import { name, span } => {
                    symbol::register_builtin_import(&name, span, &mut symbols)?;
                }
                hir::BlockItem::SigDef { name, sig, span } => {
                    symbols.install_type(name.to_string(), SigKind::Sig(sig.clone()), span)?;
                }
                hir::BlockItem::FunctionDef(function) => {
                    for mir_function in mir::lower_function(&function, &mut symbols)? {
                        codegen::function(mir_function, &mut ctx, out)?;
                    }
                }
                _ => {
                    entry_items.push(lowered);
                }
            }
        }
    }

    if !entry_items.is_empty() {
        let mir_funcs = mir::entry_function(entry_items, &mut symbols)?;
        for func in mir_funcs {
            codegen::function(func, &mut ctx, out)?;
        }
    }

    codegen::emit_builtin_definitions(symbols.builtin_imports(), &mut ctx, out)?;
    ctx.emit_externs(out)?;
    ctx.emit_data(out)?;
    Ok(())
}
