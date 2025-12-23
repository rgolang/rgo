use crate::compiler::ast::{SigItem, SigKind, Signature};
use crate::compiler::builtins;
use crate::compiler::error::{Code, Error};
use crate::compiler::hir::Closure;
use crate::compiler::mir::ENTRY_FUNCTION_NAME;
use crate::compiler::span::Span;
use crate::last_slug;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Clone)]
pub struct ContextEntry {
    pub name: String,
    pub kind: SigKind, // e.g. int, str, foo<T>, (int, str)
    pub is_builtin: bool,
    pub is_root: bool,
    pub is_param: bool,
    pub is_capture: bool,
    pub span: Span,
    pub scope: Vec<String>,
    pub captures: Vec<SigItem>,
}

pub struct Context {
    // TODO: this can be the context entry too really, then can better handle capture pushing to current context
    pub name: String,
    pub is_root_fn: bool,
    pub counter: Rc<Cell<usize>>, // counter for new_name
    pub ns: String,               // namespace string (e.g. "_foo_foo")
    scope_stack: Vec<String>,
    pub outer: HashMap<String, ContextEntry>, // inherited
    pub inner: HashMap<String, ContextEntry>, // current scope
    params: Vec<String>,
    pub closure_defs: HashMap<String, Closure>,
    pub emitted_closures: HashSet<String>,
}

impl Context {
    pub fn new() -> Self {
        Context {
            name: ENTRY_FUNCTION_NAME.to_string(),
            is_root_fn: true,
            counter: Rc::new(Cell::new(0)),
            ns: String::new(),
            scope_stack: Vec::new(),
            outer: HashMap::new(),
            inner: HashMap::new(),
            params: Vec::new(),
            closure_defs: HashMap::new(),
            emitted_closures: HashSet::new(),
        }
    }

    pub fn enter(&self, name: &str, display_ns: Option<&str>, is_root_fn: bool) -> Context {
        let new_ns = self.build_ns(display_ns);

        // merge inner + outer → new outer
        let mut new_outer = self.outer.clone();
        for (k, v) in &self.inner {
            let o = v.clone();
            new_outer.insert(k.clone(), o);
        }

        let mut new_scope_stack = self.scope_stack.clone();
        new_scope_stack.push(name.to_string());
        Context {
            name: name.to_string(),
            is_root_fn,
            counter: self.counter.clone(),
            ns: new_ns,
            scope_stack: new_scope_stack,
            outer: new_outer,
            inner: HashMap::new(),
            params: Vec::new(),
            closure_defs: HashMap::new(),
            emitted_closures: HashSet::new(),
        }
    }

    fn build_ns(&self, display_ns: Option<&str>) -> String {
        match display_ns {
            Some(ns) if !ns.is_empty() => {
                if self.ns.is_empty() {
                    ns.to_string()
                } else if self.ns == ns {
                    self.ns.clone()
                } else {
                    format!("{}_{}", self.ns, ns)
                }
            }
            _ => {
                if self.ns.is_empty() {
                    "lambda".to_string()
                } else {
                    self.ns.clone()
                }
            }
        }
    }

    pub fn is_scope_ancestor(&self, entry: &ContextEntry) -> bool {
        let ctx_scope = &self.scope_stack;
        if ctx_scope.len() <= 1 {
            return true;
        }
        let entry_scope = &entry.scope;
        if entry_scope.is_empty() || entry_scope.len() >= ctx_scope.len() {
            return false;
        }
        entry_scope
            .iter()
            .zip(ctx_scope.iter())
            .all(|(left, right)| left == right)
    }

    pub fn new_name_for_fn(&mut self, display_name: Option<&str>) -> String {
        if self.scope_stack.is_empty() {
            if let Some(name) = display_name {
                if !name.is_empty() {
                    return name.into();
                }
            }
        }
        let n = self.counter.get();
        self.counter.set(n + 1);
        let ns = self.build_ns(display_name);
        if ns.is_empty() {
            format!("_{}", n)
        } else {
            format!("_{}_{}", n, ns)
        }
    }

    pub fn new_name_for(&mut self, name: &str) -> String {
        let n = self.counter.get();
        self.counter.set(n + 1);
        format!("_{}_{}", n, name)
    }
    pub fn new_name_for_literal(&mut self) -> String {
        let n = self.counter.get();
        self.counter.set(n + 1);
        format!("_{}", n)
    }
    pub fn new_name(&mut self) -> String {
        let n = self.counter.get();
        self.counter.set(n + 1);
        format!("_{}_{}", n, self.ns)
    }

    pub fn add(&mut self, key: &str, entry: ContextEntry) -> Result<(), Error> {
        if self.inner.contains_key(key) {
            return Err(Error::new(
                Code::Resolve,
                format!("duplicate symbol `{}` in this scope", key),
                entry.span,
            ));
        }
        self.inner.insert(key.to_string(), entry);
        Ok(())
    }

