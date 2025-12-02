use std::collections::{HashMap, HashSet};

use crate::compiler::ast::TypeRef;
use crate::compiler::error::ParseError;
use crate::compiler::span::Span;

#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub name: String,
    pub params: Vec<TypeRef>,
    pub is_variadic: Vec<bool>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CaptureParam {
    pub name: String,
    pub ty: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub target: TypeRef,
    pub span: Span,
    pub variadic: Vec<bool>,
}

#[derive(Debug, Default)]
pub struct SymbolRegistry {
    lambda_counter: usize,
    temp_counter: usize,
    type_counter: usize,
    named_types: HashMap<TypeRef, String>,
    functions: HashMap<String, FunctionSig>,
    types: HashMap<String, TypeInfo>,
    values: HashMap<String, TypeRef>,
    builtin_imports: HashSet<String>,
    captures: HashMap<String, Vec<CaptureParam>>,
    type_variadics: HashMap<TypeRef, Vec<bool>>,
}

impl SymbolRegistry {
    pub fn new() -> Self {
        Self {
            lambda_counter: 0,
            temp_counter: 0,
            type_counter: 0,
            named_types: HashMap::new(),
            functions: HashMap::new(),
            types: HashMap::new(),
            values: HashMap::new(),
            builtin_imports: HashSet::new(),
            captures: HashMap::new(),
            type_variadics: HashMap::new(),
        }
    }

    pub fn register_function_capture(&mut self, name: String, captures: Vec<CaptureParam>) {
        if let Some(sig) = self.functions.get_mut(&name) {
            let mut new_params = captures
                .iter()
                .map(|cap| cap.ty.clone())
                .collect::<Vec<_>>();
            new_params.extend(sig.params.clone());
            sig.params = new_params;
        }
        if !captures.is_empty() {
            self.captures.insert(name, captures);
        }
    }

    pub fn function_captures(&self, name: &str) -> Option<&[CaptureParam]> {
        self.captures.get(name).map(|captures| captures.as_slice())
    }

    pub fn record_builtin_import(&mut self, name: &str) {
        self.builtin_imports.insert(name.to_string());
    }

    pub fn builtin_imports(&self) -> &HashSet<String> {
        &self.builtin_imports
    }

    pub fn declare_function(&mut self, sig: FunctionSig) -> Result<(), ParseError> {
        if self.functions.contains_key(&sig.name) {
            return Err(ParseError::new(
                format!("function '{}' already declared", sig.name),
                sig.span,
            ));
        }
        self.functions.insert(sig.name.clone(), sig);
        Ok(())
    }

    pub fn update_function_signature(
        &mut self,
        name: &str,
        params: Vec<TypeRef>,
        is_variadic: Vec<bool>,
    ) -> Result<(), ParseError> {
        if let Some(sig) = self.functions.get_mut(name) {
            sig.params = params;
            sig.is_variadic = is_variadic;
            Ok(())
        } else {
            Err(ParseError::new(
                format!("function '{}' not declared", name),
                Span::unknown(),
            ))
        }
    }

    pub fn install_type(
        &mut self,
        name: String,
        term: TypeRef,
        span: Span,
        variadic: Vec<bool>,
    ) -> Result<(), ParseError> {
        if self.types.contains_key(&name) {
            return Err(ParseError::new(
                format!("type '{}' already declared", name),
                span,
            ));
        }
        self.types.insert(
            name.clone(),
            TypeInfo {
                name: name.clone(),
                target: term.clone(),
                span: span,
                variadic: variadic.clone(),
            },
        );
        self.record_type_variadic(term.clone(), variadic.clone());
        self.record_type_variadic(TypeRef::Alias(name.clone()), variadic);
        Ok(())
    }

    pub fn declare_value(
        &mut self,
        name: String,
        ty: TypeRef,
        span: Span,
    ) -> Result<(), ParseError> {
        if self.values.contains_key(&name)
            || self.functions.contains_key(&name)
            || self.types.contains_key(&name)
        {
            return Err(ParseError::new(
                format!("symbol '{}' already declared", name),
                span,
            ));
        }
        self.values.insert(name, ty);
        Ok(())
    }

    pub fn fresh_type_name(&mut self) -> String {
        let name = format!("__type_{}", self.type_counter);
        self.type_counter += 1;
        name
    }

