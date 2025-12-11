use crate::compiler::error::{CompileError, CompileErrorCode};
use crate::compiler::hir::{SigItem, SigKind, Signature};
use crate::compiler::span::Span;
use std::collections::HashMap;

#[derive(Clone)]
pub struct CaptureParam {
    pub name: String,
    pub ty: SigItem,
    pub span: Span,
}

pub struct Context {
    pub counter: usize,                       // counter for new_name
    pub ns: String,                           // namespace string (e.g. "_foo_foo")
    pub outer: HashMap<String, ContextEntry>, // inherited
    pub inner: HashMap<String, ContextEntry>, // current scope
    pub captures: HashMap<String, Vec<CaptureParam>>,
}

impl Context {
    pub fn new() -> Self {
        Context {
            counter: 0,
            ns: String::new(),
            outer: HashMap::new(),
            inner: HashMap::new(),
            captures: HashMap::new(),
        }
    }

    pub fn enter(&self, name: &str) -> Context {
        // build new namespace
        let new_ns = if self.ns.is_empty() {
            format!("_{}", name)
        } else {
            format!("{}_{}", self.ns, name)
        };

        // merge inner + outer → new outer
        let mut new_outer = self.outer.clone();
        for (k, v) in &self.inner {
            new_outer.insert(k.clone(), v.clone());
        }

        Context {
            counter: 0,
            ns: new_ns,
            outer: new_outer,
            inner: HashMap::new(),
            captures: self.captures.clone(),
        }
    }

    pub fn insert(&mut self, name: &str, entry: ContextEntry) -> Result<(), CompileError> {
        if self.inner.contains_key(name) {
            return Err(CompileError::new(
                CompileErrorCode::Resolve,
                format!("duplicate symbol `{}` in this scope", name),
                entry.span(),
            ));
        }
        self.inner.insert(name.to_string(), entry);
        Ok(())
    }

    pub fn insert_type(
        &mut self,
        name: &str,
        ty: SigKind,
        span: Span,
        is_signature: bool,
    ) -> Result<(), CompileError> {
        self.insert(
            name,
            ContextEntry::Type {
                ty,
                span,
                is_signature,
            },
        )
    }

    pub fn insert_func(
        &mut self,
        name: &str,
        sig: Signature,
        span: Span,
        is_signature: bool,
    ) -> Result<(), CompileError> {
        self.insert_type(name, SigKind::Tuple(sig), span, is_signature)
    }

    pub fn get(&self, name: &str) -> Option<&ContextEntry> {
        self.inner.get(name).or_else(|| self.outer.get(name))
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut ContextEntry> {
        if let Some(entry) = self.inner.get_mut(name) {
            Some(entry)
        } else {
            self.outer.get_mut(name)
        }
    }

    pub fn new_name_for(&mut self, name: &str) -> String {
        let n = self.counter;
        self.counter += 1;
        format!("{}_{}_{}", self.ns, name, n)
    }
    pub fn new_name(&mut self) -> String {
        let n = self.counter;
        self.counter += 1;
        format!("{}_{}", self.ns, n)
    }

    pub fn register_function_with_captures(
        &mut self,
        name: &str,
        captures: &[SigItem],
    ) -> Result<(), CompileError> {
        if captures.is_empty() {
            return Ok(());
        }

        let ctx_entry = self
            .inner
            .get_mut(name)
            .or_else(|| self.outer.get_mut(name));

        let ctx_entry = match ctx_entry {
            Some(entry) => entry,
            None => {
                return Err(CompileError::new(
                    CompileErrorCode::Resolve,
                    format!("function '{}' not found in scope for captures", name),
                    Span::unknown(),
                ));
            }
        };

        match ctx_entry {
            ContextEntry::Type { ty, .. } => match ty {
                SigKind::Tuple(signature) => {
                    signature.items.splice(0..0, captures.iter().cloned());
                    Ok(())
                }
                _ => Err(CompileError::new(
                    CompileErrorCode::Resolve,
                    format!("'{}' is not a function", name),
                    Span::unknown(),
                )),
            },
            _ => Err(CompileError::new(
                CompileErrorCode::Resolve,
                format!("'{}' is not a function", name),
                Span::unknown(),
            )),
        }
    }

    pub fn record_function_captures(&mut self, name: &str, captures: Vec<CaptureParam>) {
        if captures.is_empty() {
            return;
        }
        self.captures.insert(name.to_string(), captures);
    }

    pub fn function_captures(&self, name: &str) -> Option<&[CaptureParam]> {
        self.captures.get(name).map(|vec| vec.as_slice())
    }
}

#[derive(Clone, Debug)]
pub enum ConstantValue {
    Str(String),
    Int(i64),
}

#[derive(Clone)]
pub enum ContextEntry {
    // TODO: Doesn't need to be an enum, can simply be a struct with an optional constant value
    Type {
        ty: SigKind, // e.g. int, str, foo<T>, (int, str)
        span: Span,
        is_signature: bool, // TODO: Remove this later.
    },

    Value {
        ty: SigKind,
        constant: ConstantValue, // TODO: is this even useful?
        span: Span,
    },
}

impl ContextEntry {
    pub fn span(&self) -> Span {
        match self {
            ContextEntry::Type { span, .. } => *span,
            ContextEntry::Value { span, .. } => *span,
        }
    }
}
