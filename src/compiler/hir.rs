use crate::compiler::ast;
use crate::compiler::error;
use crate::compiler::error::{Code, Error};
use crate::compiler::format_hir;
pub use crate::compiler::hir_ast::*;
use crate::compiler::hir_context as ctx;
use crate::compiler::signature;
use crate::compiler::span::Span;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

pub struct Lowerer {
    ready: VecDeque<BlockItem>,
}

impl Lowerer {
    pub fn new() -> Self {
        Self {
            ready: VecDeque::new(),
        }
    }

    pub fn produce(&mut self) -> Option<BlockItem> {
        self.ready.pop_front()
    }

    pub fn consume(&mut self, ctx: &mut ctx::Context, block: ast::BlockItem) -> Result<(), Error> {
        match block {
            ast::BlockItem::Import { label, path, .. } => {
                let item = BlockItem::Import {
                    label: label.clone(),
                    path: path.clone(),
                };
                self.ready.push_back(item);
                ctx::register_import(ctx, &label, &path, Span::unknown())?;
            }
            ast::BlockItem::FunctionDef { name, lambda, .. } => {
                let display_name = name.clone();
                lower_function(ctx, name, Some(display_name), lambda, &mut self.ready, true)?;
            }
            other => {
                let lowered_items = lower_block_item(ctx, other, &mut self.ready)?;
                for item in lowered_items {
                    self.ready.push_back(item);
                }
            }
        }

        Ok(())
    }
}

fn lower_function(
    outer_ctx: &mut ctx::Context,
    name: String,
    display_name: Option<String>,
    lambda: ast::Lambda,
    hoisted: &mut VecDeque<BlockItem>,
    is_root_def: bool,
) -> Result<(), Error> {
    let span = Span::unknown();

    let lambda_params = signature::ast_signature_to_hir(lambda.params.clone());
    let mut signature_ctx = outer_ctx.enter(&name, display_name.as_deref(), is_root_def);
    register_generic_placeholders(&mut signature_ctx, &lambda_params.generics)?;
    let signature = signature::resolve_signature(&lambda_params, &mut signature_ctx);
    let new_name = outer_ctx.new_name_for_fn(display_name.as_deref());
    outer_ctx.add_sig(&name, &new_name, signature.clone(), span, false)?;

    // lower_params
    let params = signature.items;
    let mut ctx = outer_ctx.enter(&name, display_name.as_deref(), is_root_def);
    register_generic_placeholders(&mut ctx, &lambda_params.generics)?;
    for item in &params {
        ctx.add_param(&item.name, item.kind.clone(), Span::unknown(), false)?;
    }
    let mut lowered_items: Vec<BlockItem> = Vec::with_capacity(lambda.body.items.len());

    for item in lambda.body.items {
        match item {
            ast::BlockItem::Import { .. } => {
                unreachable!("in-function imports should be blocked by the parser")
            }
            ast::BlockItem::FunctionDef { name, lambda, .. } => {
                let display_name = name.clone();
                lower_function(&mut ctx, name, Some(display_name), lambda, hoisted, false)?;
            }
            other => {
                for item in lower_block_item(&mut ctx, other, hoisted)? {
                    lowered_items.push(item);
                }
            }
        }
    }

    // TODO: params are now pushed to the front...
    let params_signature = Signature {
        items: ctx.get_params(),
        generics: lambda.params.generics.clone(),
    };
    let normalized_sig = signature::normalize_signature(&params_signature, &ctx);
    let function = Function {
        name: new_name.to_string(),
        sig: normalized_sig,
        body: Block {
            items: lowered_items,
        },
    };
    hoisted.push_back(BlockItem::FunctionDef(function));

    // TODO: ABC: This needs looking into
    // This is meta info for anyone attempting to exec this function, that there's more args than they think due to captured params
    let captures = ctx.get_captures();
    if !captures.is_empty() {
        if let Some(entry) = outer_ctx.get_mut(&name) {
            entry.captures = captures;
        }
    }

    Ok(())
}

fn register_generic_placeholders(
    ctx: &mut ctx::Context,
    generics: &BTreeSet<String>,
) -> Result<(), Error> {
    for generic in generics {
        ctx.add_type(
            generic,
            generic,
            SigKind::Generic(generic.clone()),
            Span::unknown(),
            false,
        )?;
    }
    Ok(())
}

