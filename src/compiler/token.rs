use crate::compiler::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Eof,
    Ident(String),
    Import(String),
    IntLiteral(i64),
    StringLiteral(String),
    Arrow,
    FatArrow,
    Comma,
    Colon,
    Semicolon,
    Ellipsis,
    Dot,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Equals,
    Plus,
    Minus,
    Star,
    Slash,
    Bang,
    Newline,
    Question,
    AngleOpen,
    AngleClose,
}
