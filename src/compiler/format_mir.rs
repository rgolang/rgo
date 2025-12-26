use std::collections::HashSet;
use std::fmt::Write;

use crate::compiler::mir;

pub fn render_mir_functions(functions: &[mir::MirFunction]) -> String {
    let mut out = String::new();
    for (idx, function) in functions.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        write_mir_function(function, &mut out);
        out.push('\n');
    }
    out
}

fn write_mir_function(function: &mir::MirFunction, out: &mut String) {
    let params = format_function_params(function);
    let mut prefixed_names = collect_local_names(function);
    for param in &function.sig.params {
        prefixed_names.insert(param.name.clone());
    }

    writeln!(out, "{}({}) {{", function.sig.name, params).unwrap();
    for stmt in &function.items {
        for formatted in format_mir_block_item(stmt, &prefixed_names) {
            if formatted.is_empty() {
                out.push('\n');
            } else {
                writeln!(out, "    {}", formatted).unwrap();
            }
        }
    }
    write!(out, "}}").unwrap();
}

fn format_function_params(function: &mir::MirFunction) -> String {
    if function.sig.params.is_empty() {
        String::new()
    } else {
        function
            .sig
            .params
            .iter()
            .map(format_function_param)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn format_function_param(param: &mir::SigItem) -> String {
    let ty = if param.name.ends_with("__env_end") {
        "ptr".to_string()
    } else {
        format_type(&param.ty)
    };
    format!("{}: {}", format_parameter_name(&param.name), ty)
}

fn format_mir_block_item(item: &mir::MirStmt, prefixed_names: &HashSet<String>) -> Vec<String> {
    match item {
        mir::MirStmt::EnvBase(base) => {
            let name = format_identifier(&base.name, prefixed_names);
            let env_end = format_identifier(&base.env_end, prefixed_names);
            if base.size > 0 {
                vec![format!("{} = {} - {} words", name, env_end, base.size)]
            } else {
                vec![format!("{} = {}", name, env_end)]
            }
        }
        mir::MirStmt::EnvField(field) => {
            let ty = format_type(&field.ty);
            vec![format!(
                "{}: {} = {} - {} words ({})",
                format_identifier(&field.result, prefixed_names),
                ty,
                format_identifier(&field.env_end, prefixed_names),
                field.offset_from_end,
                field.field_name
            )]
        }
        mir::MirStmt::StrDef { name, literal } => {
            vec![format!(
                "{} = \"{}\"",
                format_identifier(name, prefixed_names),
                escape_literal(literal.value.as_str())
            )]
        }
        mir::MirStmt::IntDef { name, literal } => {
            vec![format!(
                "{} = {}",
                format_identifier(name, prefixed_names),
                literal.value
            )]
        }
        mir::MirStmt::Exec(exec) => {
            let args = render_args(&exec.args, prefixed_names);
            vec![format!(
                "{}({})",
                format_mir_exec_target(&exec.target, prefixed_names),
                args
            )]
        }
        mir::MirStmt::Closure(s) => {
            let mut lines = Vec::new();

            let target = format_closure_target(&s.target, prefixed_names);
            let env_annotation = format_env_layout(&s.env_layout);
            lines.push(format!(
                "{} = ({}, {})",
                format_identifier(&s.name, prefixed_names),
                target,
                env_annotation
            ));

            if let mir::MirExecTarget::Function(sig) = &s.target {
                for (idx, arg) in s.args.iter().enumerate() {
                    if let Some(param) = sig.params.get(idx) {
                        if !param.name.is_empty() {
                            lines.push(format!(
                                "{}.{} = {}",
                                format_identifier(&s.name, prefixed_names),
                                format_parameter_name(&param.name),
                                format_identifier(&arg.name, prefixed_names)
                            ));
                        }
                    }
                }
            }
            if lines.len() > 1 {
                lines.push(String::new());
            }
            lines
        }
        mir::MirStmt::Release(release) => {
            vec![format!("release {}", format_identifier(&release.name, prefixed_names))]
        }
        mir::MirStmt::DeepCopy(mir::DeepCopy { original, copy, .. }) => {
            vec![format!(
                "{} = deepcopy {}",
                format_identifier(copy, prefixed_names),
                format_identifier(original, prefixed_names)
            )]
        }
        mir::MirStmt::Op(instr) => {
            let outputs = instr
                .outputs
                .iter()
                .map(|name| format_identifier(name, prefixed_names))
                .collect::<Vec<_>>()
                .join(", ");
            let inputs = instr
                .inputs
                .iter()
                .map(|arg| format_identifier(&arg.name, prefixed_names))
                .collect::<Vec<_>>()
                .join(", ");
            vec![format!("{} = @{}({});", outputs, instr.kind.name(), inputs)]
        }
        mir::MirStmt::SysCall(syscall) => {
            let args = render_args(&syscall.args, prefixed_names);
            let kind = syscall.kind.name();
            if syscall.outputs.is_empty() {
                vec![format!("syscall {}({})", kind, args)]
            } else {
                let outputs = render_outputs(&syscall.outputs, prefixed_names);
                vec![format!("{} = syscall {}({})", outputs, kind, args)]
            }
        }
        mir::MirStmt::Call(call) => {
            let args = render_args(&call.args, prefixed_names);
            if call.result.is_empty() {
                vec![format!("@{}({})", call.name, args)]
            } else {
                vec![format!(
                    "{} = @{}({})",
                    format_identifier(&call.result, prefixed_names),
                    call.name,
                    args
                )]
            }
        }
    }
}

fn format_mir_exec_target(
    target: &mir::MirExecTarget,
    prefixed_names: &HashSet<String>,
) -> String {
    match target {
        mir::MirExecTarget::Function(sig) => sig.name.clone(),
        mir::MirExecTarget::Closure { name } => format_identifier(name, prefixed_names),
    }
}

fn format_type(ty: &mir::SigKind) -> String {
    match ty {
        mir::SigKind::Int => "int".to_string(),
        mir::SigKind::Str => "str".to_string(),
        mir::SigKind::Variadic => "...".to_string(),
        mir::SigKind::CompileTimeInt => "int!".to_string(),
        mir::SigKind::CompileTimeStr => "str!".to_string(),
        mir::SigKind::Sig(params) => {
            let inner = params
                .items
                .iter()
                .map(|item| format_type(&item.ty))
                .collect::<Vec<_>>();
            format!("({})", inner.join(", "))
        }
        mir::SigKind::Ident(ident) => ident.name.clone(),
        _ => unreachable!("Unexpected type kind in MIR formatting"),
    }
}

fn format_env_layout(layout: &[mir::SigKind]) -> String {
    let contents = layout
        .iter()
        .map(format_type)
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{{}}}", contents)
}

fn format_closure_target(target: &mir::MirExecTarget, prefixed_names: &HashSet<String>) -> String {
    match target {
        mir::MirExecTarget::Function(sig) => {
            format_identifier(&mir::closure_unwrapper_label(&sig.name), prefixed_names)
        }
        mir::MirExecTarget::Closure { name } => format_identifier(name, prefixed_names),
    }
}

fn render_args(args: &[mir::MirArg], prefixed_names: &HashSet<String>) -> String {
    args.iter()
        .map(|arg| format_identifier(&arg.name, prefixed_names))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_outputs(outputs: &[String], prefixed_names: &HashSet<String>) -> String {
    outputs
        .iter()
        .map(|name| format_identifier(name, prefixed_names))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_parameter_name(name: &str) -> String {
    format!("${}", name)
}

fn format_identifier(name: &str, prefixed_names: &HashSet<String>) -> String {
    if prefixed_names.contains(name) {
        format_parameter_name(name)
    } else {
        name.to_string()
    }
}

fn collect_local_names(function: &mir::MirFunction) -> HashSet<String> {
    let mut locals = HashSet::new();
    for stmt in &function.items {
        add_local_names_from_stmt(stmt, &mut locals);
    }
    locals
}

fn add_local_names_from_stmt(stmt: &mir::MirStmt, locals: &mut HashSet<String>) {
    match stmt {
        mir::MirStmt::EnvBase(base) => {
            locals.insert(base.name.clone());
            locals.insert(base.env_end.clone());
        }
        mir::MirStmt::EnvField(field) => {
            locals.insert(field.result.clone());
            locals.insert(field.env_end.clone());
        }
        mir::MirStmt::StrDef { name, .. } => {
            locals.insert(name.clone());
        }
        mir::MirStmt::IntDef { name, .. } => {
            locals.insert(name.clone());
        }
        mir::MirStmt::Closure(s) => {
            locals.insert(s.name.clone());
        }
        mir::MirStmt::Release(release) => {
            locals.insert(release.name.clone());
        }
        mir::MirStmt::DeepCopy(deepcopy) => {
            locals.insert(deepcopy.original.clone());
            locals.insert(deepcopy.copy.clone());
        }
        mir::MirStmt::Op(instr) => {
            locals.extend(instr.outputs.iter().cloned());
        }
        mir::MirStmt::SysCall(syscall) => {
            locals.extend(syscall.outputs.iter().cloned());
        }
        mir::MirStmt::Call(call) => {
            if !call.result.is_empty() {
                locals.insert(call.result.clone());
            }
        }
        mir::MirStmt::Exec(_) => {}
    }
}

fn escape_literal(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '\\' => "\\\\".to_string(),
            '\"' => "\\\"".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join("")
}
