use crate::compiler::ast;
use crate::compiler::ast::{Arg, SigItem, SigKind, Signature};
use crate::compiler::error;
use crate::compiler::error::{Code, Error};
use crate::compiler::format_hir;
pub use crate::compiler::hir_ast::*;
use crate::compiler::hir_context as ctx;
use crate::compiler::signature;
use crate::compiler::span::Span;
use std::collections::{HashSet, VecDeque};

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

    pub fn consume(&mut self, ctx: &mut ctx::Context, stmt: ast::BlockItem) -> Result<(), Error> {
        match stmt {
            ast::BlockItem::Import { name, span } => {
                let item = BlockItem::Import {
                    name: name.clone(),
                    span,
                };
                self.ready.push_back(item);
                ctx::register_import(ctx, &name, span)?; // TODO: Get a definition from builtins and reg it.
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
    let span = lambda.span;

    let signature = signature::resolve_ast_signature(&lambda.params, outer_ctx);
    let new_name = outer_ctx.new_name_for_fn(display_name.as_deref());
    outer_ctx.add_sig(&name, &new_name, signature.clone(), span, false)?;

    // lower_params
    let params = signature.items;
    let mut ctx = outer_ctx.enter(&name, display_name.as_deref(), is_root_def);
    for item in &params {
        ctx.add_param(&item.name, item.ty.clone(), item.span, false)?;
    }
    let mut lowered_items = Vec::with_capacity(lambda.body.items.len());

    for ast_item in lambda.body.items {
        match ast_item {
            ast::BlockItem::Import { .. } => {
                unreachable!("in-function imports should be blocked by the parser")
            }
            ast::BlockItem::FunctionDef { name, lambda, .. } => {
                let display_name = name.clone();
                lower_function(&mut ctx, name, Some(display_name), lambda, hoisted, false)?;
            }
            other => {
                lowered_items.extend(lower_block_item(&mut ctx, other, hoisted)?);
            }
        }
    }

    // TODO: params are now pushed to the front...
    let params_signature = Signature {
        items: ctx.get_params(),
        span: lambda.params.span,
        generics: lambda.params.generics.clone(),
    };
    let normalized_sig = signature::normalize_signature(&params_signature, &ctx);
    let function = Function {
        name: new_name.to_string(),
        sig: normalized_sig,
        body: Block {
            items: lowered_items,
            span: lambda.body.span,
        },
        span,
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
                let arg_name = emit_closure_for_term(ctx, &arg.name, lowered_items, seen);
                lowered_args.push(ast::Arg {
                    name: arg_name,
                    span: arg.span,
                });
            }

            if lowered_args.is_empty() {
                result = info.of.clone();
            } else {
                lowered_items.push(BlockItem::ClosureDef(Closure {
                    name: name.to_string(),
                    of: info.of.clone(),
                    args: lowered_args,
                    span: info.span,
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
            span,
        } => {
            let lambda = ast::Lambda {
                params,
                body: continuation,
                args: Vec::new(),
                span,
            };
            let callback_term = ast::Term::Lambda(lambda);
            let term_span = term.span();
            let exec_term = append_scope_capture_arg(term, callback_term, term_span)?;
            lower_exec(ctx, exec_term, hoisted)
        }
        ast::BlockItem::Ident(term) => lower_exec(ctx, ast::Term::Ident(term), hoisted),
        ast::BlockItem::Lambda(lambda) => lower_exec(ctx, ast::Term::Lambda(lambda), hoisted),
        ast::BlockItem::StrDef {
            name,
            literal,
            span,
        } => {
            let literal_item = ast::StrLiteral {
                value: literal.value,
                span,
            };
            ctx.add_literal(&name, SigKind::CompileTimeStr, span)?;
            Ok(vec![BlockItem::StrDef {
                name: name.clone(),
                literal: literal_item,
            }])
        }
        ast::BlockItem::IntDef {
            name,
            literal,
            span,
        } => {
            let literal_item = ast::IntLiteral {
                value: literal.value,
                span,
            };
            ctx.add_literal(&name, SigKind::CompileTimeInt, span)?;
            Ok(vec![BlockItem::IntDef {
                name: name.clone(),
                literal: literal_item,
            }])
        }
        ast::BlockItem::SigDef { name, sig, span } => {
            ctx.add_type(
                &name,
                &name,
                SigKind::Ident(ast::SigIdent {
                    name: name.clone(),
                    span,
                }),
                span,
                false,
            )?;
            let hir_sig = signature::resolve_ast_signature(&sig, ctx);
            let normalized_sig = signature::normalize_signature(&hir_sig, ctx);
            if let Some(entry) = ctx.get_mut(&name) {
                entry.kind = SigKind::Sig(normalized_sig.clone());
            }
            let sig_def = BlockItem::SigDef {
                name,
                sig: normalized_sig,
                span,
            };
            Ok(vec![sig_def])
        }
        ast::BlockItem::IdentDef { name, ident, span } => {
            if ident.args.is_empty() {
                if let Some(target) = ctx.get(&ident.name) {
                    ctx.add(&name, target.clone())?;
                    Ok(Vec::new())
                } else {
                    Err(error::new(
                        Code::HIR,
                        format!("could not resolve target '{}'", ident.name),
                        span,
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
                        span: Span::unknown(),
                        generics: Vec::new(),
                    })
                } else {
                    return Err(error::new(
                        Code::HIR,
                        format!("could not resolve target '{}'", closure.of),
                        span,
                    ));
                };
                ctx.register_closure(closure);
                ctx.add_type(&name, &name, result_type, span, false)?;
                Ok(lowered_items)
            }
        }
        _ => unreachable!("imports and functions should be handled separately"),
    }?;
    Ok(lowered_items)
}

fn append_scope_capture_arg(
    term: ast::Term,
    callback: ast::Term,
    span: Span,
) -> Result<ast::Term, Error> {
    match term {
        ast::Term::Ident(mut ident) => {
            ident.args.push(callback);
            Ok(ast::Term::Ident(ident))
        }
        ast::Term::Lambda(mut lambda) => {
            lambda.args.push(callback);
            Ok(ast::Term::Lambda(lambda))
        }
        _ => Err(error::new(
            Code::Parse,
            "ctx capture target must be callable",
            span,
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
            let ast::Ident { name, args, span } = ast_ident;
            maybe_capture_name(ctx, &name)?;
            let (target, args) = resolve_target(ctx, &name, args, hoisted, &mut lowered_items)?;
            let of = emit_closure_for_term(ctx, &target.name, &mut lowered_items, &mut emitted);
            Exec { of, args, span }
        }
        ast::Term::Lambda(lambda) => {
            let span = lambda.span;
            let target_name = lower_lambda_term(ctx, lambda, hoisted, &mut lowered_items)?;
            let of = emit_closure_for_term(ctx, &target_name, &mut lowered_items, &mut emitted);
            Exec {
                of,
                args: vec![],
                span,
            }
        }
        other => unreachable!("expected exec term, got {:?}", other),
    };
    // Builtins like printf, sprintf, write, and puts rely on the MIR-level FFI bridge
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
        span,
    } = ident;
    let (target, args) = resolve_target(ctx, &target_name, ast_args, hoisted, lowered_items)?;

    Ok(Closure {
        name,
        of: target.name,
        args,
        span,
    })
}

fn lower_lambda_term(
    ctx: &mut ctx::Context,
    lambda: ast::Lambda,
    hoisted: &mut VecDeque<BlockItem>,
    lowered_items: &mut Vec<BlockItem>,
) -> Result<String, Error> {
    let span = lambda.span.clone();
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
        span,
    });

    Ok(apply_name)
}

fn lower_arg(
    ctx: &mut ctx::Context,
    term: ast::Term,
    expected_param: Option<&SigItem>,
    hoisted: &mut VecDeque<BlockItem>,
    lowered_items: &mut Vec<BlockItem>,
) -> Result<Arg, Error> {
    let term = maybe_wrap_builtin(ctx, term, expected_param)?;
    validate_input_type(ctx, &term, expected_param)?;
    let arg = match term {
        ast::Term::Ident(ast_ident) => {
            maybe_capture_name(ctx, &ast_ident.name)?;

            let (target, args) =
                resolve_target(ctx, &ast_ident.name, ast_ident.args, hoisted, lowered_items)?;

            if args.is_empty() {
                Arg {
                    name: target.name,
                    span: ast_ident.span,
                }
            } else {
                let new_name = ctx.new_name_for(&ast_ident.name); // TODO: Another new name for closure?
                ctx.register_closure(Closure {
                    name: new_name.clone(),
                    of: target.name,
                    args,
                    span: ast_ident.span,
                });
                Arg {
                    name: new_name,
                    span: ast_ident.span,
                }
            }
        }
        ast::Term::Int(literal) => {
            let new_name = ctx.new_name_for_literal();
            // Can theoretically get pushed to the root level
            lowered_items.push(BlockItem::IntDef {
                name: new_name.clone(),
                literal: ast::IntLiteral {
                    value: literal.value,
                    span: literal.span,
                },
            });
            Arg {
                name: new_name,
                span: literal.span,
            }
        }
        ast::Term::String(literal) => {
            let new_name = ctx.new_name_for_literal();
            // Can theoretically get pushed to the root level
            lowered_items.push(BlockItem::StrDef {
                name: new_name.clone(),
                literal: ast::StrLiteral {
                    value: literal.value,
                    span: literal.span,
                },
            });
            Arg {
                name: new_name,
                span: literal.span,
            }
        }
        ast::Term::Lambda(lambda) => {
            let span = lambda.span;
            let apply_name = lower_lambda_term(ctx, lambda, hoisted, lowered_items)?;
            Arg {
                name: apply_name,
                span,
            }
        }
    };

    let span = arg.span;
    let mut seen = HashSet::new();
    let new_arg_name = emit_closure_for_term(ctx, &arg.name, lowered_items, &mut seen);
    return Ok(Arg {
        name: new_arg_name,
        span,
    });
}

fn resolve_target(
    ctx: &mut ctx::Context,
    name: &str,
    ast_args: Vec<ast::Term>,
    hoisted: &mut VecDeque<BlockItem>,
    lowered_items: &mut Vec<BlockItem>,
) -> Result<(ContextEntry, Vec<Arg>), Error> {
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

    let mut args: Vec<Arg> = target
        .captures
        .iter()
        .map(|cap| Arg {
            name: cap.name.clone(),
            span: cap.span,
        })
        .collect();

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

    let params = &signature.items;
    let expected_params = signature::expected_params_for_args(params, ast_args.len());

    let mut lowered = Vec::with_capacity(ast_args.len());
    for (term, expected_param) in ast_args.into_iter().zip(expected_params.into_iter()) {
        lowered.push(lower_arg(
            ctx,
            term,
            expected_param,
            hoisted,
            lowered_items,
        )?);
    }

    args.extend(lowered);
    Ok((target, args))
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

    let SigKind::Sig(signature) = &sig_item.ty else {
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
        builtin_call.args.push(ast::Term::Ident(ast::Ident {
            name: item.name.clone(),
            args: Vec::new(),
            span: Span::unknown(),
        }));
    }

    let body = ast::Block {
        items: vec![ast::BlockItem::Ident(builtin_call)],
        span: ident.span,
    };

    Ok(ast::Term::Lambda(ast::Lambda {
        params: lambda_sig,
        body,
        args: Vec::new(),
        span: ident.span,
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
        items.push(ast::SigItem {
            name: param_name,
            ty: item.ty.clone(),
            has_bang: item.has_bang,
            span: item.span,
        });
    }
    ast::Signature {
        items,
        span: expected_sig.span,
        generics: expected_sig.generics.clone(),
    }
}

fn validate_input_type(
    ctx: &mut ctx::Context,
    term: &ast::Term,
    expected_param: Option<&SigItem>,
) -> Result<(), Error> {
    let Some(expected) = expected_param else {
        return Ok(());
    };
    if matches!(expected.ty, SigKind::Variadic) {
        return Ok(());
    }

    let normalized_expected = signature::normalize_sig_kind(&expected.ty, ctx);
    ensure_sig_kind_exists(ctx, &normalized_expected, term.span())?;

    let expected_is_compile_time = matches!(
        normalized_expected,
        SigKind::CompileTimeInt | SigKind::CompileTimeStr
    );
    let expected_is_unit_sig = if let SigKind::Sig(sig) = &normalized_expected {
        sig.items.is_empty()
    } else {
        false
    };
    let allow_idents = expected_is_compile_time || expected_is_unit_sig;

    let Some(actual_kind) = term_sig_kind(ctx, term, allow_idents) else {
        return Ok(());
    };

    let normalized_actual = signature::normalize_sig_kind(&actual_kind, ctx);

    if kind_matches(&normalized_actual, &normalized_expected) {
        return Ok(());
    }

    Err(error::new(
        Code::HIR,
        format!(
            "expected {}, found {}",
            format_hir::format_sig_kind(&normalized_expected),
            format_hir::format_sig_kind(&normalized_actual)
        ),
        term.span(),
    ))
}

fn ensure_sig_kind_exists(
    ctx: &ctx::Context,
    kind: &SigKind,
    fallback_span: Span,
) -> Result<(), Error> {
    match kind {
        SigKind::Ident(ident) => {
            if ctx.get(&ident.name).is_none() {
                return Err(error::new(
                    Code::HIR,
                    format!("type `{}` is not defined", ident.name),
                    ident.span,
                ));
            }
        }
        SigKind::Sig(signature) => {
            for item in &signature.items {
                ensure_sig_kind_exists(ctx, &item.ty, item.span)?;
            }
        }
        SigKind::GenericInst { name, args } => {
            if ctx.get(name).is_none() {
                return Err(error::new(
                    Code::HIR,
                    format!("type `{}` is not defined", name),
                    fallback_span,
                ));
            }
            for arg in args {
                ensure_sig_kind_exists(ctx, arg, fallback_span)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn term_sig_kind(ctx: &mut ctx::Context, term: &ast::Term, allow_idents: bool) -> Option<SigKind> {
    match term {
        ast::Term::Int(_) => Some(SigKind::CompileTimeInt),
        ast::Term::String(_) => Some(SigKind::CompileTimeStr),
        ast::Term::Ident(ast_ident) => {
            if allow_idents && ast_ident.args.is_empty() {
                ctx.get(&ast_ident.name).map(|entry| entry.kind.clone())
            } else {
                None
            }
        }
        ast::Term::Lambda(lambda) => {
            let signature = signature::resolve_ast_signature(&lambda.params, ctx);
            let normalized = signature::normalize_signature(&signature, ctx);
            Some(SigKind::Sig(normalized))
        }
    }
}

fn kind_matches(actual: &SigKind, expected: &SigKind) -> bool {
    if expected == actual {
        return true;
    }

    match expected {
        SigKind::CompileTimeInt => actual == &SigKind::CompileTimeInt,
        SigKind::CompileTimeStr => actual == &SigKind::CompileTimeStr,
        _ => canonicalize_kind(actual) == canonicalize_kind(expected),
    }
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
                    ty: canonicalize_kind(&item.ty),
                    has_bang: item.has_bang,
                    span: item.span,
                })
                .collect(),
            span: signature.span,
            generics: signature.generics.clone(),
        }),
        SigKind::GenericInst { name, args } => SigKind::GenericInst {
            name: name.clone(),
            args: args.iter().map(canonicalize_kind).collect(),
        },
        other => other.clone(),
    }
}