fn maybe_capture_name(ctx: &mut ctx::Context, name: &str) -> Result<(), Error> {
    if ctx.inner.contains_key(name) {
        return Ok(());
    }
    let entry: ContextEntry = ctx.outer.get(name).cloned().ok_or_else(|| {
        Error::new(
            Code::HIR,
            format!("`{}` is not defined", name),
            Span::unknown(), // TODO: Fix this
        )
    })?;
    if entry.is_builtin {
        return Ok(());
    }
    if !ctx.is_scope_ancestor(&entry) {
        return Ok(());
    }
    if entry.is_root && ctx.is_root_fn {
        return Ok(());
    }
    let span = entry.span.clone();
    let normalized_ty = signature::normalize_sig_kind(&entry.kind, ctx);
    ctx.add_param(&entry.name, normalized_ty, span, true)?; // Prevent infinite recursion, it's been turned into a local param.
    Ok(())
}

// TODO: This is a mess
fn emit_closure_for_term(
    ctx: &mut ctx::Context,
    name: &str,
    lowered_items: &mut Vec<BlockItem>,
    seen: &mut HashSet<String>,
) -> String {
    if !seen.insert(name.to_string()) {
        return name.to_string();
    }
    let mut result = name.to_string();
    if let Some(info) = ctx.closure_defs.get(name).cloned() {
        if ctx.emitted_closures.contains(name) {
            if info.args.is_empty() {
                result = info.of.clone();
            }
        } else {
            let mut lowered_args = Vec::with_capacity(info.args.len());
            for arg in &info.args {
                let arg_name = emit_closure_for_term(ctx, arg, lowered_items, seen);
                lowered_args.push(arg_name);
            }

            if lowered_args.is_empty() {
                result = info.of.clone();
            } else {
                lowered_items.push(BlockItem::ClosureDef(Closure {
                    name: name.to_string(),
                    of: info.of.clone(),
                    args: lowered_args,
                }));
            }

            ctx.emitted_closures.insert(name.to_string());
        }
    }
    seen.remove(name);
    result
}

fn lower_block_item(
    ctx: &mut ctx::Context,
    item: ast::BlockItem,
    hoisted: &mut VecDeque<BlockItem>,
) -> Result<Vec<BlockItem>, Error> {
    let lowered_items = match item {
        ast::BlockItem::ScopeCapture {
            params,
            continuation,
            term,
            ..
        } => {
            let lambda = ast::Lambda {
                params,
                body: continuation,
                args: Vec::new(),
                span: Span::unknown(),
            };
            let callback_term = ast::Term::Lambda(lambda);
            let exec_term = append_scope_capture_arg(term, callback_term)?;
            lower_exec(ctx, exec_term, hoisted)
        }
        ast::BlockItem::Ident(term) => lower_exec(ctx, ast::Term::Ident(term), hoisted),
        ast::BlockItem::Lambda(lambda) => lower_exec(ctx, ast::Term::Lambda(lambda), hoisted),
        ast::BlockItem::LitDef { name, literal, .. } => {
            let literal = lower_lit(literal.value);
            let kind = match &literal {
                Lit::Str(_) => SigKind::CompileTimeStr,
                Lit::Int(_) => SigKind::CompileTimeInt,
                Lit::F64(_) => SigKind::F64,
            };
            ctx.add_literal(&name, kind)?;
            Ok(vec![BlockItem::LitDef {
                name: name.clone(),
                literal,
            }])
        }
        ast::BlockItem::SigDef { name, sig, .. } => {
            ctx.add_type(
                &name,
                &name,
                SigKind::Ident(SigIdent { name: name.clone() }),
                Span::unknown(),
                false,
            )?;
            let sig = signature::ast_signature_to_hir(sig);
            let hir_sig = signature::resolve_signature(&sig, ctx);
            let normalized_sig = signature::normalize_signature(&hir_sig, ctx);
            if let Some(entry) = ctx.get_mut(&name) {
                entry.kind = SigKind::Sig(normalized_sig.clone());
            }
            let sig_def = BlockItem::SigDef {
                name,
                sig: normalized_sig,
            };
            Ok(vec![sig_def])
        }
        ast::BlockItem::IdentDef { name, ident, .. } => {
            if ident.args.is_empty() {
                if let Some(target) = ctx.get(&ident.name) {
                    ctx.add(&name, target.clone())?;
                    Ok(Vec::new())
                } else {
                    Err(error::new(
                        Code::HIR,
                        format!("could not resolve target '{}'", ident.name),
                        Span::unknown(),
                    ))
                }
            } else {
                let mut lowered_items = Vec::new();
                let closure = lower_closure(ctx, name.clone(), ident, hoisted, &mut lowered_items)?;
                let result_type = if let Some(signature) =
                    signature::resolve_target_signature(&closure.of, ctx)
                {
                    let remaining_items = signature
                        .items
                        .iter()
                        .skip(closure.args.len())
                        .cloned()
                        .collect::<Vec<_>>();
                    SigKind::Sig(Signature {
                        items: remaining_items,
                        generics: BTreeSet::new(),
                    })
                } else {
                    return Err(error::new(
                        Code::HIR,
                        format!("could not resolve target '{}'", closure.of),
                        Span::unknown(),
                    ));
                };
                ctx.register_closure(closure);
                ctx.add_literal(&name, result_type)?;
                Ok(lowered_items)
            }
        }
        ast::BlockItem::Import { .. } | ast::BlockItem::FunctionDef { .. } => {
            unreachable!("imports and functions should be handled separately")
        }
    }?;
    Ok(lowered_items)
}

