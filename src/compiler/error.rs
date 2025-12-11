use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;

use crate::compiler::span::Span;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CompileErrorCode {
    Io,
    Lex,
    Parse,
    Resolve,
    Codegen,
    Internal,
}

impl Display for CompileErrorCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let code = match self {
            CompileErrorCode::Io => "io",
            CompileErrorCode::Lex => "lex",
            CompileErrorCode::Parse => "parse",
            CompileErrorCode::Resolve => "resolve",
            CompileErrorCode::Codegen => "codegen",
            CompileErrorCode::Internal => "internal",
        };
        f.write_str(code)
    }
}

#[derive(Debug, Clone)]
pub struct CompileError {
    pub code: CompileErrorCode,
    pub message: String,
    pub span: Span,
}

impl CompileError {
    pub fn new(code: CompileErrorCode, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            message: message.into(),
            span,
        }
    }
}

impl Display for CompileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} at {}:{}",
            self.code, self.message, self.span.line, self.span.column
        )
    }
}

impl Error for CompileError {}

impl From<io::Error> for CompileError {
    fn from(err: io::Error) -> Self {
        Self::new(CompileErrorCode::Io, err.to_string(), Span::unknown())
    }
}
