use std::collections::{HashMap, HashSet};

use crate::compiler::builtins;
use crate::compiler::error::{Code, Error};
use crate::compiler::mir::{self, FunctionSig, SigKind};
use crate::compiler::span::Span;
use crate::last_slug;

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub target: SigKind,
    pub span: Span,
}

#[derive(Debug, Default)]
pub struct SymbolRegistry {
    functions: HashMap<String, mir::FunctionSig>,
    types: HashMap<String, TypeInfo>,
    values: HashMap<String, SigKind>,
    builtin_imports: HashSet<String>,
    builtin_function_names: HashSet<String>,
}

impl SymbolRegistry {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            types: HashMap::new(),
            values: HashMap::new(),
            builtin_imports: HashSet::new(),
            builtin_function_names: HashSet::new(),
        }
    }

    pub fn record_builtin_import(&mut self, name: &str) {
        self.builtin_imports.insert(name.to_string());
    }

    pub fn builtin_imports(&self) -> &HashSet<String> {
        &self.builtin_imports
    }

    pub fn builtin_function_generated(&self, name: &str) -> bool {
        self.builtin_function_names.contains(name)
    }

    pub fn mark_builtin_function_generated(&mut self, name: &str) {
        self.builtin_function_names.insert(name.to_string());
    }

    pub fn declare_function(&mut self, sig: mir::FunctionSig) -> Result<(), Error> {
        self.functions.insert(sig.name.clone(), sig);
        Ok(())
    }

    pub fn install_type(&mut self, name: String, term: SigKind, span: Span) -> Result<(), Error> {
        self.types.insert(
            name.clone(),
            TypeInfo {
                name: name.clone(),
                target: term.clone(),
                span: span,
            },
        );
        Ok(())
    }

    pub fn get_type_info(&self, name: &str) -> Option<&TypeInfo> {
        self.types.get(name)
    }

    pub fn get_value(&self, name: &str) -> Option<&SigKind> {
        self.values.get(name)
    }

    pub fn get_function(&self, name: &str) -> Option<&mir::FunctionSig> {
        self.functions.get(name)
    }
}

pub fn register_builtin_import<'a>(
    import_path: &'a str,
    span: Span,
    symbols: &mut SymbolRegistry,
) -> Result<&'a str, Error> {
    let name = last_slug(import_path);
    symbols.record_builtin_import(&name);

    let spec = builtins::get_spec(name, span).ok_or_else(|| {
        Error::new(
            Code::Internal,
            format!("unknown import '@{}'", import_path),
            span,
        )
    })?;

    match spec {
        builtins::BuiltinSpec::Function(sig) => {
            let func_sig = FunctionSig {
                name: name.into(),
                params: sig.items,
                span,
                builtin: builtins::get_builtin_kind(name),
            };
            symbols.declare_function(func_sig)?;
        }
        builtins::BuiltinSpec::Type(kind) => {
            symbols.install_type(name.to_string(), kind.clone(), span)?;
        }
    }

    Ok(name)
}
