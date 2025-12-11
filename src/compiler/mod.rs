use std::io::{BufRead, Write};

pub mod ast;
pub mod builtins;
pub mod codegen;
pub mod context;
pub mod error;
pub mod hir;
pub mod hir_ast;
pub mod hir_scope;
pub mod lexer;
pub mod mir;
pub mod parser;
pub mod runtime;
pub mod span;
pub mod symbol;
pub mod token;
pub mod type_utils;

#[cfg(test)]
mod codegen_test;
#[cfg(test)]
mod lexer_test;
#[cfg(test)]
mod parser_test;

use error::CompileError;
use hir::{Block as HirBlock, Function as HirFunction, Lowerer, Signature as HirSignature};
use lexer::Lexer;
use parser::Parser;
use span::Span;
use symbol::SymbolRegistry;

pub const ENTRY_FUNCTION_NAME: &str = "_start";

pub fn compile<R: BufRead, W: Write>(input: R, out: &mut W) -> Result<(), CompileError> {
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let mut symbols = SymbolRegistry::new();
    let mut scope = hir::Scope::new();

    // Codegen context holds global data + extern references.
    let mut ctx = codegen::CodegenContext::new();

    // Emit preamble (globals, default labels, etc.).
    codegen::write_preamble(out)?;

    let mut lowerer = Lowerer::new();
    let mut lowered_items = Vec::new();
    while let Some(item) = parser.next()? {
        lowerer.consume(item, &mut scope)?;
        while let Some(lowered) = lowerer.produce() {
            lowered_items.push(lowered);
        }
    }

    lowerer.finish()?;
    while let Some(lowered) = lowerer.produce() {
        lowered_items.push(lowered);
    }

    for item in &lowered_items {
        match item {
            hir::BlockItem::FunctionDef(function) => {
                mir::register_function_signature(function, &mut symbols)?;
            }
            hir::BlockItem::SigDef {
                name,
                kind,
                span,
                generics,
            } => {
                mir::register_sig_def(name, kind, *span, generics, &mut symbols)?;
            }
            hir::BlockItem::Import { name, span } => {
                builtins::register_import_symbols(name, *span, &mut symbols)?;
            }
            _ => {}
        }
    }

    let mut entry_items = Vec::new();
    for lowered in lowered_items {
        match lowered {
            hir::BlockItem::FunctionDef(function) => {
                let mir = mir::MirFunction::lower(&function, &mut symbols)?;
                codegen::function(mir, &symbols, &mut ctx, out)?;
            }
            hir::BlockItem::Import { name, span } => {
                entry_items.push(hir::BlockItem::Import { name, span });
            }
            hir::BlockItem::SigDef {
                name,
                kind,
                span,
                generics,
            } => {
                entry_items.push(hir::BlockItem::SigDef {
                    name,
                    kind,
                    span,
                    generics,
                });
            }
            _ => entry_items.push(lowered),
        }
    }

    if !entry_items.is_empty() {
        let entry_span = entry_items
            .last()
            .map(|item| item.span())
            .unwrap_or_else(Span::unknown);
        let entry_block = HirBlock {
            items: entry_items,
            span: entry_span,
        };
        let entry_function = HirFunction {
            name: ENTRY_FUNCTION_NAME.into(),
            sig: HirSignature {
                items: Vec::new(),
                span: Span::unknown(),
            },
            body: entry_block,
            span: entry_span,
        };
        let mir = mir::MirFunction::lower(&entry_function, &mut symbols)?;
        codegen::function(mir, &symbols, &mut ctx, out)?;
    }

    codegen::emit_builtin_definitions(&symbols, &mut ctx, out)?;
    ctx.emit_externs(out)?;
    ctx.emit_data(out)?;
    Ok(())
}
