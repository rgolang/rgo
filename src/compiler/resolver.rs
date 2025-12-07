use std::collections::HashSet;

use crate::compiler::error::ResolveError;
use crate::compiler::hir::{Arg, Block, BlockItem, Function};
use crate::compiler::span::Span;
use crate::compiler::symbol::SymbolRegistry;

pub fn resolve_function(func: &Function, symbols: &SymbolRegistry) -> Result<(), ResolveError> {
    let mut scope = Scope::new();
    for param in &func.params {
        scope.define(&param.name);
    }
    resolve_block(&func.body, &mut scope, symbols)?;
    Ok(())
}

fn resolve_block(
    block: &Block,
    scope: &mut Scope,
    symbols: &SymbolRegistry,
) -> Result<(), ResolveError> {
    for item in &block.items {
        match item {
            BlockItem::FunctionDef(function) => {
                scope.define(&function.name);
                resolve_function(function, symbols)?;
            }
            BlockItem::StrDef(literal) => {
                scope.define(&literal.name);
            }
            BlockItem::IntDef(literal) => {
                scope.define(&literal.name);
            }
            BlockItem::ApplyDef(apply) => {
                resolve_target(&apply.of, scope, symbols, apply.span)?;
                for arg in &apply.args {
                    resolve_arg(arg, scope, symbols)?;
                }
                scope.define(&apply.name);
            }
            BlockItem::Exec(exec) => {
                resolve_target(&exec.of, scope, symbols, exec.span)?;
                for arg in &exec.args {
                    resolve_arg(arg, scope, symbols)?;
                }
                if let Some(result) = &exec.result {
                    scope.define(result);
                }
            }
        }
    }
    Ok(())
}

fn resolve_target(
    target: &str,
    scope: &Scope,
    symbols: &SymbolRegistry,
    span: Span,
) -> Result<(), ResolveError> {
    if scope.contains(target)
        || symbols.get_function(target).is_some()
        || symbols.get_value(target).is_some()
        || symbols.builtin_imports().contains(target)
    {
        Ok(())
    } else {
        Err(ResolveError::new(
            format!("unknown variable '{}'", target),
            span,
        ))
    }
}

fn resolve_arg(arg: &Arg, scope: &Scope, symbols: &SymbolRegistry) -> Result<(), ResolveError> {
    resolve_target(&arg.name, scope, symbols, arg.span)
}

#[derive(Default)]
struct Scope {
    names: HashSet<String>,
}

impl Scope {
    fn new() -> Self {
        Self::default()
    }

    fn define(&mut self, name: &str) {
        self.names.insert(name.to_string());
    }

    fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }
}
