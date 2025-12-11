use std::collections::{HashMap, HashSet};

use crate::compiler::ast;
use crate::compiler::error::{CompileError, CompileErrorCode};
use crate::compiler::span::Span;

#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub name: String,
    pub params: Vec<ast::SigItem>,
    pub span: Span,
}

impl FunctionSig {
    pub fn param_kinds(&self) -> Vec<ast::SigKind> {
        self.params
            .iter()
            .map(|item| item.ty.kind.clone())
            .collect()
    }

    pub fn is_variadic(&self) -> bool {
        self.params.iter().any(|item| item.is_variadic)
    }

    pub fn variadic_flags(&self) -> Vec<bool> {
        self.params.iter().map(|item| item.is_variadic).collect()
    }
}

#[derive(Debug, Clone)]
pub struct CaptureParam {
    pub name: String,
    pub ty: ast::SigItem,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub target: ast::SigKind,
    pub span: Span,
    pub variadic: Vec<bool>,
    pub generics: Vec<String>,
}

#[derive(Debug, Default)]
pub struct SymbolRegistry {
    lambda_counter: usize,
    temp_counter: usize,
    type_counter: usize,
    named_types: HashMap<ast::SigKind, String>,
    functions: HashMap<String, FunctionSig>,
    types: HashMap<String, TypeInfo>,
    values: HashMap<String, ast::SigKind>,
    builtin_imports: HashSet<String>,
    captures: HashMap<String, Vec<CaptureParam>>,
    type_variadics: HashMap<ast::SigKind, Vec<bool>>,
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

