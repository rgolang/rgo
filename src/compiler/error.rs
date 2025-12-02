use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;

use crate::compiler::span::Span;

#[derive(Debug)]
pub enum CompileError {
    Io(io::Error),
    Lex(LexError),
    Parse(ParseError),
    Resolve(ResolveError),
    Codegen(CodegenError),
    Internal(String),
}

impl Display for CompileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::Io(err) => write!(f, "io error: {}", err),
            CompileError::Lex(err) => write!(f, "lex error: {}", err),
            CompileError::Parse(err) => write!(f, "parse error: {}", err),
            CompileError::Resolve(err) => write!(f, "resolve error: {}", err),
            CompileError::Codegen(err) => write!(f, "codegen error: {}", err),
            CompileError::Internal(msg) => write!(f, "internal error: {}", msg),
        }
    }
}

impl Error for CompileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CompileError::Io(err) => Some(err),
            CompileError::Lex(err) => Some(err),
            CompileError::Parse(err) => Some(err),
            CompileError::Resolve(err) => Some(err),
            CompileError::Codegen(err) => Some(err),
            CompileError::Internal(_) => None,
        }
    }
}

impl From<io::Error> for CompileError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<LexError> for CompileError {
    fn from(err: LexError) -> Self {
        Self::Lex(err)
    }
}

impl From<ParseError> for CompileError {
    fn from(err: ParseError) -> Self {
        Self::Parse(err)
    }
}

impl From<ResolveError> for CompileError {
    fn from(err: ResolveError) -> Self {
        Self::Resolve(err)
    }
}

impl From<CodegenError> for CompileError {
    fn from(err: CodegenError) -> Self {
        Self::Codegen(err)
    }
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl LexError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl Display for LexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}:{}",
            self.message, self.span.line, self.span.column
        )
    }
}

impl Error for LexError {}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ParseError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}:{}",
            self.message, self.span.line, self.span.column
        )
    }
}

impl Error for ParseError {}

#[derive(Debug, Clone)]
pub struct CodegenError {
    pub message: String,
    pub span: Span,
}

impl CodegenError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl Display for CodegenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}:{}",
            self.message, self.span.line, self.span.column
        )
    }
}

impl Error for CodegenError {}

impl From<io::Error> for CodegenError {
    fn from(err: io::Error) -> Self {
        Self::new(err.to_string(), Span::unknown())
    }
}

#[derive(Debug, Clone)]
pub struct ResolveError {
    pub message: String,
    pub span: Span,
}

impl ResolveError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl Display for ResolveError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}:{}",
            self.message, self.span.line, self.span.column
        )
    }
}

impl Error for ResolveError {}
