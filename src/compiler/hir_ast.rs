pub use crate::compiler::hir_context::{Context, ContextEntry};
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SigKind {
    Int,
    Str,
    F64,
    Variadic,
    CompileTimeInt,
    CompileTimeStr,
    Ident(SigIdent),
    Sig(Signature),
    GenericInst { name: String, args: Vec<SigKind> },
    Generic(String),
}

impl SigKind {
    pub fn tuple<I>(items: I) -> SigKind
    where
        I: IntoIterator<Item = SigKind>,
    {
        SigKind::Sig(Signature::from_tuple(items))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Signature {
    pub items: Vec<SigItem>,
    pub generics: BTreeSet<String>,
}

impl Signature {
    pub fn from_kinds<I>(kinds: I) -> Signature
    where
        I: IntoIterator<Item = SigKind>,
    {
        let items = kinds
            .into_iter()
            .map(|kind| SigItem {
                name: String::new(),
                kind,
                has_bang: false,
            })
            .collect();
        Signature {
            items,
            generics: BTreeSet::new(),
        }
    }

    pub fn kinds(&self) -> Vec<SigKind> {
        self.items.iter().map(|item| item.kind.clone()).collect()
    }

    pub fn from_tuple<I>(items: I) -> Signature
    where
        I: IntoIterator<Item = SigKind>,
    {
        let sig_items = items
            .into_iter()
            .map(|kind| SigItem {
                name: String::new(),
                kind,
                has_bang: false,
            })
            .collect();
        Signature {
            items: sig_items,
            generics: BTreeSet::new(),
        }
    }

    pub fn is_variadic(&self) -> bool {
        self.items
            .iter()
            .any(|item| matches!(item.kind, SigKind::Variadic))
    }

    pub fn names(&self) -> Vec<String> {
        self.items.iter().map(|item| item.name.clone()).collect()
    }
}

#[derive(Debug, Clone)]
pub struct SigItem {
    pub name: String,
    pub kind: SigKind,
    pub has_bang: bool,
}

impl Eq for SigItem {}

impl PartialEq for SigItem {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.has_bang == other.has_bang
    }
}

impl Hash for SigItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.has_bang.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SigIdent {
    pub name: String,
}

#[derive(Clone, Debug)]
pub enum Lit {
    Str(String),
    Int(isize),
    F64(f64),
}

impl PartialEq for Lit {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Lit::Str(left), Lit::Str(right)) => left == right,
            (Lit::Int(left), Lit::Int(right)) => left == right,
            (Lit::F64(left), Lit::F64(right)) => left.to_bits() == right.to_bits(),
            _ => false,
        }
    }
}

impl Eq for Lit {}

impl Hash for Lit {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Lit::Str(value) => {
                state.write_u8(0);
                value.hash(state);
            }
            Lit::Int(value) => {
                state.write_u8(1);
                value.hash(state);
            }
            Lit::F64(value) => {
                state.write_u8(2);
                state.write_u64(value.to_bits());
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub sig: Signature,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub items: Vec<BlockItem>,
}

#[derive(Debug, Clone)]
pub enum BlockItem {
    Import { label: String, path: String },
    FunctionDef(Function),
    SigDef { name: String, sig: Signature },
    LitDef { name: String, literal: Lit },
    ClosureDef(Closure),
    Exec(Exec),
}

#[derive(Debug, Clone)]
pub struct Exec {
    pub of: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub name: String,
    pub of: String,
    pub args: Vec<String>,
}