    pub fn add_param(
        &mut self,
        name: &str,
        kind: SigKind,
        span: Span,
        is_capture: bool,
    ) -> Result<(), Error> {
        if self.inner.contains_key(name) {
            return Err(Error::new(
                Code::Resolve,
                format!("insert param, duplicate symbol `{}` in this scope", name),
                span,
            ));
        }
        self.params.push(name.to_string());
        self.inner.insert(
            name.to_string(),
            ContextEntry {
                name: name.to_string(),
                kind: kind.clone(),
                span,
                is_builtin: false,
                is_root: false,
                is_param: true,
                is_capture,
                scope: self.scope_stack.clone(),
                captures: Vec::new(),
            },
        );
        Ok(())
    }

    pub fn get_params(&self) -> Vec<SigItem> {
        let mut capture_params = Vec::new();
        let mut regular_params = Vec::new();
        for name in &self.params {
            if let Some(entry) = self.inner.get(name) {
                let item = SigItem {
                    name: name.clone(),
                    ty: entry.kind.clone(),
                    has_bang: false,
                    span: entry.span.clone(),
                };
                if entry.is_capture {
                    capture_params.push(item);
                } else {
                    regular_params.push(item);
                }
            }
        }
        capture_params.extend(regular_params);
        capture_params
    }

    pub fn get_captures(&self) -> Vec<SigItem> {
        let mut capture_params = Vec::new();
        for name in &self.params {
            if let Some(entry) = self.inner.get(name) {
                let item = SigItem {
                    name: name.clone(),
                    ty: entry.kind.clone(),
                    has_bang: false,
                    span: entry.span.clone(),
                };
                if entry.is_capture {
                    capture_params.push(item);
                }
            }
        }
        capture_params
    }

    pub fn add_literal(&mut self, name: &str, kind: SigKind, span: Span) -> Result<(), Error> {
        self.add(
            name,
            ContextEntry {
                name: name.to_string(),
                kind,
                span,
                is_builtin: false,
                is_root: false,
                is_param: false,
                is_capture: false,
                scope: self.scope_stack.clone(),
                captures: Vec::new(),
            },
        )
    }

    pub fn add_type(
        &mut self,
        key: &str,
        name: &str,
        kind: SigKind,
        span: Span,
        is_builtin: bool,
    ) -> Result<(), Error> {
        self.add(
            key,
            ContextEntry {
                name: name.to_string(),
                kind,
                span,
                is_builtin: is_builtin,
                is_root: true,
                is_param: false,
                is_capture: false,
                scope: self.scope_stack.clone(),
                captures: Vec::new(),
            },
        )
    }

    pub fn add_sig(
        &mut self,
        key: &str,
        name: &str,
        sig: Signature,
        span: Span,
        is_builtin: bool,
    ) -> Result<(), Error> {
        let mut entry_scope = self.scope_stack.clone();
        entry_scope.push(name.to_string());
        self.add(
            key,
            ContextEntry {
                name: name.to_string(),
                kind: SigKind::Sig(sig),
                span,
                is_builtin,
                is_root: true,
                is_param: false,
                is_capture: false,
                scope: entry_scope,
                captures: Vec::new(),
            },
        )
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

    // TODO: Should use the shared context namespace
    pub fn register_closure(&mut self, closure: Closure) {
        let name = closure.name.clone();
        let flattened: Closure = self.flatten_closure(&closure);
        if flattened.args.is_empty() {
            self.emitted_closures.insert(name.clone());
        }
        self.closure_defs.insert(name, flattened);
    }

    // TODO: ABC: can use regular context.
    fn flatten_closure(&self, closure: &Closure) -> Closure {
        let mut target = closure.of.clone();
        let mut args = closure.args.clone();
        let mut seen = HashSet::new();
        while let Some(prev) = self.closure_defs.get(&target) {
            if !seen.insert(target.clone()) {
                break;
            }
            let mut merged_args = prev.args.clone();
            merged_args.extend(args);
            args = merged_args;
            target = prev.of.clone();
        }
        Closure {
            name: closure.name.clone(),
            of: target,
            args,
            span: closure.span,
        }
    }
}

pub fn register_import(ctx: &mut Context, import_path: &str, span: Span) -> Result<(), Error> {
    let name = last_slug(import_path);

    let spec = builtins::get_spec(name, span).ok_or_else(|| {
        Error::new(
            Code::Internal,
            format!("unknown import '@{}'", import_path),
            span,
        )
    })?;

    match spec {
        builtins::BuiltinSpec::Function(sig) => {
            ctx.add_sig(name, name, sig, span, true)?;
        }
        builtins::BuiltinSpec::Type(ty) => {
            ctx.add_type(name, name, ty, span, true)?;
        }
    }

    Ok(())
}
