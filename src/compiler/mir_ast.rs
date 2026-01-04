use crate::compiler::ast;
pub use crate::compiler::ast::Arg;
pub use crate::compiler::ast::{SigItem, SigKind};
use crate::compiler::builtins::MirInstKind;
pub use crate::compiler::builtins::{MirBuiltin, MirCallKind, MirSysCallKind};
use crate::compiler::span::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    Word,
    Closure,
    Variadic,
}

#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub name: String, // TODO: Maybe make it &str?
    pub params: Vec<SigItem>,
    pub span: Span,
    pub builtin: Option<MirBuiltin>,
}

impl FunctionSig {
    pub fn param_kinds(&self) -> Vec<SigKind> {
        self.params.iter().map(|item| item.kind.clone()).collect()
    }

    pub fn is_variadic(&self) -> bool {
        self.params
            .iter()
            .any(|param| matches!(param.kind, SigKind::Variadic))
    }
}

#[derive(Clone, Debug)]
pub enum MirExecTarget {
    Function(FunctionSig),
    Closure { name: String },
}

#[derive(Clone, Debug)]
pub struct MirFunction {
    pub sig: FunctionSig,
    pub items: Vec<MirStmt>,
}

impl MirFunction {
    pub fn builtin_internal_array_str_nth() -> Self {
        Self {
            sig: FunctionSig {
                name: "internal_array_str_nth".to_string(),
                params: Vec::new(),
                span: Span::unknown(),
                builtin: None,
            },
            items: Vec::new(),
        }
    }

    pub fn builtin_internal_array_str() -> Self {
        Self {
            sig: FunctionSig {
                name: "internal_array_str".to_string(),
                params: Vec::new(),
                span: Span::unknown(),
                builtin: None,
            },
            items: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ReleaseClosure {
    pub name: String,
}

// TODO: Rename to MirEnvField.
#[derive(Clone, Debug)]
pub struct MirField {
    pub env_end: String,
    pub offset_from_end: usize,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct MirCopyField {
    pub env_end: String,
    pub offset: isize,
    pub env_word_count: usize,
    pub result: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct MirLabel {
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct MirJump {
    pub target: String,
}

#[derive(Clone, Debug)]
pub enum MirComparisonKind {
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

#[derive(Clone, Debug)]
pub enum MirOperand {
    Binding(String),
    Literal(i64),
}

#[derive(Clone, Debug)]
pub struct MirCondJump {
    pub left: MirOperand,
    pub right: MirOperand,
    pub kind: MirComparisonKind,
    pub target: String,
}

#[derive(Clone, Debug)]
pub struct MirReturn {
    pub value: Option<String>,
}

#[derive(Clone, Debug)]
pub enum MirStmt {
    EnvField(MirEnvField),
    StrDef {
        name: String,
        literal: ast::StrLiteral,
    },
    IntDef {
        name: String,
        literal: ast::IntLiteral,
    },
    Exec(MirExec),
    Closure(MirClosure),
    ReleaseClosure(ReleaseClosure),
    Op(MirInstruction),
    SysCall(MirSysCall),
    Call(MirCall),
    Copy(MirCopyField),
    Label(MirLabel),
    Jump(MirJump),
    CondJump(MirCondJump),
    Return(MirReturn),
}

#[derive(Clone, Debug)]
pub struct MirEnvField {
    pub result: String,
    pub env_end: String,
    pub field_name: String,
    pub offset_from_end: isize,
    pub kind: SigKind,
    pub continuation_params: Vec<SigKind>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct MirInstruction {
    pub kind: MirInstKind,
    pub opcode: &'static str,
    pub operand_comments: (&'static str, &'static str, &'static str),
    pub inputs: Vec<MirArg>,
    pub outputs: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum MirCallTarget {
    Builtin(MirCallKind),
    Field(MirField),
}

#[derive(Clone, Debug)]
pub struct MirCall {
    pub result: String,
    pub target: MirCallTarget,
    pub args: Vec<MirArg>,
    pub arg_kinds: Vec<SigKind>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct MirSysCall {
    pub kind: MirSysCallKind,
    pub operand_comments: (&'static str, &'static str, &'static str),
    pub args: Vec<MirArg>,
    pub outputs: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct MirExec {
    pub target: MirExecTarget,
    pub args: Vec<MirArg>,
    pub span: Span,
}

// TODO: ABC: This needs adapting and fixing.
#[derive(Clone, Debug)]
pub struct MirClosure {
    pub name: String,
    pub target: MirExecTarget,
    pub args: Vec<MirArg>,
    pub env_layout: Vec<SigKind>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MirArg {
    pub name: String,
    pub kind: SigKind,
}