fn append_scope_capture_arg(term: ast::Term, callback: ast::Term) -> Result<ast::Term, Error> {
    match term {
        ast::Term::Ident(mut ident) => {
            ident.args.push(ast::Arg {
                name: None,
                term: callback,
                span: Span::unknown(),
            });
            Ok(ast::Term::Ident(ident))
        }
        ast::Term::Lambda(mut lambda) => {
            lambda.args.push(ast::Arg {
                name: None,
                term: callback,
                span: Span::unknown(),
            });
            Ok(ast::Term::Lambda(lambda))
        }
        _ => Err(error::new(
            Code::Parse,
            "ctx capture target must be callable",
            Span::unknown(),
        )),
    }
}

fn lower_exec(
    ctx: &mut ctx::Context,
    term: ast::Term,
    hoisted: &mut VecDeque<BlockItem>,
) -> Result<Vec<BlockItem>, Error> {
    let mut emitted = HashSet::new(); // TODO: Should not be needed
    let mut lowered_items: Vec<BlockItem> = Vec::new();
    let exec = match term {
        ast::Term::Ident(ast_ident) => {
            let ast::Ident { name, args, .. } = ast_ident;
            maybe_capture_name(ctx, &name)?;
            let (target, args) = resolve_target(ctx, &name, args, hoisted, &mut lowered_items)?;
            ensure_exec_args_complete(ctx, &target, args.len())?;
            let of = emit_closure_for_term(ctx, &target.name, &mut lowered_items, &mut emitted);
            let args = args
                .into_iter()
                .map(|arg| emit_closure_for_term(ctx, &arg, &mut lowered_items, &mut emitted))
                .collect();
            Exec { of, args }
        }
        ast::Term::Lambda(lambda) => {
            let target_name = lower_lambda_term(ctx, lambda, hoisted, &mut lowered_items)?;
            let of = emit_closure_for_term(ctx, &target_name, &mut lowered_items, &mut emitted);
            Exec { of, args: vec![] }
        }
        other => unreachable!("expected exec term, got {:?}", other),
    };
    // Builtins like printf, sprintf, write, and puts rely on the AIR-level FFI bridge
    // so continuations and variadic arguments are resolved consistently later.
    lowered_items.push(BlockItem::Exec(exec));
    Ok(lowered_items)
}

fn lower_closure(
    ctx: &mut ctx::Context,
    name: String,
    ident: ast::Ident,
    hoisted: &mut VecDeque<BlockItem>,
    lowered_items: &mut Vec<BlockItem>,
) -> Result<Closure, Error> {
    maybe_capture_name(ctx, &ident.name)?;
    let ast::Ident {
        name: target_name,
        args: ast_args,
        ..
    } = ident;
    let (target, args) = resolve_target(ctx, &target_name, ast_args, hoisted, lowered_items)?;

    Ok(Closure {
        name,
        of: target.name,
        args,
    })
}

fn lower_lambda_term(
    ctx: &mut ctx::Context,
    lambda: ast::Lambda,
    hoisted: &mut VecDeque<BlockItem>,
    lowered_items: &mut Vec<BlockItem>,
) -> Result<String, Error> {
    let ast_args = lambda.args.clone(); // This is because I cheated to keep the AST simpler and made the lambda contain the args...

    let contextual_name = ctx.new_name();
    lower_function(ctx, contextual_name.clone(), None, lambda, hoisted, false)?;

    let (target, args) = resolve_target(ctx, &contextual_name, ast_args, hoisted, lowered_items)?;

    let target_name = target.name.clone();

    let apply_name = ctx.new_name();
    ctx.register_closure(Closure {
        name: apply_name.clone(),
        of: target_name,
        args,
    });

    Ok(apply_name)
}

