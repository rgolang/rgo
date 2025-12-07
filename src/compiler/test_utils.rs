use std::fmt::Write;

use super::ast::TypeRef;
use super::hir::{Apply, Arg, Block, BlockItem, Function, Param};

pub fn render_normalized_rgo(functions: &[Function]) -> String {
    let mut out = String::new();
    for (index, function) in functions.iter().enumerate() {
        write_function(function, &mut out);
        if index + 1 != functions.len() {
            out.push('\n');
        }
    }
    out
}

fn write_function(function: &Function, out: &mut String) {
    writeln!(
        out,
        "{}: {}{{",
        function.name,
        format_param_list(&function.params)
    )
    .unwrap();
    write_block(&function.body, out, 1);
    writeln!(out, "}}").unwrap();
}

fn write_block(block: &Block, out: &mut String, indent: usize) {
    for item in &block.items {
        write_block_item(item, out, indent);
    }
}

fn write_block_item(item: &BlockItem, out: &mut String, indent: usize) {
    write_indent(out, indent);
    match item {
        BlockItem::FunctionDef(function) => {
            write!(out, "{}: {}", function.name, function.name).unwrap();
        }
        BlockItem::StrDef(literal) => {
            write!(
                out,
                "{}: {}",
                literal.name,
                format_string_literal(&literal.value)
            )
            .unwrap();
        }
        BlockItem::IntDef(literal) => {
            write!(out, "{}: {}", literal.name, literal.value).unwrap();
        }
        BlockItem::ApplyDef(Apply { name, of, args, .. }) => {
            write!(out, "{}: {}(", name, of).unwrap();
            write_args(args, out);
            out.push(')');
        }
        BlockItem::Exec(exec) => {
            if let Some(result) = &exec.result {
                write!(out, "{}: ", result).unwrap();
            }
            write!(out, "{}(", exec.of).unwrap();
            write_args(&exec.args, out);
            out.push(')');
        }
    }
    out.push('\n');
}

fn write_args(args: &[Arg], out: &mut String) {
    let mut first = true;
    for arg in args {
        if !first {
            out.push_str(", ");
        }
        first = false;
        write!(out, "{}", arg.name).unwrap();
    }
}

fn write_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("    ");
    }
}

fn format_param_list(params: &[Param]) -> String {
    let entries: Vec<String> = params
        .iter()
        .map(|param| {
            let ty = format_type_ref(&param.ty);
            let name_label = if param.is_variadic() {
                format!("...{}", param.name)
            } else {
                param.name.clone()
            };
            if ty.starts_with('(') {
                format!("{}:{}", name_label, ty)
            } else {
                format!("{}: {}", name_label, ty)
            }
        })
        .collect();
    format!("({})", entries.join(", "))
}

fn format_type_ref(ty: &TypeRef) -> String {
    match ty {
        super::ast::TypeRef::Int => "int".to_string(),
        super::ast::TypeRef::Str => "str".to_string(),
        super::ast::TypeRef::CompileTimeInt => "int!".to_string(),
        super::ast::TypeRef::CompileTimeStr => "str!".to_string(),
        super::ast::TypeRef::Alias(name) => name.clone(),
        super::ast::TypeRef::AliasInstance { name, args } => format!(
            "{}<{}>",
            name,
            args.iter()
                .map(format_type_ref)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        super::ast::TypeRef::Type(inner) => format!(
            "({})",
            inner
                .iter()
                .map(format_type_ref)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        super::ast::TypeRef::Generic(name) => name.clone(),
    }
}

fn format_string_literal(value: &str) -> String {
    let mut literal = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            other => literal.push(other),
        }
    }
    literal.push('"');
    literal
}
