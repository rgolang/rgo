use std::fmt::Write;

use compiler::compiler::hir;
use compiler::compiler::hir::{Apply, Arg, Block, BlockItem, Function, SigItem};
pub fn render_normalized_rgo(items: &[BlockItem]) -> String {
    let mut out = String::new();
    for (i, item) in items.iter().enumerate() {
        match item {
            BlockItem::FunctionDef(function) => {
                write_function(function, &mut out);
                if matches!(items.get(i + 1), Some(BlockItem::FunctionDef(_))) {
                    out.push('\n');
                }
            }
            _ => write_block_item(item, &mut out, 0),
        }
    }

    out
}

fn write_function(function: &Function, out: &mut String) {
    writeln!(
        out,
        "{}: {}{{",
        function.name,
        format_param_list(&function.sig.items)
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
        BlockItem::Import { name, .. } => {
            write!(out, "@{}", name).unwrap();
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
        BlockItem::SigDef {
            name,
            kind,
            generics,
            ..
        } => {
            let type_str = format_hir_sig_kind(kind);
            if generics.is_empty() {
                write!(out, "{}: {}", name, type_str).unwrap();
            } else {
                write!(out, "{}: <{}>{}", name, generics.join(", "), type_str).unwrap();
            }
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

fn format_param_list(params: &[SigItem]) -> String {
    let entries: Vec<String> = params
        .iter()
        .map(|param| {
            let ty = format_hir_type_ref(&param.ty);
            let name_label = if param.is_variadic {
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

fn format_hir_type_ref(ty: &hir::SigType) -> String {
    format_hir_sig_kind(&ty.kind)
}

fn format_hir_sig_kind(kind: &hir::SigKind) -> String {
    match kind {
        hir::SigKind::Int => "int".to_string(),
        hir::SigKind::Str => "str".to_string(),
        hir::SigKind::CompileTimeInt => "int!".to_string(),
        hir::SigKind::CompileTimeStr => "str!".to_string(),
        hir::SigKind::Tuple(inner) => {
            let entries = inner
                .items
                .iter()
                .map(|item| format_hir_type_ref(&item.ty))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({})", entries)
        }
        hir::SigKind::Ident(ident) => {
            if ident.has_bang {
                format!("{}!", ident.name)
            } else {
                ident.name.clone()
            }
        }
        hir::SigKind::Generic(name) => name.clone(),
        hir::SigKind::GenericInst { name, args } => format!(
            "{}<{}>",
            name,
            args.iter()
                .map(format_hir_sig_kind)
                .collect::<Vec<_>>()
                .join(", ")
        ),
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
