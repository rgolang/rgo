use crate::compiler::ast::TypeRef;
use crate::compiler::error::ParseError;
use crate::compiler::span::Span;
use crate::compiler::symbol::{FunctionSig, SymbolRegistry};

pub fn register_import(
    name: &str,
    span: Span,
    symbols: &mut SymbolRegistry,
) -> Result<(), ParseError> {
    symbols.record_builtin_import(name);
    match name {
        "add" => register_function(
            "add",
            span,
            vec![
                TypeRef::Int,
                TypeRef::Int,
                TypeRef::Type(vec![TypeRef::Int]),
            ],
            symbols,
        ),
        "sub" => register_function(
            "sub",
            span,
            vec![
                TypeRef::Int,
                TypeRef::Int,
                TypeRef::Type(vec![TypeRef::Int]),
            ],
            symbols,
        ),
        "mul" => register_function(
            "mul",
            span,
            vec![
                TypeRef::Int,
                TypeRef::Int,
                TypeRef::Type(vec![TypeRef::Int]),
            ],
            symbols,
        ),
        "div" => register_function(
            "div",
            span,
            vec![
                TypeRef::Int,
                TypeRef::Int,
                TypeRef::Type(vec![TypeRef::Int]),
            ],
            symbols,
        ),
        "eq" => register_function(
            "eq",
            span,
            vec![
                TypeRef::Int,
                TypeRef::Int,
                TypeRef::Type(vec![]),
                TypeRef::Type(vec![]),
            ],
            symbols,
        ),
        "fmt" => {
            register_function(
                "fmt",
                span,
                vec![
                    TypeRef::Str,
                    TypeRef::Int,
                    TypeRef::Type(vec![TypeRef::Str]),
                ],
                symbols,
            )?;
            register_function(
                "write",
                span,
                vec![TypeRef::Str, TypeRef::Type(vec![])],
                symbols,
            )
        }
        "eqi" => register_function(
            "eqi",
            span,
            vec![
                TypeRef::Int,
                TypeRef::Int,
                TypeRef::Type(vec![]),
                TypeRef::Type(vec![]),
            ],
            symbols,
        ),
        "lt" => register_function(
            "lt",
            span,
            vec![
                TypeRef::Int,
                TypeRef::Int,
                TypeRef::Type(vec![]),
                TypeRef::Type(vec![]),
            ],
            symbols,
        ),
        "gt" => register_function(
            "gt",
            span,
            vec![
                TypeRef::Int,
                TypeRef::Int,
                TypeRef::Type(vec![]),
                TypeRef::Type(vec![]),
            ],
            symbols,
        ),
        "eqs" => register_function(
            "eqs",
            span,
            vec![
                TypeRef::Str,
                TypeRef::Str,
                TypeRef::Type(vec![]),
                TypeRef::Type(vec![]),
            ],
            symbols,
        ),
        "itoa" => register_function(
            "itoa",
            span,
            vec![TypeRef::Int, TypeRef::Type(vec![TypeRef::Str])],
            symbols,
        ),
        "stdout" => symbols.declare_value("stdout".to_string(), TypeRef::Str, span),
        "write" => register_function(
            "write",
            span,
            vec![TypeRef::Str, TypeRef::Type(vec![])],
            symbols,
        ),
        "rgo_write" => register_function(
            "rgo_write",
            span,
            vec![TypeRef::Str, TypeRef::Type(vec![])],
            symbols,
        ),
        "exit" => register_function("exit", span, vec![TypeRef::Int], symbols),
        "printf" => Ok(()),
        "sprintf" => Ok(()),
        _ => Ok(()),
    }
}

fn register_function(
    name: &str,
    span: Span,
    params: Vec<TypeRef>,
    symbols: &mut SymbolRegistry,
) -> Result<(), ParseError> {
    if symbols.get_function(name).is_some() {
        return Ok(());
    }
    let is_variadic = vec![false; params.len()];
    symbols.declare_function(FunctionSig {
        name: name.to_string(),
        params,
        is_variadic,
        span,
    })
}
