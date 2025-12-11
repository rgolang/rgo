use crate::compiler::ast;
pub use crate::compiler::hir_context::{ConstantValue, Context, ContextEntry};
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

impl SigKind {
    pub fn tuple<I>(items: I) -> SigKind
    where
        I: IntoIterator<Item = SigKind>,
    {
        let sig_items = items
            .into_iter()
            .map(|kind| SigItem {
                name: String::new(),
                ty: SigType {
                    kind,
                    span: Span::unknown(),
                },
                is_variadic: false,
                span: Span::unknown(),
            })
            .collect();
        SigKind::Tuple(Signature {
            items: sig_items,
            span: Span::unknown(),
        })
    }
}

impl From<&ast::SigType> for SigType {
    fn from(ast_sig_type: &ast::SigType) -> Self {
        SigType {
            kind: SigKind::from(&ast_sig_type.kind),
            span: ast_sig_type.span,
        }
    }
}

impl From<&ast::SigItem> for SigItem {
    fn from(ast_item: &ast::SigItem) -> Self {
        SigItem {
            name: ast_item.name.clone().unwrap_or_default(),
            ty: SigType::from(&ast_item.ty),
            is_variadic: ast_item.is_variadic,
            span: ast_item.span,
        }
    }
}

impl From<&ast::Signature> for Signature {
    fn from(ast_signature: &ast::Signature) -> Self {
        Signature {
            items: ast_signature.items.iter().map(SigItem::from).collect(),
            span: ast_signature.span,
        }
    }
}

impl From<&ast::SigKind> for SigKind {
    fn from(ast_kind: &ast::SigKind) -> Self {
        match ast_kind {
            ast::SigKind::Tuple(signature) => SigKind::Tuple(Signature::from(signature)),
            ast::SigKind::Ident(ident) => SigKind::Ident(SigIdent {
                name: ident.name.clone(),
                has_bang: ident.has_bang,
            }),
            ast::SigKind::GenericInst { name, args } => SigKind::GenericInst {
                name: name.clone(),
                args: args.iter().map(|arg| SigKind::from(&arg.kind)).collect(),
            },
            ast::SigKind::Generic(name) => SigKind::Generic(name.clone()),
        }
    }
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