    pub fn register_function_with_captures(&mut self, name: String, captures: Vec<CaptureParam>) {
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

    pub fn declare_function(&mut self, sig: FunctionSig) -> Result<(), CompileError> {
        // if sig.name == "myexit" {
        //     panic!("cannot declare a function with name 'myexit'");
        // }
        self.functions.insert(sig.name.clone(), sig);
        Ok(())
    }

    pub fn update_function_signature(
        &mut self,
        name: &str,
        params: Vec<ast::SigItem>,
    ) -> Result<(), CompileError> {
        if let Some(sig) = self.functions.get_mut(name) {
            sig.params = params;
            Ok(())
        } else {
            Err(CompileError::new(
                CompileErrorCode::Parse,
                format!("function '{}' not declared", name),
                Span::unknown(),
            ))
        }
    }

    pub fn install_type(
        &mut self,
        name: String,
        term: ast::SigKind,
        span: Span,
        variadic: Vec<bool>,
        generics: Vec<String>,
    ) -> Result<(), CompileError> {
        self.types.insert(
            name.clone(),
            TypeInfo {
                name: name.clone(),
                target: term.clone(),
                span: span,
                variadic: variadic.clone(),
                generics: generics.clone(),
            },
        );
        self.record_type_variadic(term.clone(), variadic.clone());
        self.record_type_variadic(
            ast::SigKind::Ident(ast::SigIdent {
                name: name.clone(),
                has_bang: false,
                span,
            }),
            variadic,
        );
        Ok(())
    }

    pub fn declare_value(
        &mut self,
        name: String,
        ty: ast::SigKind,
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

    pub fn fresh_type_name(&mut self) -> String {
        let name = format!("__type_{}", self.type_counter);
        self.type_counter += 1;
        name
    }

    pub fn normalize_top_level_type(&mut self, ty: ast::SigKind) -> ast::SigKind {
        match ty {
            ast::SigKind::Tuple(sig) => {
                let new_items = sig
                    .items
                    .into_iter()
                    .map(|item| {
                        let normalized_kind = self.normalize_nested_type(item.ty.kind);

                        ast::SigItem {
                            name: item.name,
                            ty: ast::SigType {
                                kind: normalized_kind,
                                span: item.ty.span,
                            },
                            is_variadic: item.is_variadic,
                            span: item.span,
                        }
                    })
                    .collect();

                ast::SigKind::Tuple(ast::Signature {
                    items: new_items,
                    span: sig.span,
                })
            }

            other => other,
        }
    }

    fn normalize_nested_type(&mut self, ty: ast::SigKind) -> ast::SigKind {
        match ty {
            ast::SigKind::Tuple(sig) => {
                // Normalize each SigItem's type recursively
                let mut new_items = Vec::with_capacity(sig.items.len());

                for item in sig.items {
                    let normalized_kind = self.normalize_nested_type(item.ty.kind);

                    new_items.push(ast::SigItem {
                        name: item.name,
                        ty: ast::SigType {
                            kind: normalized_kind,
                            span: item.ty.span,
                        },
                        is_variadic: item.is_variadic,
                        span: item.span,
                    });
                }

                let normalized = ast::SigKind::Tuple(ast::Signature {
                    items: new_items,
                    span: sig.span,
                });

                // If it still contains generics → do NOT alias it.
                if self.type_contains_generic(&normalized, &mut HashSet::new()) {
                    return normalized;
                }

                // Check if we already assigned a name to this normalized tuple
                if let Some(name) = self.named_types.get(&normalized) {
                    return ast::SigKind::Ident(ast::SigIdent {
                        name: name.clone(),
                        has_bang: false,
                        span: Span::unknown(),
                    });
                }

                // Create a fresh alias name for the tuple
                let alias_name = self.fresh_type_name();

                // Register alias → underlying tuple
                let variadic = self
                    .get_type_variadic(&normalized)
                    .cloned()
                    .unwrap_or_default();

                self.named_types
                    .insert(normalized.clone(), alias_name.clone());

                self.install_type(
                    alias_name.clone(),
                    normalized.clone(),
                    Span::unknown(), // TODO
                    variadic,
                    Vec::new(), // no generic params
                )
                .expect("failed to register normalized type");

                // Return the alias as a SigKind::Ident
                ast::SigKind::Ident(ast::SigIdent {
                    name: alias_name,
                    has_bang: false,
                    span: Span::unknown(),
                })
            }

            // non-tuple → identity
            other => other,
        }
    }
    fn type_contains_generic(&self, ty: &ast::SigKind, visited: &mut HashSet<String>) -> bool {
        match ty {
            // --- Generic parameter directly present ---
            ast::SigKind::Generic(_) => true,

            // --- Tuple signature: recursively check each SigItem ---
            ast::SigKind::Tuple(sig) => sig
                .items
                .iter()
                .any(|item| self.type_contains_generic(&item.ty.kind, visited)),

            // --- Generic instantiation: Foo<T, X> ---
            ast::SigKind::GenericInst { args, .. } => args
                .iter()
                .any(|arg| self.type_contains_generic(&arg.kind, visited)),

            // --- Identifier: resolve alias and check its target ---
            ast::SigKind::Ident(ident) => {
                let name = &ident.name;

                if visited.contains(name) {
                    return false;
                }

                if let Some(info) = self.types.get(name) {
                    visited.insert(name.clone());
                    let result = self.type_contains_generic(&info.target, visited);
                    visited.remove(name);
                    return result;
                }

                false
            }

            // --- All other kinds ---
            _ => false,
        }
    }

    pub fn update_type(
        &mut self,
        name: &str,
        target: ast::SigKind,
        variadic: Vec<bool>,
    ) -> Result<(), CompileError> {
        if let Some(info) = self.types.get_mut(name) {
            let target_clone = target.clone();
            info.target = target;
            info.variadic = variadic.clone();
            self.record_type_variadic(target_clone, variadic.clone());
            self.record_type_variadic(
                ast::SigKind::Ident(ast::SigIdent {
                    name: name.to_string(),
                    has_bang: false,
                    span: Span::unknown(),
                }),
                variadic,
            );
            Ok(())
        } else {
            Err(CompileError::new(
                CompileErrorCode::Parse,
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

    pub fn get_value(&self, name: &str) -> Option<&ast::SigKind> {
        self.values.get(name)
    }

    pub fn record_type_variadic(&mut self, ty: ast::SigKind, variadic: Vec<bool>) {
        self.type_variadics.insert(ty, variadic);
    }

    pub fn get_type_variadic(&self, ty: &ast::SigKind) -> Option<&Vec<bool>> {
        let mut visited = HashSet::new();
        self.get_type_variadic_inner(ty, &mut visited)
    }

    fn get_type_variadic_inner(
        &self,
        ty: &ast::SigKind,
        visited: &mut HashSet<String>,
    ) -> Option<&Vec<bool>> {
        if let Some(flags) = self.type_variadics.get(ty) {
            return Some(flags);
        }
        if let ast::SigKind::Ident(ident) = ty {
            let name = &ident.name;
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
        } else if let ast::SigKind::GenericInst { name, .. } = ty {
            if visited.contains(name) {
                return None;
            }
            if let Some(info) = self.types.get(name) {
                return if !info.variadic.is_empty() {
                    Some(&info.variadic)
                } else {
                    None
                };
            }
        }
        None
    }

    pub fn resolve_type(&self, name: &str) -> Option<ast::SigKind> {
        if name == "Int" || name == "int" {
            return Some(ast::SigKind::Int);
        }
        if name == "Str" || name == "str" {
            return Some(ast::SigKind::Str);
        }
        if let Some(info) = self.types.get(name) {
            return Some(ast::SigKind::Ident(ast::SigIdent {
                name: name.to_string(),
                has_bang: false,
                span: info.span,
            }));
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
