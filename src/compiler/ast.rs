use crate::compiler::span::Span;

// An Item is a block-level construct: function definitions, type definitions, imports (for root level) and execs.
#[derive(Debug, Clone)]
pub enum BlockItem {
    Import {
        name: String,
        span: Span,
    },
    SigDef {
        // TODO: This can be a SigType directly
        name: String,
        term: SigKind,
        generics: Vec<String>,
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
    Int,              // TODO: ast doesn't have str or int type
    Str,              // TODO: ast doesn't have str or int type
    CompileTimeInt,   // TODO: ast doesn't have str or int type
    CompileTimeStr,   // TODO: ast doesn't have str or int type
    Ident(SigIdent),  // `foo`, `str!`, `list`
    Tuple(Signature), // Nested tuple signature: `(int, b:int, tail:list)`

    // Generic instantiation: `arr<int, list>`
    GenericInst { name: String, args: Vec<SigType> },
    Generic(String), // Unbound generic type parameter: `T`
}

impl SigKind {
    // TODO: This is very useful, maybe also add to hir?
    pub fn tuple<I>(items: I) -> SigKind
    where
        I: IntoIterator<Item = SigKind>,
    {
        let sig_items = items
            .into_iter()
            .map(|kind| {
                SigItem {
                    name: None,
                    ty: SigType {
                        kind,
                        span: Span::unknown(), // TODO: real span
                    },
                    is_variadic: false,
                    span: Span::unknown(), // TODO:
                }
            })
            .collect::<Vec<_>>();

        SigKind::Tuple(Signature {
            items: sig_items,
            span: Span::unknown(), // TODO:
        })
    }

    pub fn tuple_vec(items: Vec<SigKind>) -> SigKind {
        Self::tuple(items)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Signature {
    /// The items between the parentheses: (a: int, ...items: arr<int>, ok: (str))
    pub items: Vec<SigItem>,
    /// Span of the entire `( ... )` tuple, including parens.
    pub span: Span,
}

impl Signature {
    pub fn kinds(&self) -> Vec<SigKind> {
        self.items.iter().map(|item| item.ty.kind.clone()).collect()
    }

    pub fn from_kinds<I>(kinds: I, span: Span) -> Signature
    where
        I: IntoIterator<Item = SigKind>,
    {
        let items = kinds
            .into_iter()
            .map(|kind| SigItem {
                name: None,
                ty: SigType {
                    kind,
                    span: Span::unknown(),
                },
                is_variadic: false,
                span: Span::unknown(),
            })
            .collect();
        Signature { items, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SigType {
    pub kind: SigKind,
    /// Span for just the type part (not including param name or `...`).
    pub span: Span,
}

// TODO: Add support for inferred types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SigItem {
    /// Optional name: `x: int` → Some("x"), `int` → None
    pub name: Option<String>,
    /// The type of this item (can itself be a tuple/signature)
    pub ty: SigType,
    /// `true` if prefixed with `...`
    pub is_variadic: bool,
    /// Span of the whole item: e.g. `"x:int"`, `"....items: arr<int>"`, `"int"`.
    pub span: Span,
}
impl SigItem {
    pub fn span(&self) -> Span {
        self.span
    }

    /// Returns the SigKind of the type.
    /// (You may want to rename this to `kind()` later.)
    pub fn ty_kind(&self) -> &SigKind {
        &self.ty.kind
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn into_ty_kind(self) -> SigKind {
        self.ty.kind
    }

    pub fn into_name(self) -> Option<String> {
        self.name
    }

    pub fn is_variadic(&self) -> bool {
        self.is_variadic
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub items: Vec<BlockItem>,
    pub span: Span,
}

impl Block {
    pub fn to_function(self, name: String) -> BlockItem {
        BlockItem::FunctionDef {
            name,
            lambda: Lambda {
                params: Signature {
                    items: Vec::new(),
                    span: Span::unknown(),
                },
                body: self,
                args: Vec::new(),
                span: Span::unknown(),
            },
            span: Span::unknown(),
        }
    }
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
    pub has_bang: bool,
    /// Span of just the identifier (including bang)
    pub span: Span,
}