    pub fn normalize_top_level_type(&mut self, ty: TypeRef) -> TypeRef {
        match ty {
            TypeRef::Type(params) => TypeRef::Type(
                params
                    .into_iter()
                    .map(|param| self.normalize_nested_type(param))
                    .collect(),
            ),
            other => other,
        }
    }

    fn normalize_nested_type(&mut self, ty: TypeRef) -> TypeRef {
        match ty {
            TypeRef::Type(params) => {
                let normalized_params = params
                    .into_iter()
                    .map(|param| self.normalize_nested_type(param))
                    .collect::<Vec<_>>();
                let normalized = TypeRef::Type(normalized_params);
                if let Some(name) = self.named_types.get(&normalized) {
                    return TypeRef::Alias(name.clone());
                }
                let alias_name = self.fresh_type_name();
                self.named_types
                    .insert(normalized.clone(), alias_name.clone());
                let variadic = self
                    .get_type_variadic(&normalized)
                    .cloned()
                    .unwrap_or_default();
                self.install_type(
                    alias_name.clone(),
                    normalized.clone(),
                    Span::unknown(),
                    variadic,
                )
                .expect("failed to register normalized type");
                TypeRef::Alias(alias_name)
            }
            other => other,
        }
    }

    pub fn update_type(
        &mut self,
        name: &str,
        target: TypeRef,
        variadic: Vec<bool>,
    ) -> Result<(), ParseError> {
        if let Some(info) = self.types.get_mut(name) {
            let target_clone = target.clone();
            info.target = target;
            info.variadic = variadic.clone();
            self.record_type_variadic(target_clone, variadic.clone());
            self.record_type_variadic(TypeRef::Alias(name.to_string()), variadic);
            Ok(())
        } else {
            Err(ParseError::new(
                format!("type '{}' not declared", name),
                Span::unknown(),
            ))
        }
    }

    pub fn remove_type(&mut self, name: &str) {
        self.types.remove(name);
    }

    pub fn get_type_info(&self, name: &str) -> Option<&TypeInfo> {
        self.types.get(name)
    }

    pub fn get_type_info_mut(&mut self, name: &str) -> Option<&mut TypeInfo> {
        self.types.get_mut(name)
    }

    pub fn get_value(&self, name: &str) -> Option<&TypeRef> {
        self.values.get(name)
    }

    pub fn record_type_variadic(&mut self, ty: TypeRef, variadic: Vec<bool>) {
        self.type_variadics.insert(ty, variadic);
    }

    pub fn get_type_variadic(&self, ty: &TypeRef) -> Option<&Vec<bool>> {
        let mut visited = HashSet::new();
        self.get_type_variadic_inner(ty, &mut visited)
    }

    fn get_type_variadic_inner(
        &self,
        ty: &TypeRef,
        visited: &mut HashSet<String>,
    ) -> Option<&Vec<bool>> {
        if let Some(flags) = self.type_variadics.get(ty) {
            return Some(flags);
        }
        if let TypeRef::Alias(name) = ty {
            if visited.contains(name) {
                return None;
            }
            if let Some(info) = self.types.get(name) {
                if !info.variadic.is_empty() {
                    return Some(&info.variadic);
                }
                visited.insert(name.clone());
                let result = self.get_type_variadic_inner(&info.target, visited);
                visited.remove(name);
                return result;
            }
        }
        None
    }

    pub fn resolve_type(&self, name: &str) -> Option<TypeRef> {
        if name == "Int" || name == "int" {
            return Some(TypeRef::Int);
        }
        if name == "Str" || name == "str" {
            return Some(TypeRef::Str);
        }
        if self.types.contains_key(name) {
            return Some(TypeRef::Alias(name.to_string()));
        }
        None
    }

    pub fn get_function(&self, name: &str) -> Option<&FunctionSig> {
        self.functions.get(name)
    }

    pub fn type_exists(&self, name: &str) -> bool {
        name == "Int"
            || name == "int"
            || name == "Str"
            || name == "str"
            || self.types.contains_key(name)
    }

    pub fn fresh_lambda_name(&mut self, _span: Span) -> String {
        let name = format!("__lambda_{}", self.lambda_counter);
        self.lambda_counter += 1;
        name
    }

    pub fn fresh_temp_name(&mut self) -> String {
        let name = format!("__temp_{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }
}