fn lower_arg(
    ctx: &mut ctx::Context,
    term: ast::Term,
    expected_param: Option<&SigItem>,
    active_generics: &BTreeSet<String>,
    generic_bindings: &mut HashMap<String, SigKind>,
    hoisted: &mut VecDeque<BlockItem>,
    lowered_items: &mut Vec<BlockItem>,
) -> Result<String, Error> {
    let term = maybe_wrap_builtin(ctx, term, expected_param)?;
    validate_input_type(
        ctx,
        &term,
        expected_param,
        active_generics,
        generic_bindings,
    )?;
    let arg = match term {
        ast::Term::Ident(ast_ident) => {
            maybe_capture_name(ctx, &ast_ident.name)?;

            let (target, args) =
                resolve_target(ctx, &ast_ident.name, ast_ident.args, hoisted, lowered_items)?;

            if args.is_empty() {
                target.name
            } else {
                let new_name = ctx.new_name_for(&ast_ident.name); // TODO: Another new name for closure?
                ctx.register_closure(Closure {
                    name: new_name.clone(),
                    of: target.name,
                    args,
                });
                new_name
            }
        }
        ast::Term::Lit(literal) => {
            let new_name = ctx.new_name_for_literal();
            // Can theoretically get pushed to the root level
            lowered_items.push(BlockItem::LitDef {
                name: new_name.clone(),
                literal: lower_lit(literal.value),
            });
            new_name
        }
        ast::Term::Lambda(lambda) => {
            let apply_name = lower_lambda_term(ctx, lambda, hoisted, lowered_items)?;
            apply_name
        }
    };

    let mut seen = HashSet::new();
    Ok(emit_closure_for_term(ctx, &arg, lowered_items, &mut seen))
}

fn resolve_target(
    ctx: &mut ctx::Context,
    name: &str,
    ast_args: Vec<ast::Arg>,
    hoisted: &mut VecDeque<BlockItem>,
    lowered_items: &mut Vec<BlockItem>,
) -> Result<(ContextEntry, Vec<String>), Error> {
    let target: ContextEntry = ctx.get(name).cloned().ok_or_else(|| {
        error::new(
            Code::HIR,
            format!("could not resolve target '{}'", name),
            Span::unknown(), // TODO:
        )
    })?;

    for cap in &target.captures {
        maybe_capture_name(ctx, &cap.name)?;
    }

    let args: Vec<String> = target.captures.iter().map(|cap| cap.name.clone()).collect();

    if ast_args.is_empty() {
        return Ok((target, args));
    }

    let mut seen = HashSet::new();
    let signature =
        signature::signature_from_kind(&target.kind, ctx, &mut seen).ok_or_else(|| {
            error::new(
                Code::HIR,
                format!("could not resolve target signature '{}'", name),
                target.span.clone(),
            )
        })?;

    let resolved = resolve_call_arguments(ctx, &target, signature, ast_args, hoisted)?;
    let mut args: Vec<String> = Vec::with_capacity(resolved.target.captures.len());
    for cap in &resolved.target.captures {
        maybe_capture_name(ctx, &cap.name)?;
        args.push(cap.name.clone());
    }
    let expected_params =
        signature::expected_params_for_args(&resolved.signature.items, resolved.terms.len());

    let mut lowered = Vec::with_capacity(resolved.terms.len());
    let mut active_generics = current_scope_generics(ctx);
    active_generics.extend(resolved.signature.generics.iter().cloned());
    let mut generic_bindings = HashMap::new();
    for (term, expected_param) in resolved.terms.into_iter().zip(expected_params.into_iter()) {
        lowered.push(lower_arg(
            ctx,
            term,
            expected_param,
            &active_generics,
            &mut generic_bindings,
            hoisted,
            lowered_items,
        )?);
    }

    args.extend(lowered);
    let total_param_count = resolved.signature.items.len() + resolved.target.captures.len();
    if !resolved.signature.is_variadic() && args.len() > total_param_count {
        return Err(error::new(
            Code::HIR,
            format!(
                "function '{}' expected {} arguments but got {}",
                resolved.target.name,
                total_param_count,
                args.len()
            ),
            Span::unknown(),
        ));
    }
    Ok((resolved.target, args))
}

fn current_scope_generics(ctx: &ctx::Context) -> BTreeSet<String> {
    if let Some(entry) = ctx.get(&ctx.name) {
        if let SigKind::Sig(signature) = &entry.kind {
            return signature.generics.clone();
        }
    }
    BTreeSet::new()
}

struct ResolvedCallArgs {
    target: ContextEntry,
    signature: Signature,
    terms: Vec<ast::Term>,
}

