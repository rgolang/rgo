use crate::compiler::ast::{self, SigItem, SigKind, Signature};
use crate::compiler::span::Span;

pub enum BuiltinSpec {
    Function(ast::Signature),
    Type(ast::SigKind),
}
pub fn get_spec(name: &str, span: Span) -> Option<BuiltinSpec> {
    return match name {
        "int" => Some(BuiltinSpec::Type(ast::SigKind::Int)),
        "str" => Some(BuiltinSpec::Type(ast::SigKind::Str)),

        "add" | "sub" | "mul" | "div" => Some(BuiltinSpec::Function(math_binary_sig(span))),

        "eq" | "eqi" | "lt" | "gt" => {
            Some(BuiltinSpec::Function(comparison_sig(SigKind::Int, span)))
        }

        "eqs" => Some(BuiltinSpec::Function(comparison_sig(SigKind::Str, span))),

        "itoa" => Some(BuiltinSpec::Function(sig_from_items(
            vec![
                sig_item("value", SigKind::Int, span),
                sig_item("ok", SigKind::tuple([SigKind::Str]), span),
            ],
            span,
        ))),

        "write" | "puts" => Some(BuiltinSpec::Function(sig_from_items(
            vec![
                sig_item("value", SigKind::Str, span),
                sig_item("ok", SigKind::tuple([]), span),
            ],
            span,
        ))),

        "exit" => Some(BuiltinSpec::Function(sig_from_items(
            vec![sig_item("code", SigKind::Int, span)],
            span,
        ))),

        "printf" => Some(BuiltinSpec::Function(sig_from_items(
            vec![
                sig_item("format", SigKind::CompileTimeStr, span),
                sig_item("args", SigKind::Variadic, span),
                sig_item("ok", SigKind::tuple([]), span),
            ],
            span,
        ))),
        "sprintf" => Some(BuiltinSpec::Function(sig_from_items(
            vec![
                sig_item("format", SigKind::CompileTimeStr, span),
                sig_item("args", SigKind::Variadic, span),
                sig_item("ok", SigKind::tuple([SigKind::Str]), span),
            ],
            span,
        ))),

        _ => None,
    };
}

fn sig_item(name: &str, ty: SigKind, span: Span) -> SigItem {
    SigItem {
        name: name.to_string(),
        kind: ty,
        has_bang: false,
        span,
    }
}

fn tuple_sig(items: Vec<SigItem>, span: Span) -> SigKind {
    SigKind::Sig(Signature {
        items,
        span,
        generics: Vec::new(),
    })
}

fn sig_from_items(items: Vec<SigItem>, span: Span) -> Signature {
    Signature {
        items,
        span,
        generics: Vec::new(),
    }
}

fn comparison_sig(arg_kind: SigKind, span: Span) -> Signature {
    sig_from_items(
        vec![
            sig_item("left", arg_kind.clone(), span),
            sig_item("right", arg_kind, span),
            sig_item("ok", SigKind::tuple([]), span),
            sig_item("err", SigKind::tuple([]), span),
        ],
        span,
    )
}

fn math_binary_sig(span: Span) -> Signature {
    sig_from_items(
        vec![
            sig_item("x", SigKind::Int, span),
            sig_item("y", SigKind::Int, span),
            sig_item(
                "ok",
                tuple_sig(vec![sig_item("res", SigKind::Int, span)], span),
                span,
            ),
        ],
        span,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MirInstKind {
    Add,
    Sub,
    Mul,
    Div,
    EqInt,
    EqStr,
    Lt,
    Gt,
}

impl MirInstKind {
    pub fn name(&self) -> &'static str {
        match self {
            MirInstKind::Add => "add",
            MirInstKind::Sub => "sub",
            MirInstKind::Mul => "mul",
            MirInstKind::Div => "div",
            MirInstKind::EqInt => "eqi",
            MirInstKind::EqStr => "eqs",
            MirInstKind::Lt => "lt",
            MirInstKind::Gt => "gt",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MirCallKind {
    Printf,
    Sprintf,
    Write,
    Puts,
}

impl MirCallKind {
    pub fn name(&self) -> &'static str {
        match self {
            MirCallKind::Printf => "printf",
            MirCallKind::Sprintf => "sprintf",
            MirCallKind::Write => "write",
            MirCallKind::Puts => "puts",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MirSysCallKind {
    Exit,
}

impl MirSysCallKind {
    pub fn name(&self) -> &'static str {
        match self {
            MirSysCallKind::Exit => "exit",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MirBuiltin {
    Instruction(MirInstKind),
    Call(MirCallKind),
    SysCall(MirSysCallKind),
}

impl MirBuiltin {
    pub fn name(&self) -> &'static str {
        match self {
            MirBuiltin::Instruction(instruction) => instruction.name(),
            MirBuiltin::SysCall(syscall) => syscall.name(),
            MirBuiltin::Call(call) => call.name(),
        }
    }
}

pub fn get_builtin_kind(name: &str) -> Option<MirBuiltin> {
    match name {
        "add" => Some(MirBuiltin::Instruction(MirInstKind::Add)),
        "sub" => Some(MirBuiltin::Instruction(MirInstKind::Sub)),
        "mul" => Some(MirBuiltin::Instruction(MirInstKind::Mul)),
        "div" => Some(MirBuiltin::Instruction(MirInstKind::Div)),
        "eq" | "eqi" => Some(MirBuiltin::Instruction(MirInstKind::EqInt)),
        "eqs" => Some(MirBuiltin::Instruction(MirInstKind::EqStr)),
        "lt" => Some(MirBuiltin::Instruction(MirInstKind::Lt)),
        "gt" => Some(MirBuiltin::Instruction(MirInstKind::Gt)),
        "exit" => Some(MirBuiltin::SysCall(MirSysCallKind::Exit)),
        "printf" => Some(MirBuiltin::Call(MirCallKind::Printf)),
        "sprintf" => Some(MirBuiltin::Call(MirCallKind::Sprintf)),
        "write" => Some(MirBuiltin::Call(MirCallKind::Write)),
        "puts" => Some(MirBuiltin::Call(MirCallKind::Puts)),
        _ => None,
    }
}
