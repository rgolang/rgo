use std::fmt;

use crate::compiler::air;
use crate::compiler::ast;
use crate::compiler::span::Span;

pub fn render_air_functions(functions: &[air::AirFunction]) -> String {
    let mut out = String::new();
    for (idx, function) in functions.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        out.push_str(&format!("{function:#?}"));
        out.push('\n');
    }
    out
}

struct SigDisplay<'a>(&'a air::FunctionSig);

impl<'a> fmt::Debug for SigDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sig = self.0;
        write!(f, "{}(", sig.name)?;
        for (idx, param) in sig.params.iter().enumerate() {
            if idx > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", format_sig_item(param))?;
        }
        write!(f, "):")?;
        if let Some(builtin) = &sig.builtin {
            write!(f, " @{}", builtin.name())?;
        }
        Ok(())
    }
}

impl fmt::Debug for air::AirFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:?}", SigDisplay(&self.sig))?;
        for stmt in &self.items {
            match stmt {
                air::AirStmt::Label(_) => writeln!(f, "{:?}", stmt)?,
                _ => writeln!(f, "    {:?}", stmt)?,
            }
        }
        Ok(())
    }
}

impl fmt::Debug for air::AirStmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            air::AirStmt::Op(op) => match op {
                air::AirOp::NewClosure(closure) => {
                    let layout_values = closure
                        .target
                        .param_kinds()
                        .iter()
                        .map(format_sig_kind)
                        .collect::<Vec<_>>();
                    let layout_text = layout_values.join(", ");
                    let target_text = format_function_target(&closure.target);
                    let args_text = format_args_inline(&closure.args);
                    let invocation_args = if args_text.is_empty() {
                        target_text
                    } else {
                        format!("{}, {}", target_text, args_text)
                    };
                    write!(
                        f,
                        "{} = @newclosure<{}>({})",
                        format_binding_name(&closure.name),
                        layout_text,
                        invocation_args,
                    )
                }
                air::AirOp::Jump(jump) => write!(f, "@jump({})", jump.target),
                air::AirOp::Pin(pin) => {
                    write!(
                        f,
                        "{} = @pin({})",
                        format_binding_name(&pin.result),
                        format_operand(&pin.value)
                    )
                }
                air::AirOp::Field(field) => write!(
                    f,
                    "{} = @field({}, {})",
                    format_binding_name(&field.result),
                    format_binding_name(&field.ptr),
                    field.offset
                ),
                air::AirOp::ReleaseHeap(heap) => {
                    write!(f, "@release({})", format_binding_name(&heap.name))
                }
                air::AirOp::SetField(set) => write!(
                    f,
                    "@setfield({}, {}, {})",
                    format_binding_name(&set.env_end),
                    set.offset,
                    format_arg(&set.value)
                ),
                air::AirOp::CopyField(field) => write!(
                    f,
                    "@deepcopy({}, {}, {})",
                    format_binding_name(&field.result),
                    format_binding_name(&field.ptr),
                    field.offset
                ),
                air::AirOp::CloneClosure(clone) => {
                    write!(
                        f,
                        "{} = @cloneclosure({}, {})",
                        format_binding_name(&clone.dst),
                        format_binding_name(&clone.src),
                        format_sig_kinds_inline(&clone.remaining)
                    )
                }
                air::AirOp::JumpEqInt(eq) => {
                    let args = format_args_inline(&eq.args);
                    if args.is_empty() {
                        write!(f, "@eqi({})", eq.target)
                    } else {
                        write!(f, "@eqi({}, {})", eq.target, args)
                    }
                }
                air::AirOp::JumpEqStr(eq) => {
                    let args = format_args_inline(&eq.args);
                    if args.is_empty() {
                        write!(f, "@eqs({})", eq.target)
                    } else {
                        write!(f, "@eqs({}, {})", eq.target, args)
                    }
                }
                air::AirOp::JumpLt(jump) => write!(
                    f,
                    "@lt({}, {}, {})",
                    jump.target,
                    format_operand(&jump.left),
                    format_operand(&jump.right),
                ),
                air::AirOp::Add(op) => {
                    write!(f, "{}", format_instr_op("add", &op.inputs, &op.target))
                }
                air::AirOp::Sub(op) => {
                    write!(f, "{}", format_instr_op("sub", &op.inputs, &op.target))
                }
                air::AirOp::Mul(op) => {
                    write!(f, "{}", format_instr_op("mul", &op.inputs, &op.target))
                }
                air::AirOp::Div(op) => {
                    write!(f, "{}", format_instr_op("div", &op.inputs, &op.target))
                }
                air::AirOp::JumpGt(jump) => write!(
                    f,
                    "@gt({}, {}, {})",
                    jump.target,
                    format_operand(&jump.left),
                    format_operand(&jump.right),
                ),
                air::AirOp::Printf(call) => {
                    write!(f, "{}", format_call_op("printf", &call.args, &call.target))
                }
                air::AirOp::Sprintf(call) => {
                    write!(f, "{}", format_call_op("sprintf", &call.args, &call.target))
                }
                air::AirOp::Write(call) => {
                    write!(f, "{}", format_call_op("write", &call.args, &call.target))
                }
                air::AirOp::Puts(call) => {
                    write!(f, "{}", format_call_op("puts", &call.args, &call.target))
                }
                air::AirOp::JumpArgs(ja) => {
                    let args = format_args_inline(&ja.args);
                    let target = if let Some(builtin) = &ja.target.builtin {
                        format!("@{}", builtin.name())
                    } else {
                        ja.target.name.clone()
                    };
                    if args.is_empty() {
                        write!(f, "@jumpargs({target})")
                    } else {
                        write!(f, "@jumpargs({target}, {args})")
                    }
                }
                air::AirOp::CallPtr(call) => {
                    let target = match &call.target {
                        air::AirCallPtrTarget::Binding(name) => format_binding_name(name),
                    };
                    write!(f, "@callptr({target})")
                }
                air::AirOp::SysExit(syscall) => {
                    let args = format_args_inline(&syscall.args);
                    if args.is_empty() {
                        write!(f, "@exit()")
                    } else {
                        write!(f, "@exit({})", args)
                    }
                }
                air::AirOp::JumpClosure(jump) => {
                    let args = format_args_inline(&jump.args);
                    let target = format_binding_name(&jump.env_end);
                    if args.is_empty() {
                        write!(f, "@jumpclosure({})", target)
                    } else {
                        write!(f, "@jumpclosure({}, {})", target, args)
                    }
                }
                air::AirOp::Return(ret) => {
                    if let Some(value) = &ret.value {
                        write!(f, "@return({})", format_binding_name(value))
                    } else {
                        write!(f, "@return()")
                    }
                }
            },
            air::AirStmt::Label(label) => write!(f, "{}:", label.name),
        }
    }
}

