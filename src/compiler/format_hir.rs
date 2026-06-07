use std::fmt::Write;

use crate::compiler::hir::{self, Block, BlockItem, Closure, Function};
pub fn render_normalized_rgo(items: &[BlockItem]) -> String {
    let mut out = String::new();
    for (i, item) in items.iter().enumerate() {
        match item {
            BlockItem::FunctionDef(function) => {
                write_function(function, &mut out, 0);
                if matches!(items.get(i + 1), Some(BlockItem::FunctionDef(_))) {
                    out.push('\n');
                }
            }
            _ => write_block_item(item, &mut out, 0),
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
        BlockItem::Import { label, path } => {
            write!(out, "{}: @{}", label, path).unwrap();
        }
        BlockItem::LitDef { name, literal } => match literal {
            hir::Lit::Str(s) => {
                write!(out, "{}: {}", name, format_string_literal(s)).unwrap();
            }
            hir::Lit::Int(i) => {
                write!(out, "{}: {}", name, i).unwrap();
            }
            hir::Lit::F64(f) => {
                write!(out, "{}: {}", name, f).unwrap();
            }
        },
        BlockItem::ClosureDef(Closure { name, of, args }) => {
            write!(out, "{}: {}(", name, of).unwrap();
            write_args(args, out);
            out.push(')');
        }
        BlockItem::Exec(exec) => {
            write!(out, "{}(", exec.of).unwrap();
            write_args(&exec.args, out);
            out.push(')');
        }
        BlockItem::SigDef { name, sig } => {
            let type_str = format_sig_kind(&hir::SigKind::Sig(sig.clone()));
            if sig.generics.is_empty() {
                write!(out, "{}: {}", name, type_str).unwrap();
            } else {
                let generics = sig.generics.iter().cloned().collect::<Vec<_>>().join(", ");
                write!(out, "{}: <{}>{}", name, generics, type_str).unwrap();
            }
        }
        BlockItem::FunctionDef(_) => unreachable!(),
    }
    out.push('\n');
}

fn write_args(args: &[String], out: &mut String) {
    let mut first = true;
    for arg in args {
        if !first {
            out.push_str(", ");
        }
        first = false;
        write!(out, "{}", arg).unwrap();
    }
}

fn write_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("    ");
    }
}

fn format_param_list(params: &[hir::SigItem]) -> String {
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

pub fn format_sig_kind(kind: &hir::SigKind) -> String {
    match kind {
        hir::SigKind::Byte => "byte".to_string(),
        hir::SigKind::Int => "int".to_string(),
        hir::SigKind::Str => "str".to_string(),
        hir::SigKind::F64 => "f64".to_string(),
        hir::SigKind::CompileTimeInt => "int!".to_string(),
        hir::SigKind::CompileTimeStr => "str!".to_string(),
        hir::SigKind::Sig(inner) => {
            let entries = inner
                .items
                .iter()
                .map(|item| format_sig_kind(&item.kind))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({})", entries)
        }
        hir::SigKind::Ident(ident) => ident.name.clone(),
        hir::SigKind::Variadic => "...".to_string(),
        hir::SigKind::GenericInst { name, args } => {
            let entries = args
                .iter()
                .map(format_sig_kind)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{name}<{entries}>")
        }
        hir::SigKind::Generic(name) => name.clone(),
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
