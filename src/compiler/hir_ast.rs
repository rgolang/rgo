use crate::compiler::ast;
use crate::compiler::ast::Signature;
pub use crate::compiler::hir_context::{Context, ContextEntry};
use crate::compiler::span::Span;

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub sig: Signature,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub items: Vec<BlockItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum BlockItem {
    Import {
        label: String,
        path: String,
        span: Span,
    },
    FunctionDef(Function),
    SigDef {
        name: String,
        sig: Signature,
        span: Span,
    },
    LitDef {
        name: String,
        literal: ast::Literal,
    },
    ClosureDef(Closure),
    Exec(Exec),
}

impl BlockItem {
    pub fn span(&self) -> Span {
        match self {
            BlockItem::FunctionDef(function) => function.span,
            BlockItem::LitDef { literal, .. } => literal.span,
            BlockItem::ClosureDef(Closure { span, .. })
            | BlockItem::Import { span, .. }
            | BlockItem::Exec(Exec { span, .. }) => *span,
            _ => Span::unknown(), // TODO: Handle the unknown
        }
    }
}

#[derive(Debug, Clone)]
pub struct Exec {
    pub of: String,
    pub args: Vec<ast::Arg>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub name: String,
    pub of: String,
    pub args: Vec<ast::Arg>,
    pub span: Span,
}
