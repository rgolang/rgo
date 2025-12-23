use std::error::Error as StdError;
use std::fmt::{self, Display, Formatter};
use std::io;

use crate::compiler::span::Span;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Code {
    Io,
    Lex,
    Parse,
    HIR,
    Resolve,
    Codegen,
    Internal,
}

impl Display for Code {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let code = match self {
            Code::Io => "io",
            Code::Lex => "lex",
            Code::Parse => "parse",
            Code::HIR => "hir",
            Code::Resolve => "resolve",
            Code::Codegen => "codegen",
            Code::Internal => "internal",
        };
        f.write_str(code)
    }
}

#[derive(Debug, Clone)]
pub struct Error {
    pub code: Code,
    pub message: String,
    pub span: Span,
}

impl Error {
    pub fn new(code: Code, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            message: message.into(),
            span,
        }
    }
}

pub fn new(code: Code, message: impl Into<String>, span: Span) -> Error {
    return Error {
        code,
        message: message.into(),
        span,
    };
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} at {}:{}",
            self.code, self.message, self.span.line, self.span.column
        )
    }
}

impl StdError for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::new(Code::Io, err.to_string(), Span::unknown())
    }
}
