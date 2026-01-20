use crate::compiler::ast::{self, SigItem, SigKind, Signature};
use crate::compiler::span::Span;

pub enum BuiltinSpec {
    Function(ast::Signature),
    Type(ast::SigKind),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Builtin {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Eqi,
    Eqs,
    Lt,
    Gt,
    Itoa,
    Write,
    Puts,
    Exit,
    Printf,
    Sprintf,
}

impl Builtin {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "add" => Some(Builtin::Add),
            "sub" => Some(Builtin::Sub),
            "mul" => Some(Builtin::Mul),
            "div" => Some(Builtin::Div),
            "eq" => Some(Builtin::Eq),
            "eqi" => Some(Builtin::Eqi),
            "eqs" => Some(Builtin::Eqs),
            "lt" => Some(Builtin::Lt),
            "gt" => Some(Builtin::Gt),
            "itoa" => Some(Builtin::Itoa),
            "write" => Some(Builtin::Write),
            "puts" => Some(Builtin::Puts),
            "exit" => Some(Builtin::Exit),
            "printf" => Some(Builtin::Printf),
            "sprintf" => Some(Builtin::Sprintf),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Builtin::Add => "add",
            Builtin::Sub => "sub",
            Builtin::Mul => "mul",
            Builtin::Div => "div",
            Builtin::Eq => "eq",
            Builtin::Eqi => "eqi",
            Builtin::Eqs => "eqs",
            Builtin::Lt => "lt",
            Builtin::Gt => "gt",
            Builtin::Itoa => "itoa",
            Builtin::Write => "write",
            Builtin::Puts => "puts",
            Builtin::Exit => "exit",
            Builtin::Printf => "printf",
            Builtin::Sprintf => "sprintf",
        }
    }

    pub fn signature(self, span: Span) -> Signature {
        match self {
            Builtin::Add | Builtin::Sub | Builtin::Mul | Builtin::Div => math_binary_sig(span),
            Builtin::Eq | Builtin::Eqi | Builtin::Lt | Builtin::Gt => {
                comparison_sig(SigKind::Int, span)
            }
            Builtin::Eqs => comparison_sig(SigKind::Str, span),
            Builtin::Itoa => sig_from_items(
                vec![
                    sig_item("value", SigKind::Int, span),
                    sig_item("ok", SigKind::tuple([SigKind::Str]), span),
                ],
                span,
            ),
            Builtin::Write | Builtin::Puts => sig_from_items(
                vec![
                    sig_item("value", SigKind::Str, span),
                    sig_item("ok", SigKind::tuple([]), span),
                ],
                span,
            ),
            Builtin::Exit => sig_from_items(vec![sig_item("code", SigKind::Int, span)], span),
            Builtin::Printf => sig_from_items(
                vec![
                    sig_item("format", SigKind::CompileTimeStr, span),
                    sig_item("args", SigKind::Variadic, span),
                    sig_item("ok", SigKind::tuple([]), span),
                ],
                span,
            ),
            Builtin::Sprintf => sig_from_items(
                vec![
                    sig_item("format", SigKind::CompileTimeStr, span),
                    sig_item("args", SigKind::Variadic, span),
                    sig_item("ok", SigKind::tuple([SigKind::Str]), span),
                ],
                span,
            ),
        }
    }

    pub fn is_call(self) -> bool {
        return matches!(
            self,
            Builtin::Printf | Builtin::Sprintf | Builtin::Write | Builtin::Puts | Builtin::Exit
        );
    }

    pub fn is_conditional(self) -> bool {
        return matches!(self, Builtin::Eq | Builtin::Eqi | Builtin::Eqs);
    }

    pub fn is_instruction(self) -> bool {
        return matches!(
            self,
            Builtin::Add
                | Builtin::Sub
                | Builtin::Mul
                | Builtin::Div
                | Builtin::Lt
                | Builtin::Gt
        );
    }

    pub fn is_libc_call(self) -> bool {
        return matches!(
            self,
            Builtin::Printf | Builtin::Sprintf | Builtin::Write | Builtin::Puts | Builtin::Exit
        );
    }
}

pub fn get_spec(name: &str, span: Span) -> Option<BuiltinSpec> {
    if let Some(builtin) = Builtin::from_name(name) {
        return Some(BuiltinSpec::Function(builtin.signature(span)));
    }
    match name {
        "int" => Some(BuiltinSpec::Type(ast::SigKind::Int)),
        "str" => Some(BuiltinSpec::Type(ast::SigKind::Str)),
        _ => None,
    }
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
pub enum AirRuntimeHelper {
    ReleaseHeapPtr,
    DeepCopyHeapPtr,
    MemcpyHelper,
}

impl AirRuntimeHelper {
    pub fn name(&self) -> &'static str {
        match self {
            AirRuntimeHelper::ReleaseHeapPtr => "release_heap_ptr",
            AirRuntimeHelper::DeepCopyHeapPtr => "deepcopy_heap_ptr",
            AirRuntimeHelper::MemcpyHelper => "memcpy_helper",
        }
    }
}