fn resolve_call_arguments(
    ctx: &mut ctx::Context,
    target: &ContextEntry,
    signature: Signature,
    ast_args: Vec<ast::Arg>,
    hoisted: &mut VecDeque<BlockItem>,
) -> Result<ResolvedCallArgs, Error> {
    let has_named_args = ast_args.iter().any(|arg| arg.name.is_some());
    if !has_named_args {
        return Ok(ResolvedCallArgs {
            target: target.clone(),
            signature,
            terms: ast_args.into_iter().map(|arg| arg.term).collect(),
        });
    }

    let params = &signature.items;
    let mut assigned = HashSet::new();
    let mut resolved_indices = Vec::with_capacity(ast_args.len());
    let mut resolved_terms = Vec::with_capacity(ast_args.len());

    for call_arg in ast_args {
        let param_index = if let Some(arg_name) = call_arg.name {
            params
                .iter()
                .position(|param| !param.name.is_empty() && param.name == arg_name)
                .ok_or_else(|| {
                    error::new(
                        Code::HIR,
                        format!(
                            "function '{}' has no parameter named '{}'",
                            target.name, arg_name
                        ),
                        Span::unknown(),
                    )
                })?
        } else {
            first_unassigned_param(params.len(), &assigned).ok_or_else(|| {
                error::new(
                    Code::HIR,
                    format!(
                        "function '{}' expected {} arguments but got {}",
                        target.name,
                        target.captures.len() + params.len(),
                        target.captures.len() + resolved_indices.len() + 1
                    ),
                    Span::unknown(),
                )
            })?
        };

        if !assigned.insert(param_index) {
            let dup_name = params
                .get(param_index)
                .map(|param| param.name.as_str())
                .filter(|name| !name.is_empty())
                .unwrap_or("?");
            return Err(error::new(
                Code::HIR,
                format!(
                    "function '{}' argument '{}' was provided more than once",
                    target.name, dup_name
                ),
                Span::unknown(),
            ));
        }

        resolved_indices.push(param_index);
        resolved_terms.push(call_arg.term);
    }

    let mut new_order = Vec::with_capacity(params.len());
    let mut seen_indices = HashSet::new();
    for param_index in &resolved_indices {
        if seen_indices.insert(*param_index) {
            new_order.push(*param_index);
        }
    }
    for index in 0..params.len() {
        if seen_indices.insert(index) {
            new_order.push(index);
        }
    }

    let is_reordered = new_order
        .iter()
        .enumerate()
        .any(|(index, param_index)| index != *param_index);
    if !is_reordered {
        return Ok(ResolvedCallArgs {
            target: target.clone(),
            signature,
            terms: resolved_terms,
        });
    }

    let (wrapper_target, wrapper_signature) =
        create_named_arg_wrapper(ctx, target, &signature, &new_order, hoisted)?;
    Ok(ResolvedCallArgs {
        target: wrapper_target,
        signature: wrapper_signature,
        terms: resolved_terms,
    })
}

fn first_unassigned_param(param_count: usize, assigned: &HashSet<usize>) -> Option<usize> {
    (0..param_count).find(|index| !assigned.contains(index))
}

