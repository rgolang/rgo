pub use crate::compiler::ast;
pub use crate::compiler::ast::Arg;
pub use crate::compiler::ast::{SigItem, SigKind};
use crate::compiler::builtins;
use crate::compiler::span::Span;

#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub name: String, // TODO: Maybe make it &str?
    pub params: Vec<SigItem>,
    pub span: Span,
    pub builtin: Option<builtins::Builtin>,
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
pub enum AirExecTarget {
    Function(FunctionSig),
    Closure { name: String },
}

#[derive(Clone)]
pub struct AirFunction {
    pub sig: FunctionSig,
    pub items: Vec<AirStmt>,
}

impl AirFunction {
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
pub struct AirReleaseHeap {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirLabel {
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct AirJump {
    pub target: String,
}

#[derive(Clone, Debug)]
pub enum AirValue {
    Binding(String),
    Literal(i64),
}

#[derive(Clone, Debug)]
pub struct AirReturn {
    pub value: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AirPin {
    pub result: String,
    pub value: AirValue,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirJumpEq {
    pub args: Vec<AirArg>,
    pub target: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirJumpLt {
    pub left: AirValue,
    pub right: AirValue,
    pub target: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirSetField {
    pub env_end: String,
    pub offset: isize,
    pub value: AirArg,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirJumpClosure {
    pub env_end: String,
    pub args: Vec<AirArg>,
    pub span: Span,
}

#[derive(Clone)]
pub enum AirStmt {
    Op(AirOp),
    Label(AirLabel),
}

#[derive(Clone, Debug)]
pub enum AirOp {
    Return(AirReturn),

    Jump(AirJump),
    JumpArgs(AirJumpArgs),
    JumpClosure(AirJumpClosure),
    JumpEqInt(AirJumpEq),
    JumpEqStr(AirJumpEq),
    JumpLt(AirJumpLt),
    JumpGt(AirJumpGt),

    Add(AirAdd),
    Sub(AirSub),
    Mul(AirMul),
    Div(AirDiv),

    SysExit(AirSysExit),

    Printf(AirPrintf),
    Sprintf(AirSprintf),
    Write(AirWrite),
    Puts(AirPuts),

    CallPtr(AirCallPtr),
    NewClosure(AirNewClosure),
    CloneClosure(AirCloneClosure),
    ReleaseHeap(AirReleaseHeap),
    Pin(AirPin),
    Field(AirField),
    CopyField(AirField),
    SetField(AirSetField),
}

#[derive(Clone, Debug)]
pub struct AirCloneClosure {
    pub src: String,
    pub dst: String,
    pub remaining: Vec<SigKind>, // TODO: Why does it need this?
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirField {
    pub result: String,
    pub ptr: String,
    pub offset: isize,
    pub kind: SigKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirAdd {
    pub inputs: Vec<AirArg>,
    pub target: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirSub {
    pub inputs: Vec<AirArg>,
    pub target: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirMul {
    pub inputs: Vec<AirArg>,
    pub target: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirDiv {
    pub inputs: Vec<AirArg>,
    pub target: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirJumpGt {
    pub left: AirValue,
    pub right: AirValue,
    pub target: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirPrintf {
    pub args: Vec<AirArg>,
    pub arg_kinds: Vec<SigKind>,
    pub target: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirSprintf {
    pub args: Vec<AirArg>,
    pub arg_kinds: Vec<SigKind>,
    pub target: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirWrite {
    pub args: Vec<AirArg>,
    pub arg_kinds: Vec<SigKind>,
    pub target: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirPuts {
    pub args: Vec<AirArg>,
    pub arg_kinds: Vec<SigKind>,
    pub target: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirSysExit {
    pub args: Vec<AirArg>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum AirCallPtrTarget {
    Binding(String),
}

#[derive(Clone, Debug)]
pub struct AirCallPtr {
    pub target: AirCallPtrTarget,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AirJumpArgs {
    pub target: FunctionSig,
    pub args: Vec<AirArg>,
    pub span: Span,
}

// TODO: ABC: This needs adapting and fixing.
#[derive(Clone, Debug)]
pub struct AirNewClosure {
    pub name: String,
    pub target: FunctionSig,
    pub args: Vec<AirArg>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AirArg {
    pub name: String,
    pub kind: SigKind,
    pub literal: Option<ast::Lit>,
}
