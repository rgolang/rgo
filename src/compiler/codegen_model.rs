use std::fmt;

pub struct AsmModule {
    pub functions: Vec<AsmFunction>,
}

impl AsmModule {
    pub fn new(functions: Vec<AsmFunction>) -> Self {
        Self { functions }
    }
}

impl Default for AsmModule {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

#[derive(Clone, Debug)]
pub struct AsmFunction {
    pub name: String,
    pub params: Vec<String>,
    pub block: AsmBlock,
}

impl AsmFunction {
    pub fn new(name: String, params: Vec<String>, statements: Vec<AsmStatement>) -> Self {
        let block = AsmBlock {
            label: name.clone(),
            statements,
        };
        Self {
            name,
            params,
            block,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AsmBlock {
    pub label: String,
    pub statements: Vec<AsmStatement>,
}

#[derive(Clone, Debug)]
pub enum AsmStatement {
    Literal {
        target: String,
        value: AsmLiteral,
    },
    Invocation {
        target: Option<String>,
        callee: String,
        args: Vec<String>,
        tail: bool,
    },
    HeapAlloc {
        size: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        offset: i32,
    },
}

#[derive(Clone, Debug)]
pub enum AsmLiteral {
    Int(i64),
    Str(String),
}

impl fmt::Display for AsmModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, func) in self.functions.iter().enumerate() {
            writeln!(f, "{func}")?;
            if idx + 1 < self.functions.len() {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for AsmFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params = if self.params.is_empty() {
            "()".to_string()
        } else {
            let joined = self.params.join(", ");
            format!("({joined})")
        };
        writeln!(f, "{}{} {{", self.block.label, params)?;
        for stmt in &self.block.statements {
            writeln!(f, "    {stmt}")?;
        }
        writeln!(f, "}}")
    }
}

impl fmt::Display for AsmStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AsmStatement::Literal { target, value } => write!(f, "{target} = {value}"),
            AsmStatement::Invocation {
                target,
                callee,
                args,
                tail,
            } => {
                let args = args.join(", ");
                let call = if let Some(target) = target {
                    format!("{target} = {callee}({args})")
                } else {
                    format!("{callee}({args})")
                };
                if *tail {
                    write!(f, "{}", call)
                } else {
                    write!(f, "{call}")
                }
            }
            AsmStatement::HeapAlloc {
                size,
                prot,
                flags,
                fd,
                offset,
            } => write!(
                f,
                "heap_alloc mmap(size={}, prot={}, flags={}, fd={}, offset={})",
                size, prot, flags, fd, offset
            ),
        }
    }
}

impl fmt::Display for AsmLiteral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AsmLiteral::Int(value) => write!(f, "{}", value),
            AsmLiteral::Str(value) => {
                write!(f, "\"")?;
                for c in value.chars() {
                    match c {
                        '\\' => write!(f, "\\\\")?,
                        '"' => write!(f, "\\\"")?,
                        '\n' => write!(f, "\\n")?,
                        '\r' => write!(f, "\\r")?,
                        '\t' => write!(f, "\\t")?,
                        other => write!(f, "{other}")?,
                    }
                }
                write!(f, "\"")?;
                Ok(())
            }
        }
    }
}
