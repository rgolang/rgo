use std::io::{BufRead, Write};

pub mod ast;
pub mod builtins;
pub mod codegen;
pub mod error;
pub mod hir;
pub mod lexer;
pub mod mir;
pub mod parser;
pub mod resolver;
pub mod runtime;
pub mod span;
pub mod symbol;
pub mod token;
pub mod type_utils;

#[cfg(test)]
mod codegen_test;
#[cfg(test)]
mod hir_test;
#[cfg(test)]
mod lexer_test;
#[cfg(test)]
mod parser_test;
pub mod test_utils;

use crate::compiler::hir::{normalize_type_alias, ConstantValue, Env, EnvEntry, Function};
use crate::compiler::mir::MirModule;
use ast::{Item, TypeRef};
use error::{CompileError, ParseError};
use lexer::Lexer;
use parser::Parser;
use span::Span;
use symbol::SymbolRegistry;

pub struct CompileMetadata {
    pub mir_module: MirModule,
}

pub fn compile<R: BufRead, W: Write>(
    input: R,
    out: &mut W,
) -> Result<CompileMetadata, CompileError> {
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    let mut symbols = SymbolRegistry::new();
    let mut env = Env::new();

    // Codegen context holds global data + extern references.
    let mut ctx = codegen::CodegenContext::new();

    // Emit preamble (globals, default labels, etc.).
    codegen::write_preamble(out)?;

    // TODO: Collect block lines that are not functions, then emit entrypoint at the end.
    // ---------- STREAM ITEMS ----------
    let mut entry_defs = Vec::new();
    let mut entry_items = Vec::new();
    while let Some(item) = parser.next(&mut symbols)? {
        match item {
            function @ Item::FunctionDef { .. } => {
                let (main_fn, nested_fns) = hir::lower_function(function, &mut symbols, &env)?;

                // Emit nested functions *first*
                for nf in nested_fns {
                    emit_function(nf, &symbols, &mut ctx, out)?;
                }

                // Then emit the main function
                emit_function(main_fn, &symbols, &mut ctx, out)?;
            }
            Item::StrDef {
                name,
                literal,
                span,
            } => {
                let entry_name = name.clone();
                if !entry_items.is_empty() {
                    return Err(ParseError::new(
                        "literal/alias definitions must appear before the top-level entry",
                        span,
                    )
                    .into());
                }
                let literal_value = literal.value.clone();
                symbols.declare_value(name.clone(), TypeRef::Str, span)?;
                ctx.register_global_str(&name, &literal_value);
                entry_defs.push(Item::StrDef {
                    name,
                    literal,
                    span,
                });
                env.insert(
                    entry_name.clone(),
                    EnvEntry {
                        ty: TypeRef::Str,
                        span,
                        constant: Some(ConstantValue::Str(literal_value.clone())),
                    },
                );
            }
            Item::IntDef {
                name,
                literal,
                span,
            } => {
                if !entry_items.is_empty() {
                    return Err(ParseError::new(
                        "literal/alias definitions must appear before the top-level entry",
                        span,
                    )
                    .into());
                }
                let entry_name = name.clone();
                symbols.declare_value(name.clone(), TypeRef::Int, span)?;
                let literal_value = literal.value;
                ctx.register_global_int(&name, literal_value);
                entry_defs.push(Item::IntDef {
                    name,
                    literal,
                    span,
                });
                env.insert(
                    entry_name.clone(),
                    EnvEntry {
                        ty: TypeRef::Int,
                        span,
                        constant: Some(ConstantValue::Int(literal_value)),
                    },
                );
            }
            definition @ Item::IdentDef { .. } => {
                if !entry_items.is_empty() {
                    return Err(ParseError::new(
                        "literal/alias definitions must appear before the top-level entry",
                        item_span(&definition),
                    )
                    .into());
                }
                entry_defs.push(definition);
            }
            Item::Ident(term) => {
                entry_items.push(Item::Ident(term));
            }
            Item::Lambda(term) => {
                entry_items.push(Item::Lambda(term));
            }
            Item::ScopeCapture { params, term, span } => {
                entry_items.push(Item::ScopeCapture { params, term, span });
            }
            Item::TypeDef { name, .. } => {
                normalize_type_alias(&name, &mut symbols)?;
            }
            Item::Import { .. } => {
                // Nothing to emit; symbols already updated.
            }
        }
    }

    // ---------- FINISH GLOBALS ----------
    emit_entry_point(entry_defs, entry_items, &mut symbols, &mut ctx, out, &env)?;
    codegen::emit_builtin_definitions(&symbols, &mut ctx, out)?;
    ctx.emit_externs(out)?;
    ctx.emit_data(out)?;

    let metadata = CompileMetadata {
        mir_module: ctx.take_mir_module(),
    };
    Ok(metadata)
}

fn emit_function<W: Write>(
    func: Function,
    symbols: &SymbolRegistry,
    ctx: &mut codegen::CodegenContext,
    out: &mut W,
) -> Result<(), CompileError> {
    resolver::resolve_function(&func, symbols)?;
    // Convert HIR to MIR before passing to codegen
    let mir = mir::MirFunction::lower(&func, symbols)?;
    codegen::function(mir, symbols, ctx, out)?;
    Ok(())
}

//TODO: Duplicate.
fn emit_entry_point<W: Write>(
    prelude: Vec<Item>,
    entry_items: Vec<Item>,
    symbols: &mut SymbolRegistry,
    ctx: &mut codegen::CodegenContext,
    out: &mut W,
    env: &Env,
) -> Result<(), CompileError> {
    if entry_items.is_empty() {
        return Ok(());
    }
    let span = item_span(entry_items.last().unwrap());
    // Convert top-level expressions into synthetic functions
    if let Some(funcs) = hir::lower_entry(prelude, entry_items, span, symbols)? {
        for f in funcs {
            let (main_fn, nested_fns) = hir::lower_function(f, symbols, env)?;

            for nf in nested_fns {
                emit_function(nf, &symbols, ctx, out)?;
            }

            emit_function(main_fn, &symbols, ctx, out)?;
        }
    }

    Ok(())
}

fn item_span(item: &Item) -> Span {
    match item {
        Item::Import { span, .. }
        | Item::TypeDef { span, .. }
        | Item::FunctionDef { span, .. }
        | Item::StrDef { span, .. }
        | Item::IntDef { span, .. }
        | Item::IdentDef { span, .. } => *span,
        Item::ScopeCapture { span, .. } => *span,
        Item::Ident(ident) => ident.span,
        Item::Lambda(lambda) => lambda.span,
    }
}
