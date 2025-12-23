use crate::compiler::span::Span;

// An Item is a block-level construct: function definitions, type definitions, imports (for root level) and execs.
#[derive(Debug, Clone)]
pub enum BlockItem {
    Import {
        name: String,
        span: Span,
    },
    SigDef {
        name: String,
        sig: Signature,
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
        params: Signature,
        continuation: Block,
        term: Term,
        span: Span,
    },
}

impl BlockItem {
    pub fn span(&self) -> Span {
        match self {
            BlockItem::Import { span, .. }
            | BlockItem::SigDef { span, .. }
            | BlockItem::FunctionDef { span, .. }
            | BlockItem::StrDef { span, .. }
            | BlockItem::IntDef { span, .. }
            | BlockItem::IdentDef { span, .. } => *span,
            BlockItem::ScopeCapture { span, .. } => *span,
            BlockItem::Ident(ident) => ident.span,
            BlockItem::Lambda(lambda) => lambda.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SigKind {
    Int,
    Str,
    Variadic, // TODO: This is before we have staged DSL
    CompileTimeInt,
    CompileTimeStr,
    Ident(SigIdent),                                  // `foo`, `str`, `list`
    Sig(Signature), // Nested tuple signature: `(int, b:int, tail:list)`
    GenericInst { name: String, args: Vec<SigKind> }, // Generic instantiation: `arr<int, list>`
    Generic(String), // Unbound generic type parameter: `T`
}

impl SigKind {
    pub fn tuple<I>(items: I) -> SigKind
    where
        I: IntoIterator<Item = SigKind>,
    {
        SigKind::Sig(Signature::from_tuple(items, Span::unknown()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Signature {
    /// The items between the parentheses: (a: int, ...items: arr<int>, ok: (str))
    pub items: Vec<SigItem>,
    /// Span of the entire `( ... )` tuple, including parens.
    pub span: Span,
    /// Generic parameters declared just before the signature (e.g. `<T>`).
    pub generics: Vec<String>,
}

impl Signature {
    pub fn from_kinds<I>(kinds: I, span: Span) -> Signature
    where
        I: IntoIterator<Item = SigKind>,
    {
        let items = kinds
            .into_iter()
            .map(|kind| SigItem {
                name: String::new(), // TODO: could avoid empty string...
                ty: kind,
                has_bang: false,
                span: Span::unknown(),
            })
            .collect();
        Signature {
            items,
            span,
            generics: Vec::new(),
        }
    }

    pub fn kinds(&self) -> Vec<SigKind> {
        self.items.iter().map(|item| item.ty.clone()).collect()
    }
    pub fn from_tuple<I>(items: I, span: Span) -> Signature
    where
        I: IntoIterator<Item = SigKind>,
    {
        let sig_items = items
            .into_iter()
            .map(|kind| SigItem {
                name: String::new(), // TODO: could avoid empty string...
                ty: kind,            // TODO: Rename to kind
                has_bang: false,
                span: Span::unknown(),
            })
            .collect();
        Signature {
            items: sig_items,
            span: span,
            generics: Vec::new(),
        }
    }
    pub fn is_variadic(&self) -> bool {
        self.items
            .iter()
            .any(|item| matches!(item.ty, SigKind::Variadic))
    }
    pub fn names(&self) -> Vec<String> {
        self.items.iter().map(|item| item.name.clone()).collect()
    }
}

#[derive(Debug, Clone)]
pub struct SigItem {
    pub name: String,
    pub ty: SigKind,
    pub has_bang: bool,
    pub span: Span,
}
impl Eq for SigItem {}
impl PartialEq for SigItem {
    fn eq(&self, other: &Self) -> bool {
        // name is ignored for comparison
        self.ty == other.ty && self.has_bang == other.has_bang
    }
}

impl std::hash::Hash for SigItem {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ty.hash(state);
        self.has_bang.hash(state);
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub items: Vec<BlockItem>,
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
    pub params: Signature,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SigIdent {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Arg {
    pub name: String,
    pub span: Span,
}
