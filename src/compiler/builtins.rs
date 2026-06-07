use crate::compiler::hir::{self, SigItem, SigKind, Signature};
use std::collections::BTreeSet;

// TODO: Needed?
#[derive(Debug)]
pub enum BuiltinSpec {
    Function(hir::Signature),
    Type(hir::SigKind),
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
    Write,
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
            "divint" => Some(Builtin::Div),
            "addf64" => Some(Builtin::AddF64),
            "mulf64" => Some(Builtin::MulF64),
            "divf64" => Some(Builtin::DivF64),
            "eq" => Some(Builtin::Eqi),
            "eqi" => Some(Builtin::Eqi),
            "eqs" => Some(Builtin::Eqs),
            "lt" => Some(Builtin::Lt),
            "gt" => Some(Builtin::Gt),
            "write" => Some(Builtin::Write),
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
            Builtin::Eqi => "eq",
            Builtin::Eqs => "eqs",
            Builtin::Lt => "lt",
            Builtin::Gt => "gt",
            Builtin::Write => "write",
            Builtin::Exit => "exit",
            Builtin::Printf => "printf",
            Builtin::Sprintf => "sprintf",
        }
    }

    pub fn signature(self) -> Signature {
        match self {
            Builtin::Add | Builtin::Sub | Builtin::Mul => math_binary_sig(SigKind::Int),
            Builtin::Div => div_sig(),
            Builtin::AddF64 | Builtin::MulF64 | Builtin::DivF64 => math_binary_sig(SigKind::F64),
            Builtin::Eq | Builtin::Eqi | Builtin::Lt | Builtin::Gt => comparison_sig(SigKind::Int),
            Builtin::Eqs => comparison_sig(SigKind::Str),
            Builtin::Write => sig_from_items(vec![
                sig_item("value", SigKind::Str),
                sig_item("ok", SigKind::tuple([])),
            ]),
            Builtin::Exit => sig_from_items(vec![sig_item("code", SigKind::Int)]),
            Builtin::Printf => sig_from_items(vec![
                sig_item("format", SigKind::CompileTimeStr),
                sig_item("args", SigKind::Variadic),
                sig_item("ok", SigKind::tuple([])),
            ]),
            Builtin::Sprintf => sig_from_items(vec![
                sig_item("format", SigKind::CompileTimeStr),
                sig_item("args", SigKind::Variadic),
                sig_item("ok", SigKind::tuple([SigKind::Str])),
            ]),
        }
    }

    pub fn is_call(self) -> bool {
        matches!(
            self,
            Builtin::Printf | Builtin::Sprintf | Builtin::Write | Builtin::Exit
        )
    }

    pub fn is_conditional(self) -> bool {
        matches!(self, Builtin::Eq | Builtin::Eqi | Builtin::Eqs)
    }

    pub fn is_instruction(self) -> bool {
        matches!(
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
        )
    }

    pub fn is_libc_call(self) -> bool {
        matches!(
            self,
            Builtin::Printf | Builtin::Sprintf | Builtin::Write | Builtin::Exit
        )
    }
}

pub fn get_spec(name: &str) -> Option<BuiltinSpec> {
    if let Some(builtin) = Builtin::from_name(name) {
        return Some(BuiltinSpec::Function(builtin.signature()));
    }
    match name {
        "byte" => Some(BuiltinSpec::Type(hir::SigKind::Byte)),
        "int" => Some(BuiltinSpec::Type(hir::SigKind::Int)),
        "str" => Some(BuiltinSpec::Type(hir::SigKind::Str)),
        "f64" => Some(BuiltinSpec::Type(hir::SigKind::F64)),
        _ => None,
    }
}

fn sig_item(name: &str, ty: SigKind) -> SigItem {
    SigItem {
        name: name.to_string(),
        kind: ty,
        has_bang: false,
    }
}

fn tuple_sig(items: Vec<SigItem>) -> SigKind {
    SigKind::Sig(Signature {
        items,
        generics: BTreeSet::new(),
    })
}

fn sig_from_items(items: Vec<SigItem>) -> Signature {
    Signature {
        items,
        generics: BTreeSet::new(),
    }
}

fn comparison_sig(arg_kind: SigKind) -> Signature {
    sig_from_items(vec![
        sig_item("left", arg_kind.clone()),
        sig_item("right", arg_kind),
        sig_item("ok", SigKind::tuple([])),
        sig_item("err", SigKind::tuple([])),
    ])
}

fn div_sig() -> Signature {
    let err_sig = tuple_sig(vec![sig_item("res", SigKind::Int)]);
    let ok_sig = tuple_sig(vec![sig_item("res", SigKind::Int)]);
    sig_from_items(vec![
        sig_item("x", SigKind::Int),
        sig_item("y", SigKind::Int),
        sig_item("err", err_sig),
        sig_item("ok", ok_sig),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f64_type_registration() {
        match get_spec("f64") {
            Some(BuiltinSpec::Type(kind)) => assert_eq!(kind, hir::SigKind::F64),
            other => panic!("expected builtin f64 type, got {:?}", other),
        }
    }

    #[test]
    fn byte_type_registration() {
        match get_spec("byte") {
            Some(BuiltinSpec::Type(kind)) => assert_eq!(kind, hir::SigKind::Byte),
            other => panic!("expected builtin byte type, got {:?}", other),
        }
    }

    #[test]
    fn f64_math_signature_contains_f64() {
        let builtin = Builtin::from_name("addf64").expect("addf64 builtin should exist");
        let sig = builtin.signature();
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

fn math_binary_sig(arg_kind: SigKind) -> Signature {
    let result_sig = tuple_sig(vec![sig_item("res", arg_kind.clone())]);
    sig_from_items(vec![
        sig_item("x", arg_kind.clone()),
        sig_item("y", arg_kind.clone()),
        sig_item("ok", result_sig),
    ])
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
