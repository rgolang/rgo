use std::collections::HashMap;

use crate::compiler::air::{self, FunctionSig, SigKind};
use crate::compiler::builtins;
use crate::compiler::error::{Code, Error};
use crate::compiler::span::Span;
use crate::last_slug;

#[derive(Debug, Default)]
pub struct SymbolRegistry {
    functions: HashMap<String, air::FunctionSig>,
    types: HashMap<String, SigKind>,
    values: HashMap<String, SigKind>,
    pub builtin_imports: HashMap<String, String>,
}

impl SymbolRegistry {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            types: HashMap::new(),
            values: HashMap::new(),
            builtin_imports: HashMap::new(),
        }
    }

    pub fn record_builtin_import(&mut self, alias: &str, builtin_name: &str) {
        self.builtin_imports
            .insert(alias.to_string(), builtin_name.to_string());
    }

    pub fn builtin_name_for_alias(&self, alias: &str) -> Option<&String> {
        self.builtin_imports.get(alias)
    }

    pub fn declare_function(&mut self, sig: air::FunctionSig) -> Result<(), Error> {
        self.functions.insert(sig.name.clone(), sig);
        Ok(())
    }

    pub fn install_type(&mut self, name: String, kind: SigKind) -> Result<(), Error> {
        self.types.insert(name.clone(), kind.clone());
        Ok(())
    }

    pub fn get_type_info(&self, name: &str) -> Option<&SigKind> {
        self.types.get(name)
    }

    pub fn get_value(&self, name: &str) -> Option<&SigKind> {
        self.values.get(name)
    }

    pub fn get_function(&self, name: &str) -> Option<&air::FunctionSig> {
        self.functions.get(name)
    }
}

pub fn register_builtin_import(
    alias: &str,
    import_path: &str,
    span: Span,
    symbols: &mut SymbolRegistry,
) -> Result<(), Error> {
    let name = last_slug(import_path);
    symbols.record_builtin_import(alias, name);

    let spec = builtins::get_spec(name, span).ok_or_else(|| {
        Error::new(
            Code::Internal,
            format!("unknown import '@{}'", import_path),
            span,
        )
    })?;

    if let builtins::BuiltinSpec::Type(kind) = spec {
        symbols.install_type(alias.to_string(), kind.clone())?;
    }

    Ok(())
}

pub fn builtin_function_sig(builtin_name: &str, span: Span) -> Result<FunctionSig, Error> {
    let spec = builtins::get_spec(builtin_name, span).ok_or_else(|| {
        Error::new(
            Code::Internal,
            format!("unknown import '@{}'", builtin_name),
            span,
        )
    })?;

    if let builtins::BuiltinSpec::Function(sig) = spec {
        let builtin = builtins::Builtin::from_name(builtin_name).ok_or_else(|| {
            Error::new(
                Code::Internal,
                format!("unknown builtin '{}'", builtin_name),
                span,
            )
        })?;
        return Ok(FunctionSig {
            name: builtin_name.into(),
            params: sig.items,
            span,
            builtin: Some(builtin),
        });
    }

    Err(Error::new(
        Code::Internal,
        format!("'{}' is not a callable builtin", builtin_name),
        span,
    ))
}
