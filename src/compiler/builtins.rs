use crate::compiler::ast::{self, SigItem, SigKind, Signature};
use crate::compiler::span::Span;

#[derive(Debug)]
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
    AddF64,
    MulF64,
    DivF64,
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
            "addf64" => Some(Builtin::AddF64),
            "mulf64" => Some(Builtin::MulF64),
            "divf64" => Some(Builtin::DivF64),
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
            Builtin::AddF64 => "addf64",
            Builtin::MulF64 => "mulf64",
            Builtin::DivF64 => "divf64",
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
            Builtin::Add | Builtin::Sub | Builtin::Mul | Builtin::Div => {
                math_binary_sig(SigKind::Int, span)
            }
            Builtin::AddF64 | Builtin::MulF64 | Builtin::DivF64 => {
                math_binary_sig(SigKind::F64, span)
            }
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
                | Builtin::AddF64
                | Builtin::MulF64
                | Builtin::DivF64
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
        "f64" => Some(BuiltinSpec::Type(ast::SigKind::F64)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::span::Span;

    #[test]
    fn f64_type_registration() {
        let span = Span::unknown();
        match get_spec("f64", span) {
            Some(BuiltinSpec::Type(kind)) => assert_eq!(kind, ast::SigKind::F64),
            other => panic!("expected builtin f64 type, got {:?}", other),
        }
    }

    #[test]
    fn f64_math_signature_contains_f64() {
        let span = Span::unknown();
        let builtin = Builtin::from_name("addf64").expect("addf64 builtin should exist");
        let sig = builtin.signature(span);
        assert_eq!(sig.items.len(), 3);
        assert_eq!(sig.items[0].kind, SigKind::F64);
        assert_eq!(sig.items[1].kind, SigKind::F64);

        let tuple = match &sig.items[2].kind {
            SigKind::Sig(inner) => inner,
            other => panic!("expected tuple for ok, got {:?}", other),
        };
        assert_eq!(tuple.items.len(), 1);
        assert_eq!(tuple.items[0].kind, SigKind::F64);
    }

    #[test]
    fn builtin_variants_exist_for_float_ops() {
        assert!(Builtin::from_name("mulf64").is_some());
        assert!(Builtin::from_name("divf64").is_some());
    }
}

fn math_binary_sig(arg_kind: SigKind, span: Span) -> Signature {
    let result_sig = tuple_sig(vec![sig_item("res", arg_kind.clone(), span)], span);
    sig_from_items(
        vec![
            sig_item("x", arg_kind.clone(), span),
            sig_item("y", arg_kind.clone(), span),
            sig_item("ok", result_sig, span),
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