fn create_named_arg_wrapper(
    ctx: &mut ctx::Context,
    target: &ContextEntry,
    signature: &Signature,
    new_order: &[usize],
    hoisted: &mut VecDeque<BlockItem>,
) -> Result<(ContextEntry, Signature), Error> {
    let wrapper_name = ctx.new_name_for_fn(None);
    let mut used_names = HashSet::new();
    let mut reordered_params = Vec::with_capacity(signature.items.len());
    let mut by_original_index = vec![String::new(); signature.items.len()];

    for (position, param_index) in new_order.iter().enumerate() {
        let item = &signature.items[*param_index];
        let fallback_name = format!("_named_arg_{}", position);
        let preferred_name = if item.name.is_empty() {
            fallback_name.as_str()
        } else {
            item.name.as_str()
        };
        let param_name = unique_param_name(ctx, preferred_name, &mut used_names);
        by_original_index[*param_index] = param_name.clone();
        reordered_params.push(SigItem {
            name: param_name,
            kind: item.kind.clone(),
            has_bang: item.has_bang,
        });
    }

    let wrapper_sig = Signature {
        items: reordered_params.clone(),
        generics: signature.generics.clone(),
    };

    let mut wrapper_captures = Vec::new();
    let is_globally_addressable_target = target.is_root
        && target
            .scope
            .last()
            .is_some_and(|scope_name| scope_name == &target.name);
    if !is_globally_addressable_target {
        wrapper_captures.push(SigItem {
            name: target.name.clone(),
            kind: target.kind.clone(),
            has_bang: false,
        });
    }
    for capture in &target.captures {
        if !wrapper_captures
            .iter()
            .any(|item| item.name == capture.name)
        {
            wrapper_captures.push(capture.clone());
        }
    }

    ctx.add_sig(
        &wrapper_name,
        &wrapper_name,
        wrapper_sig.clone(),
        Span::unknown(),
        false,
    )?;

    if let Some(entry) = ctx.get_mut(&wrapper_name) {
        entry.captures = wrapper_captures.clone();
    }

    let mut runtime_sig_items = wrapper_captures.clone();
    runtime_sig_items.extend(reordered_params);

    let mut exec_args = target
        .captures
        .iter()
        .map(|capture| capture.name.clone())
        .collect::<Vec<_>>();
    for name in by_original_index {
        exec_args.push(name);
    }

    hoisted.push_back(BlockItem::FunctionDef(Function {
        name: wrapper_name.clone(),
        sig: Signature {
            items: runtime_sig_items,
            generics: signature.generics.clone(),
        },
        body: Block {
            items: vec![BlockItem::Exec(Exec {
                of: target.name.clone(),
                args: exec_args,
            })],
        },
    }));

    let wrapper_target = ctx.get(&wrapper_name).cloned().ok_or_else(|| {
        error::new(
            Code::HIR,
            format!("could not resolve target '{}'", wrapper_name),
            Span::unknown(),
        )
    })?;
    Ok((wrapper_target, wrapper_sig))
}

fn unique_param_name(
    ctx: &mut ctx::Context,
    preferred_name: &str,
    used_names: &mut HashSet<String>,
) -> String {
    if !preferred_name.is_empty() && used_names.insert(preferred_name.to_string()) {
        return preferred_name.to_string();
    }
    loop {
        let generated = ctx.new_name();
        if used_names.insert(generated.clone()) {
            return generated;
        }
    }
}

fn required_exec_arg_count(signature: &Signature, capture_count: usize) -> usize {
    let is_variadic = signature
        .items
        .iter()
        .any(|item| matches!(item.kind, SigKind::Variadic));
    let param_count = if is_variadic {
        signature.items.len().saturating_sub(1)
    } else {
        signature.items.len()
    };
    capture_count + param_count
}

fn ensure_exec_args_complete(
    ctx: &ctx::Context,
    target: &ContextEntry,
    arg_count: usize,
) -> Result<(), Error> {
    let mut seen = HashSet::new();
    let Some(signature) = signature::signature_from_kind(&target.kind, ctx, &mut seen) else {
        return Ok(());
    };
    let required_count = required_exec_arg_count(&signature, target.captures.len());
    if arg_count >= required_count {
        return Ok(());
    }
    Err(error::new(
        Code::HIR,
        format!(
            "cannot execute function '{}': not all args have been provided (expected at least {}, got {})",
            target.name, required_count, arg_count
        ),
        Span::unknown(),
    ))
}

fn maybe_wrap_builtin(
    ctx: &mut ctx::Context,
    term: ast::Term,
    expected_param: Option<&SigItem>,
) -> Result<ast::Term, Error> {
    let ast::Term::Ident(ident) = term else {
        return Ok(term);
    };

    let Some(entry) = ctx.get(&ident.name) else {
        return Ok(ast::Term::Ident(ident));
    };

    if !entry.is_builtin {
        return Ok(ast::Term::Ident(ident));
    }

    let Some(sig_item) = expected_param else {
        return Ok(ast::Term::Ident(ident));
    };

    let SigKind::Sig(signature) = &sig_item.kind else {
        return Ok(ast::Term::Ident(ident));
    };

    new_builtin_wrapper(ctx, ident, signature)
}

fn new_builtin_wrapper(
    ctx: &mut ctx::Context,
    ident: ast::Ident,
    expected_sig: &Signature,
) -> Result<ast::Term, Error> {
    let lambda_sig = ensure_param_names(ctx, expected_sig);

    let mut builtin_call = ident.clone();
    for item in &lambda_sig.items {
        builtin_call.args.push(ast::Arg {
            name: None,
            term: ast::Term::Ident(ast::Ident {
                name: item.name.clone(),
                args: Vec::new(),
                span: Span::unknown(),
            }),
            span: Span::unknown(),
        });
    }

    let body = ast::Block {
        items: vec![ast::BlockItem::Ident(builtin_call)],
        span: Span::unknown(),
    };

    Ok(ast::Term::Lambda(ast::Lambda {
        params: lambda_sig,
        body,
        args: Vec::new(),
        span: Span::unknown(),
    }))
}

