pub use crate::compiler::hir_scope::{ConstantValue, Scope, ScopeItem};
use crate::compiler::span::Span;

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub sig: Signature,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Signature {
    pub items: Vec<SigItem>,
    pub span: Span,
}

impl Signature {
    pub fn variadic_flags(&self) -> Vec<bool> {
        return self
            .items
            .iter()
            .map(|item| item.is_variadic)
            .collect::<Vec<_>>();
    }
}

#[derive(Debug, Clone)]
pub struct SigItem {
    pub name: String,
    pub ty: SigType,
    // TODO: Move has_bang to SigItem, it's a not part of the type
    pub is_variadic: bool,
    pub span: Span,
}
impl Eq for SigItem {}
impl PartialEq for SigItem {
    fn eq(&self, other: &Self) -> bool {
        // name is ignored for comparison
        self.ty == other.ty && self.is_variadic == other.is_variadic
    }
}

impl std::hash::Hash for SigItem {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ty.hash(state);
        self.is_variadic.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SigType {
    pub kind: SigKind,
    /// Span for just the type part (not including param name or `...`).
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SigKind {
    Int,
    Str,
    CompileTimeInt,
    CompileTimeStr,
    Tuple(Signature),
    Ident(SigIdent),
    GenericInst {
        name: String,
        args: Vec<SigKind>, // TODO: in ast this is SigType, not sure it matters
    },
    Generic(String), // TODO: Should we keep generics in hir? types are easy to normalize
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SigIdent {
    pub name: String,
    pub has_bang: bool,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub items: Vec<BlockItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum BlockItem {
    Import {
        name: String,
        span: Span,
    },
    FunctionDef(Function),
    SigDef {
        name: String,
        kind: SigKind,
        span: Span,
        generics: Vec<String>,
    },
    StrDef(StrLiteral),
    IntDef(IntLiteral),
    ApplyDef(Apply),
    Exec(Exec),
}

#[derive(Debug, Clone)]
pub struct StrLiteral {
    pub name: String,
    pub value: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IntLiteral {
    pub name: String,
    pub value: i64,
    pub span: Span,
}

impl BlockItem {
    pub fn span(&self) -> Span {
        match self {
            BlockItem::FunctionDef(function) => function.span,
            BlockItem::StrDef(StrLiteral { span, .. })
            | BlockItem::IntDef(IntLiteral { span, .. })
            | BlockItem::ApplyDef(Apply { span, .. })
            | BlockItem::Import { span, .. }
            | BlockItem::Exec(Exec { span, .. }) => *span,
            _ => Span::unknown(), // TODO: Handle the unknown
        }
    }

    pub fn binding_info(&self) -> Option<(&String, Span)> {
        match self {
            BlockItem::FunctionDef(function) => Some((&function.name, function.span)),
            BlockItem::StrDef(literal) => Some((&literal.name, literal.span)),
            BlockItem::IntDef(literal) => Some((&literal.name, literal.span)),
            BlockItem::Exec(Exec {
                result: Some(name),
                span,
                ..
            }) => Some((name, *span)),
            BlockItem::ApplyDef(Apply { name, span, .. }) => Some((name, *span)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Exec {
    pub of: String,
    pub args: Vec<Arg>,
    pub span: Span,
    pub result: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Apply {
    pub name: String,
    pub of: String,
    pub args: Vec<Arg>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Arg {
    pub name: String,
    pub span: Span,
}
