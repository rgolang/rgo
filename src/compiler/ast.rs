use crate::compiler::span::Span;

// TODO: Move to parser.rs
// TODO: Rename Item to BlockItem
// An Item is a block-level construct: function definitions, type definitions, imports (for root level) and invocations.
#[derive(Debug, Clone)]
pub enum Item {
    Import {
        name: String,
        span: Span,
        is_libc: bool,
    },
    TypeDef {
        name: String,
        term: TypeRef,
        span: Span,
    },
    FunctionDef {
        name: String,
        lambda: Lambda,
        span: Span,
    },
    StrDef {
        name: String,
        literal: StrLiteral,
        span: Span,
    },
    IntDef {
        name: String,
        literal: IntLiteral,
        span: Span,
    },
    IdentDef {
        name: String,
        ident: Ident,
        span: Span,
    },
    Lambda(Lambda),
    Ident(Ident),
    ScopeCapture {
        params: Params,
        term: Term,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeRef {
    Int,
    Str,
    CompileTimeInt,
    CompileTimeStr,
    Type(Vec<TypeRef>),
    Alias(String),
    AliasInstance { name: String, args: Vec<TypeRef> },
    Generic(String),
}

#[derive(Debug, Clone)]
pub struct Params {
    pub items: Vec<Param>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Param {
    NameAndType {
        name: String,
        ty: TypeRef,
        span: Span,
        is_variadic: bool,
    },
    TypeOnly {
        ty: TypeRef,
        span: Span,
        is_variadic: bool,
    },
    NameOnly {
        name: String,
        span: Span,
        is_variadic: bool,
    },
}

impl Param {
    pub fn span(&self) -> Span {
        match self {
            Param::NameAndType { span, .. }
            | Param::TypeOnly { span, .. }
            | Param::NameOnly { span, .. } => *span,
        }
    }

    pub fn ty(&self) -> Option<&TypeRef> {
        match self {
            Param::NameAndType { ty, .. } | Param::TypeOnly { ty, .. } => Some(ty),
            Param::NameOnly { .. } => None,
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Param::NameAndType { name, .. } | Param::NameOnly { name, .. } => Some(name),
            Param::TypeOnly { .. } => None,
        }
    }

    pub fn into_ty(self) -> Option<TypeRef> {
        match self {
            Param::NameAndType { ty, .. } | Param::TypeOnly { ty, .. } => Some(ty),
            Param::NameOnly { .. } => None,
        }
    }

    pub fn into_name(self) -> Option<String> {
        match self {
            Param::NameAndType { name, .. } | Param::NameOnly { name, .. } => Some(name),
            Param::TypeOnly { .. } => None,
        }
    }

    pub fn is_variadic(&self) -> bool {
        match self {
            Param::NameAndType { is_variadic, .. }
            | Param::TypeOnly { is_variadic, .. }
            | Param::NameOnly { is_variadic, .. } => *is_variadic,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Term {
    Int(IntLiteral),
    String(StrLiteral),
    Lambda(Lambda),
    Ident(Ident),
}

impl Term {
    pub fn span(&self) -> Span {
        match self {
            Term::Int(literal) => literal.span,
            Term::Ident(ident) => ident.span,
            Term::String(literal) => literal.span,
            Term::Lambda(lambda) => lambda.span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StrLiteral {
    pub value: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IntLiteral {
    pub value: i64,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Lambda {
    pub params: Params,
    pub body: Block,
    pub args: Vec<Term>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Ident {
    pub name: String,
    pub args: Vec<Term>,
    pub span: Span,
}