fn ensure_param_names(ctx: &mut ctx::Context, expected_sig: &Signature) -> ast::Signature {
    let mut items = Vec::with_capacity(expected_sig.items.len());
    for item in &expected_sig.items {
        let param_name = if item.name.is_empty() {
            ctx.new_name()
        } else {
            item.name.clone()
        };
        items.push(SigItem {
            name: param_name,
            kind: item.kind.clone(),
            has_bang: item.has_bang,
        });
    }
    signature::hir_signature_to_ast(Signature {
        items,
        generics: expected_sig.generics.clone(),
    })
}

fn validate_input_type(
    ctx: &mut ctx::Context,
    term: &ast::Term,
    expected_param: Option<&SigItem>,
    active_generics: &BTreeSet<String>,
    generic_bindings: &mut HashMap<String, SigKind>,
) -> Result<(), Error> {
    let Some(expected) = expected_param else {
        return Ok(());
    };
    if matches!(expected.kind, SigKind::Variadic) {
        return Ok(());
    }

    let normalized_expected = signature::normalize_sig_kind(&expected.kind, ctx);
    ensure_sig_kind_exists(ctx, &normalized_expected, active_generics)?;

    let expected_is_compile_time = matches!(
        normalized_expected,
        SigKind::CompileTimeInt | SigKind::CompileTimeStr
    );
    let expected_is_unit_sig = if let SigKind::Sig(sig) = &normalized_expected {
        sig.items.is_empty()
    } else {
        false
    };
    let allow_idents = expected_is_compile_time
        || expected_is_unit_sig
        || has_generic_kind(&normalized_expected, active_generics);

    let Some(actual_kind) = term_sig_kind(ctx, term, allow_idents) else {
        return Ok(());
    };

    let actual_is_compile_time_int = matches!(actual_kind, SigKind::CompileTimeInt);
    let normalized_actual = signature::normalize_sig_kind(&actual_kind, ctx);

    if kind_matches(
        &normalized_actual,
        &normalized_expected,
        actual_is_compile_time_int,
        active_generics,
        generic_bindings,
    ) {
        return Ok(());
    }

    Err(error::new(
        Code::HIR,
        format!(
            "expected {}, found {}",
            format_hir::format_sig_kind(&normalized_expected),
            format_hir::format_sig_kind(&normalized_actual)
        ),
        Span::unknown(),
    ))
}

