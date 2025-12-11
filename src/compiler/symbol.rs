use std::collections::{HashMap, HashSet};

use crate::compiler::error::{CompileError, CompileErrorCode};
use crate::compiler::mir::{SigItem, SigKind};
use crate::compiler::span::Span;

#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub name: String,
    pub params: Vec<SigItem>,
    pub span: Span,
}

impl FunctionSig {
    pub fn param_kinds(&self) -> Vec<SigKind> {
        self.params
            .iter()
            .map(|item| item.ty.kind.clone())
            .collect()
    }

    pub fn is_variadic(&self) -> bool {
        self.params.iter().any(|item| item.is_variadic)
    }
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub target: SigKind,
    pub span: Span,
    pub variadic: Vec<bool>,
}

#[derive(Debug, Default)]
pub struct SymbolRegistry {
    functions: HashMap<String, FunctionSig>,
    types: HashMap<String, TypeInfo>,
    values: HashMap<String, SigKind>,
    builtin_imports: HashSet<String>,
}

impl SymbolRegistry {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            types: HashMap::new(),
            values: HashMap::new(),
            builtin_imports: HashSet::new(),
        }
    }

    pub fn record_builtin_import(&mut self, name: &str) {
        self.builtin_imports.insert(name.to_string());
    }

    pub fn builtin_imports(&self) -> &HashSet<String> {
        &self.builtin_imports
    }

    pub fn declare_function(&mut self, sig: FunctionSig) -> Result<(), CompileError> {
        self.functions.insert(sig.name.clone(), sig);
        Ok(())
    }

    pub fn install_type(
        &mut self,
        name: String,
        term: SigKind,
        span: Span,
        variadic: Vec<bool>,
    ) -> Result<(), CompileError> {
        self.types.insert(
            name.clone(),
            TypeInfo {
                name: name.clone(),
                target: term.clone(),
                span: span,
                variadic: variadic.clone(),
            },
        );
        Ok(())
    }

    pub fn declare_value(
        &mut self,
        name: String,
        ty: SigKind,
        span: Span,
    ) -> Result<(), CompileError> {
        if self.values.contains_key(&name)
            || self.functions.contains_key(&name)
            || self.types.contains_key(&name)
        {
            return Err(CompileError::new(
                CompileErrorCode::Parse,
                format!("symbol '{}' already declared", name),
                span,
            ));
        }
        self.values.insert(name, ty);
        Ok(())
    }

    pub fn get_type_info(&self, name: &str) -> Option<&TypeInfo> {
        self.types.get(name)
    }

    pub fn get_value(&self, name: &str) -> Option<&SigKind> {
        self.values.get(name)
    }

    pub fn get_function(&self, name: &str) -> Option<&FunctionSig> {
        self.functions.get(name)
    }
}
