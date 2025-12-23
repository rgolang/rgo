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
    writeln!(out, "{}({}) {{", function.sig.name, params).unwrap();
    for stmt in &function.items {
        for formatted in format_mir_block_item(stmt) {
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
    format!("{}: {}", param.name, ty)
}

fn format_mir_block_item(item: &mir::MirStmt) -> Vec<String> {
    match item {
        mir::MirStmt::EnvBase(base) => {
            if base.size > 0 {
                vec![format!(
                    "{} = {} - {} words",
                    base.name, base.env_end, base.size
                )]
            } else {
                vec![format!("{} = {}", base.name, base.env_end)]
            }
        }
        mir::MirStmt::EnvField(field) => {
            let ty = format_type(&field.ty);
            vec![format!(
                "{}: {} = {} - {} words ({})",
                field.result, ty, field.env_end, field.offset_from_end, field.field_name
            )]
        }
        mir::MirStmt::StrDef { name, literal } => {
            vec![format!(
                "{} = \"{}\"",
                name,
                escape_literal(literal.value.as_str())
            )]
        }
        mir::MirStmt::IntDef { name, literal } => {
            vec![format!("{} = {}", name, literal.value)]
        }
        mir::MirStmt::Exec(exec) => {
            let args = render_args(&exec.args);
            vec![format!(
                "{}({})",
                format_mir_exec_target(&exec.target),
                args
            )]
        }
        mir::MirStmt::Closure(s) => {
            let mut lines = Vec::new();

            let args = render_args(&s.args);
            lines.push(format!(
                "{} = {}({})",
                &s.name,
                format_mir_exec_target(&s.target),
                args
            ));

            if let mir::MirExecTarget::Function(sig) = &s.target {
                for (idx, arg) in s.args.iter().enumerate() {
                    if let Some(param) = sig.params.get(idx) {
                        if !param.name.is_empty() {
                            lines.push(format!("{}.{} = {}", &s.name, param.name, arg.name));
                        }
                    }
                }
            }
            if lines.len() > 1 {
                lines.push(String::new());
            }
            lines
        }
        mir::MirStmt::ReleaseEnv(mir::ReleaseEnv { name, .. }) => {
            vec![format!("release {}", name)]
        }
        mir::MirStmt::DeepCopy(mir::DeepCopy { original, copy, .. }) => {
            vec![format!("{} = deepcopy {}", copy, original)]
        }
        mir::MirStmt::Op(instr) => {
            let outputs = instr.outputs.join(", ");
            let inputs = instr
                .inputs
                .iter()
                .map(|arg| arg.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            vec![format!("{} = @{}({});", outputs, instr.kind.name(), inputs)]
        }
        mir::MirStmt::SysCall(syscall) => {
            let args = render_args(&syscall.args);
            let kind = syscall.kind.name();
            if syscall.outputs.is_empty() {
                vec![format!("syscall {}({})", kind, args)]
            } else {
                let outputs = render_outputs(&syscall.outputs);
                vec![format!("{} = syscall {}({})", outputs, kind, args)]
            }
        }
        mir::MirStmt::Call(call) => {
            let args = call
                .args
                .iter()
                .map(|arg| arg.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            if call.result.is_empty() {
                vec![format!("@{}({})", call.name, args)]
            } else {
                vec![format!("{} = @{}({})", call.result, call.name, args)]
            }
        }
    }
}

fn format_mir_exec_target(target: &mir::MirExecTarget) -> String {
    match target {
        mir::MirExecTarget::Function(sig) => sig.name.clone(),
        mir::MirExecTarget::Closure { name } => name.clone(),
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

fn render_args(args: &[mir::MirArg]) -> String {
    args.iter()
        .map(|arg| arg.name.clone())
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_outputs(outputs: &[String]) -> String {
    outputs.join(", ")
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