fn ensure_sig_kind_exists(
    ctx: &ctx::Context,
    kind: &SigKind,
    active_generics: &BTreeSet<String>,
) -> Result<(), Error> {
    match kind {
        SigKind::Ident(ident) => {
            if active_generics.contains(&ident.name) {
                return Ok(());
            }
            if ctx.get(&ident.name).is_none() {
                return Err(error::new(
                    Code::HIR,
                    format!("type `{}` is not defined", ident.name),
                    Span::unknown(),
                ));
            }
        }
        SigKind::Sig(signature) => {
            for item in &signature.items {
                ensure_sig_kind_exists(ctx, &item.kind, active_generics)?;
            }
        }
        SigKind::GenericInst { name, args } => {
            if ctx.get(name).is_none() {
                return Err(error::new(
                    Code::HIR,
                    format!("type `{}` is not defined", name),
                    Span::unknown(),
                ));
            }
            for arg in args {
                ensure_sig_kind_exists(ctx, arg, active_generics)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn term_sig_kind(ctx: &mut ctx::Context, term: &ast::Term, allow_idents: bool) -> Option<SigKind> {
    match term {
        ast::Term::Lit(ast::Literal {
            value: ast::Lit::Int(_),
            ..
        }) => Some(SigKind::CompileTimeInt),
        ast::Term::Lit(ast::Literal {
            value: ast::Lit::Str(_),
            ..
        }) => Some(SigKind::CompileTimeStr),
        ast::Term::Lit(ast::Literal {
            value: ast::Lit::F64(_),
            ..
        }) => Some(SigKind::F64),
        ast::Term::Ident(ast_ident) => {
            if allow_idents && ast_ident.args.is_empty() {
                ctx.get(&ast_ident.name).map(|entry| entry.kind.clone())
            } else {
                None
            }
        }
        ast::Term::Lambda(lambda) => {
            let mut signature = signature::resolve_signature(
                &signature::ast_signature_to_hir(lambda.params.clone()),
                ctx,
            );
            let drop_count = lambda.args.len().min(signature.items.len());
            signature.items.drain(0..drop_count);
            Some(SigKind::Sig(signature))
        }
    }
}

fn lower_lit(lit: ast::Lit) -> Lit {
    match lit {
        ast::Lit::Str(value) => Lit::Str(value),
        ast::Lit::Int(value) => Lit::Int(value),
        ast::Lit::F64(value) => Lit::F64(value),
    }
}

fn kind_matches(
    actual: &SigKind,
    expected: &SigKind,
    actual_is_compile_time_int: bool,
    active_generics: &BTreeSet<String>,
    generic_bindings: &mut HashMap<String, SigKind>,
) -> bool {
    if expected == actual {
        return true;
    }

    if let SigKind::Generic(name) = expected {
        return match_generic_kind(
            name,
            actual,
            actual_is_compile_time_int,
            active_generics,
            generic_bindings,
        );
    }
    if let SigKind::Ident(ident) = expected {
        if active_generics.contains(&ident.name) {
            return match_generic_kind(
                &ident.name,
                actual,
                actual_is_compile_time_int,
                active_generics,
                generic_bindings,
            );
        }
    }

    match expected {
        SigKind::CompileTimeInt => actual == &SigKind::CompileTimeInt,
        SigKind::CompileTimeStr => actual == &SigKind::CompileTimeStr,
        SigKind::F64 if actual_is_compile_time_int => true,
        SigKind::Sig(expected_sig) => {
            let SigKind::Sig(actual_sig) = actual else {
                return false;
            };
            if actual_sig.items.len() != expected_sig.items.len() {
                return false;
            }
            for (actual_item, expected_item) in
                actual_sig.items.iter().zip(expected_sig.items.iter())
            {
                if !kind_matches(
                    &actual_item.kind,
                    &expected_item.kind,
                    false,
                    active_generics,
                    generic_bindings,
                ) {
                    return false;
                }
            }
            true
        }
        SigKind::GenericInst {
            name: expected_name,
            args: expected_args,
        } => {
            let SigKind::GenericInst {
                name: actual_name,
                args: actual_args,
            } = actual
            else {
                return false;
            };
            if expected_name != actual_name || expected_args.len() != actual_args.len() {
                return false;
            }
            for (actual_arg, expected_arg) in actual_args.iter().zip(expected_args.iter()) {
                if !kind_matches(
                    actual_arg,
                    expected_arg,
                    false,
                    active_generics,
                    generic_bindings,
                ) {
                    return false;
                }
            }
            true
        }
        _ => canonicalize_kind(actual) == canonicalize_kind(expected),
    }
}

fn has_generic_kind(kind: &SigKind, active_generics: &BTreeSet<String>) -> bool {
    match kind {
        SigKind::Generic(_) => true,
        SigKind::Ident(ident) => active_generics.contains(&ident.name),
        SigKind::Sig(signature) => signature
            .items
            .iter()
            .any(|item| has_generic_kind(&item.kind, active_generics)),
        SigKind::GenericInst { args, .. } => args
            .iter()
            .any(|arg| has_generic_kind(arg, active_generics)),
        _ => false,
    }
}

fn match_generic_kind(
    name: &str,
    actual: &SigKind,
    actual_is_compile_time_int: bool,
    active_generics: &BTreeSet<String>,
    generic_bindings: &mut HashMap<String, SigKind>,
) -> bool {
    if let SigKind::Ident(actual_ident) = actual {
        if active_generics.contains(&actual_ident.name) {
            return actual_ident.name == name;
        }
    }
    if let SigKind::Generic(actual_name) = actual {
        return actual_name == name;
    }

    let canonical_actual = if actual_is_compile_time_int {
        SigKind::Int
    } else {
        canonicalize_kind(actual)
    };
    if let Some(bound) = generic_bindings.get(name) {
        return canonicalize_kind(bound) == canonical_actual;
    }
    generic_bindings.insert(name.to_string(), canonical_actual);
    true
}

fn canonicalize_kind(kind: &SigKind) -> SigKind {
    match kind {
        SigKind::CompileTimeInt => SigKind::Int,
        SigKind::CompileTimeStr => SigKind::Str,
        SigKind::Sig(signature) => SigKind::Sig(Signature {
            items: signature
                .items
                .iter()
                .map(|item| SigItem {
                    name: item.name.clone(),
                    kind: canonicalize_kind(&item.kind),
                    has_bang: item.has_bang,
                })
                .collect(),
            generics: signature.generics.clone(),
        }),
        SigKind::GenericInst { name, args } => SigKind::GenericInst {
            name: name.clone(),
            args: args.iter().map(canonicalize_kind).collect(),
        },
        other => other.clone(),
    }
}