fn format_sig_item(param: &air::SigItem) -> String {
    format_sig_item_inner(param, true)
}

fn format_sig_item_inner(param: &air::SigItem, show_names: bool) -> String {
    if show_names && !param.name.is_empty() {
        let binding = format_binding_name(&param.name);
        if matches!(param.kind, air::SigKind::Sig(_)) {
            format!("{binding}: ()")
        } else {
            format!(
                "{binding}: {}",
                format_sig_kind_inner(&param.kind, show_names)
            )
        }
    } else {
        if matches!(param.kind, air::SigKind::Sig(_)) {
            "()".to_string()
        } else {
            format_sig_kind_inner(&param.kind, show_names)
        }
    }
}

fn format_sig_kind(kind: &air::SigKind) -> String {
    format_sig_kind_inner(kind, true)
}

fn format_sig_kind_inner(kind: &air::SigKind, show_names: bool) -> String {
    match kind {
        air::SigKind::Int => "int".to_string(),
        air::SigKind::Str => "str".to_string(),
        air::SigKind::Variadic => "...".to_string(),
        air::SigKind::CompileTimeInt => "int!".to_string(),
        air::SigKind::CompileTimeStr => "str!".to_string(),
        air::SigKind::Ident(ident) => ident.name.clone(),
        air::SigKind::Sig(sig) => {
            let items = sig
                .items
                .iter()
                .map(|item| format_sig_item_inner(item, show_names))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({})", items)
        }
        air::SigKind::GenericInst { name, args } => {
            let inner = args
                .iter()
                .map(|kind| format_sig_kind_inner(kind, show_names))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{name}<{inner}>")
        }
        air::SigKind::Generic(name) => name.clone(),
    }
}

fn format_arg(arg: &air::AirArg) -> String {
    let kind_text = if matches!(arg.kind, air::SigKind::Sig(_)) {
        "()".to_string()
    } else {
        format_sig_kind(&arg.kind)
    };
    let binding = format_binding_name(&arg.name);
    let mut text = format!("{binding}: {kind_text}");
    if let Some(literal) = &arg.literal {
        let literal_text = match literal {
            ast::Lit::Str(value) => format_literal(value),
            ast::Lit::Int(value) => value.to_string(),
        };
        text.push_str(&format!(" = {}", literal_text));
    }
    text
}

fn format_args_inline(args: &[air::AirArg]) -> String {
    args.iter().map(format_arg).collect::<Vec<_>>().join(", ")
}

fn format_sig_kinds_inline(kinds: &[air::SigKind]) -> String {
    kinds
        .iter()
        .map(format_sig_kind)
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_function_target(target: &air::FunctionSig) -> String {
    if let Some(builtin) = &target.builtin {
        format!("@{}", builtin.name())
    } else {
        target.name.clone()
    }
}

fn format_instr_op(name: &str, inputs: &[air::AirArg], target: &str) -> String {
    let inputs = format_args_inline(inputs);
    let formatted_target = format_binding_name(target);
    if inputs.is_empty() {
        format!("@{}({})", name, formatted_target)
    } else {
        format!("@{}({}, {})", name, inputs, formatted_target)
    }
}

fn format_call_op(name: &str, args: &[air::AirArg], target: &str) -> String {
    let args = format_args_inline(args);
    let formatted_target = format_binding_name(target);
    if args.is_empty() {
        format!("@{}({})", name, formatted_target)
    } else {
        format!("@{}({}, {})", name, args, formatted_target)
    }
}

fn format_literal(value: &str) -> String {
    format!("\"{}\"", escape_literal(value))
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

fn format_operand(operand: &air::AirValue) -> String {
    match operand {
        air::AirValue::Binding(name) => format_binding_name(name),
        air::AirValue::Literal(value) => value.to_string(),
    }
}

fn format_binding_name(name: &str) -> String {
    if name.is_empty() {
        String::new()
    } else if name.starts_with('$') {
        name.to_string()
    } else {
        format!("${name}")
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}
