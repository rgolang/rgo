use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct Context {
    counter: usize,
    // types: HashMap<hir::SigKind, String>,
    functions: HashMap<String, FunctionInfo>,
}

#[derive(Clone, Debug)]
pub enum Type {
    Int,               // machine integer (size chosen by target)
    Str,               // pointer to runtime string object
    Struct(Vec<Type>), // product type (tuples, envs, closures and user structs)
    CodePtr,           // pointer to a function (represented by FunctionId)
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct FunctionInfo {
    signature: FnSignature, // types of the function's inputs & output
    env_type: Option<Type>, // None for normal functions, Some(Struct(...)) for closures
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct FnSignature {
    // TODO: Rename to FunctionInfoSig or something...
    params: Vec<Type>,
    ret: Type,
}
