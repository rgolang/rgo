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
    let mut mir_functions: Vec<mir::MirFunction> = Vec::new();

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
                    mir_functions.extend(mir::lower_function(&function, &mut symbols)?);
                }
                _ => {
                    entry_items.push(lowered);
                }
            }
        }
    }

    if !entry_items.is_empty() {
        let mir_funcs = mir::entry_function(entry_items, &mut symbols)?;
        mir_functions.extend(mir_funcs);
    }

    mir_functions.extend(mir::builtin_functions(&symbols));
    let requirements = mir::CodegenRequirements::compute(&mir_functions);
    let mut artifacts = codegen::Artifacts::collect(&mir_functions);
    for func in mir_functions {
        codegen::function(func, &mut artifacts, out)?;
    }

    if requirements.release_helper {
        codegen::emit_release_helper(out)?;
    }
    if requirements.deep_copy_helper {
        codegen::emit_deep_copy_helper(out)?;
    }
    codegen::emit_externs(&artifacts.externs, out)?;
    codegen::emit_data(artifacts.string_literals(), out)?;
    Ok(())
}
