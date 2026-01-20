use std::fmt::Write;

use crate::compiler::ast::{self, Arg};
use crate::compiler::hir::{Block, BlockItem, Closure, Function};
pub fn render_normalized_rgo(items: &[BlockItem]) -> String {
    let mut out = String::new();
    for (i, item) in items.iter().enumerate() {
        match item {
            BlockItem::FunctionDef(function) => {
                write_function(&function, &mut out, 0);
                if matches!(items.get(i + 1), Some(BlockItem::FunctionDef(_))) {
                    out.push('\n');
                }
            }
            _ => write_block_item(&item, &mut out, 0),
        }
    }

    out
}

fn write_function(function: &Function, out: &mut String, indent: usize) {
    write_indent(out, indent);
    writeln!(
        out,
        "{}: {}{{",
        function.name,
        format_param_list(&function.sig.items)
    )
    .unwrap();
    write_block(&function.body, out, indent + 1);
    write_indent(out, indent);
    writeln!(out, "}}").unwrap();
}

fn write_block(block: &Block, out: &mut String, indent: usize) {
    for item in &block.items {
        write_block_item(item, out, indent);
    }
}

fn write_block_item(item: &BlockItem, out: &mut String, indent: usize) {
    if let BlockItem::FunctionDef(function) = item {
        write_function(function, out, indent);
        return;
    }

    write_indent(out, indent);
    match item {
        BlockItem::Import { label, path, .. } => {
            write!(out, "{}: @{}", label, path).unwrap();
        }
        BlockItem::LitDef { name, literal } => match &literal.value {
            ast::Lit::Str(s) => {
                write!(out, "{}: {}", name, format_string_literal(&s)).unwrap();
            }
            ast::Lit::Int(i) => {
                write!(out, "{}: {}", name, i).unwrap();
            }
        },
        BlockItem::ClosureDef(Closure { name, of, args, .. }) => {
            write!(out, "{}: {}(", name, of).unwrap();
            write_args(args, out);
            out.push(')');
        }
        BlockItem::Exec(exec) => {
            write!(out, "{}(", exec.of).unwrap();
            write_args(&exec.args, out);
            out.push(')');
        }
        BlockItem::SigDef { name, sig, .. } => {
            let type_str = format_sig_kind(&ast::SigKind::Sig(sig.clone()));
            if sig.generics.is_empty() {
                write!(out, "{}: {}", name, type_str).unwrap();
            } else {
                write!(out, "{}: <{}>{}", name, sig.generics.join(", "), type_str).unwrap();
            }
        }
        BlockItem::FunctionDef(_) => unreachable!(),
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

fn format_param_list(params: &[ast::SigItem]) -> String {
    let entries: Vec<String> = params
        .iter()
        .map(|param| {
            let ty = format_sig_kind(&param.kind);
            let ty = if param.has_bang && !ty.ends_with('!') {
                format!("{ty}!")
            } else {
                ty
            };
            let name_label = param.name.clone();
            if ty.starts_with('(') {
                format!("{}:{}", name_label, ty)
            } else {
                format!("{}: {}", name_label, ty)
            }
        })
        .collect();
    format!("({})", entries.join(", "))
}

pub fn format_sig_kind(kind: &ast::SigKind) -> String {
    match kind {
        ast::SigKind::Int => "int".to_string(),
        ast::SigKind::Str => "str".to_string(),
        ast::SigKind::CompileTimeInt => "int!".to_string(),
        ast::SigKind::CompileTimeStr => "str!".to_string(),
        ast::SigKind::Sig(inner) => {
            let entries = inner
                .items
                .iter()
                .map(|item| format_sig_kind(&item.kind))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({})", entries)
        }
        ast::SigKind::Ident(ident) => ident.name.clone(),
        ast::SigKind::Variadic => "...".to_string(),
        ast::SigKind::GenericInst { name, args } => {
            let entries = args
                .iter()
                .map(|arg| format_sig_kind(arg))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{name}<{entries}>")
        }
        ast::SigKind::Generic(name) => name.clone(),
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
