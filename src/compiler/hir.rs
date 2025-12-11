use crate::compiler::ast;
use crate::compiler::builtins;
use crate::compiler::error::{CompileError, CompileErrorCode};
pub use crate::compiler::hir_ast::*;
use crate::compiler::hir_context as ctx;
use crate::compiler::span::Span;
use std::collections::{HashMap, HashSet, VecDeque};

fn ast_signature_to_hir_signature(params: &ast::Signature, span: Span) -> Signature {
    let mut signature = Signature::from(params);
    signature.span = span;
    signature
}

fn lower_block(
    block: ast::Block,
    nested: &mut Vec<Function>,
    ctx: &mut ctx::Context,
    builtin_imports: &mut HashSet<String>,
) -> Result<Block, CompileError> {
    let len = block.items.len();
    let mut items = Vec::with_capacity(len);
    let ast_items = block.items;
    let mut idx = 0;

    while idx < ast_items.len() {
        let stmt = ast_items[idx].clone();
        match stmt {
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
                let exec_term = append_ctx_capture_arg(term, callback_term, term_span)?;
                let lowered_items = lower_exec_stmt(exec_term, nested, ctx, builtin_imports)?;
                items.extend(lowered_items);
            }
            ast::BlockItem::Ident(term) => {
                items.extend(lower_exec_stmt(
                    ast::Term::Ident(term),
                    nested,
                    ctx,
                    builtin_imports,
                )?);
            }
            ast::BlockItem::Lambda(lambda) => {
                items.extend(lower_exec_stmt(
                    ast::Term::Lambda(lambda),
                    nested,
                    ctx,
                    builtin_imports,
                )?);
            }
            ast::BlockItem::FunctionDef { name, lambda, span } => {
                let renamed = ctx.new_name_for(&name);
                insert_alias(ctx, &name, &renamed, span)?;
                let nested_item = ast::BlockItem::FunctionDef {
                    name: renamed.clone(),
                    lambda,
                    span,
                };
                let (function, extra) = lower_function(nested_item, ctx, builtin_imports)?;
                nested.extend(extra);
                nested.push(function.clone());
                items.push(BlockItem::FunctionDef(function));
            }
            ast::BlockItem::StrDef {
                name,
                literal,
                span,
            } => {
                let name_clone = name.clone();
                let literal_value = literal.value.clone();
                items.push(BlockItem::StrDef(StrLiteral {
                    name: name_clone.clone(),
                    value: literal.value,
                    span,
                }));
                ctx.insert(
                    &name_clone,
                    ctx::ContextEntry::Value {
                        ty: SigKind::Str,
                        span,
                        constant: ConstantValue::Str(literal_value),
                    },
                )?;
            }
            ast::BlockItem::IntDef {
                name,
                literal,
                span,
            } => {
                let name_clone = name.clone();
                let literal_value = literal.value;
                items.push(BlockItem::IntDef(IntLiteral {
                    name: name_clone.clone(),
                    value: literal.value,
                    span,
                }));
                ctx.insert(
                    &name_clone,
                    ctx::ContextEntry::Value {
                        ty: SigKind::Int,
                        span,
                        constant: ConstantValue::Int(literal_value),
                    },
                )?;
            }
            ast::BlockItem::IdentDef { name, ident, span } => {
                if ident.args.is_empty() {
                    let target = resolve_function_name(&ident.name, ctx);
                    insert_alias(ctx, &name, &target, span)?;
                    idx += 1;
                    continue;
                }
                let mut lowered_items = Vec::new();
                let apply = lower_ident_as_apply(
                    name.clone(),
                    ident,
                    nested,
                    &mut lowered_items,
                    ctx,
                    builtin_imports,
                )?;
                let result_type = if let Some(signature) = resolve_target_signature(&apply.of, ctx)
                {
                    let remaining_items = signature
                        .items
                        .iter()
                        .skip(apply.args.len())
                        .cloned()
                        .collect::<Vec<_>>();
                    SigKind::Tuple(Signature {
                        items: remaining_items,
                        span: Span::unknown(),
                    })
                } else {
                    return Err(CompileError::new(
                        CompileErrorCode::Resolve,
                        "could not resolve target",
                        span,
                    ));
                };
                items.extend(lowered_items);
                items.push(BlockItem::ApplyDef(apply));
                ctx.insert_type(&name, result_type, span, false)?;
            }
            ast::BlockItem::Import { name, span } => {
                let name_clone = name.clone();
                items.push(BlockItem::Import {
                    name: name_clone.clone(),
                    span,
                });
                let recorded = builtins::register_import(&name, span, ctx)?;
                builtin_imports.extend(recorded);
            }
            ast::BlockItem::SigDef {
                name,
                term,
                span,
                generics,
            } => {
                let sig_def = lower_sig_def(name, term, span, generics, ctx)?;
                items.push(sig_def);
            }
        }
        idx += 1;
    }

    Ok(Block {
        items,
        span: block.span,
    })
}

fn lower_sig_def(
    name: String,
    term: ast::SigKind,
    span: Span,
    generics: Vec<String>,
    ctx: &mut ctx::Context,
) -> Result<BlockItem, CompileError> {
    let generics_for_block = generics.clone();
    ctx.insert_type(&name, SigKind::from(&term), span, false)?;
    let kind = match ctx.get(&name) {
        Some(ctx::ContextEntry::Type { ty, .. }) => ty.clone(),
        _ => {
            return Err(CompileError::new(
                CompileErrorCode::Internal,
                format!("missing type info for '{}'", name),
                span,
            ));
        }
    };
    Ok(BlockItem::SigDef {
        name,
        kind,
        span,
        generics: generics_for_block,
    })
}

pub struct Lowerer {
    buffered_functions: Vec<Function>,
    ready: VecDeque<BlockItem>,
    builtin_imports: HashSet<String>,
}

impl Lowerer {
    pub fn new() -> Self {
        Self {
            buffered_functions: Vec::new(),
            ready: VecDeque::new(),
            builtin_imports: HashSet::new(),
        }
    }

    pub fn produce(&mut self) -> Option<BlockItem> {
        self.ready.pop_front()
    }

    pub fn consume(
        &mut self,
        stmt: ast::BlockItem,
        ctx: &mut ctx::Context,
    ) -> Result<(), CompileError> {
        match stmt {
            ast::BlockItem::Import { name, span } => {
                self.enqueue_block_item(
                    BlockItem::Import {
                        name: name.clone(),
                        span,
                    },
                    ctx,
                );
                let recorded = builtins::register_import(&name, span, ctx)?; // TODO: builtin instead of registering can simply return the value that can be used to register.
                self.builtin_imports.extend(recorded);
            }
            ast::BlockItem::FunctionDef { .. } => {
                let (function, mut extra) = lower_function(stmt, ctx, &mut self.builtin_imports)?;
                self.enqueue_block_item(BlockItem::FunctionDef(function), ctx);
                self.buffered_functions.append(&mut extra);
                self.emit_buffered_functions();
            }
            ast::BlockItem::StrDef {
                name,
                literal,
                span,
            } => {
                let value = literal.value.clone();
                let literal_item = StrLiteral {
                    name: name.clone(),
                    value: literal.value,
                    span,
                };
                self.enqueue_block_item(BlockItem::StrDef(literal_item), ctx);
                ctx.insert(
                    &name,
                    ctx::ContextEntry::Value {
                        ty: SigKind::Str,
                        span,
                        constant: ctx::ConstantValue::Str(value),
                    },
                )?;
            }
            ast::BlockItem::IntDef {
                name,
                literal,
                span,
            } => {
                let value = literal.value;
                let literal_item = IntLiteral {
                    name: name.clone(),
                    value,
                    span,
                };
                self.enqueue_block_item(BlockItem::IntDef(literal_item), ctx);
                ctx.insert(
                    &name,
                    ctx::ContextEntry::Value {
                        ty: SigKind::Int,
                        span,
                        constant: ConstantValue::Int(value),
                    },
                )?;
            }
            ast::BlockItem::SigDef {
                name,
                term,
                span,
                generics,
            } => {
                let sig_def = lower_sig_def(name, term, span, generics, ctx)?;
                self.enqueue_block_item(sig_def, ctx);
            }
            ast::BlockItem::Ident(term) => {
                let lowered_items = lower_exec_stmt(
                    ast::Term::Ident(term),
                    &mut self.buffered_functions,
                    ctx,
                    &mut self.builtin_imports,
                )?;
                self.emit_buffered_functions();
                for item in lowered_items {
                    self.enqueue_block_item(item, ctx);
                }
            }
            ast::BlockItem::Lambda(lambda) => {
                let lowered_items = lower_exec_stmt(
                    ast::Term::Lambda(lambda),
                    &mut self.buffered_functions,
                    ctx,
                    &mut self.builtin_imports,
                )?;
                self.emit_buffered_functions();
                for item in lowered_items {
                    self.enqueue_block_item(item, ctx);
                }
            }
            ast::BlockItem::IdentDef { name, ident, span } => {
                if ident.args.is_empty() {
                    let target = resolve_function_name(&ident.name, ctx);
                    insert_alias(ctx, &name, &target, span)?;
                } else {
                    let mut lowered_items = Vec::new();
                    let apply = lower_ident_as_apply(
                        name.clone(),
                        ident,
                        &mut self.buffered_functions,
                        &mut lowered_items,
                        ctx,
                        &mut self.builtin_imports,
                    )?;
                    let result_type =
                        if let Some(signature) = resolve_target_signature(&apply.of, ctx) {
                            let remaining_items = signature
                                .items
                                .iter()
                                .skip(apply.args.len())
                                .cloned()
                                .collect::<Vec<_>>();
                            SigKind::Tuple(Signature {
                                items: remaining_items,
                                span: Span::unknown(),
                            })
                        } else {
                            return Err(CompileError::new(
                                CompileErrorCode::Resolve,
                                "could not resolve target",
                                span,
                            ));
                        };
                    self.emit_buffered_functions();
                    for item in lowered_items {
                        self.enqueue_block_item(item, ctx);
                    }
                    self.enqueue_block_item(BlockItem::ApplyDef(apply), ctx);
                    ctx.insert_type(&name, result_type, span, false)?;
                }
            }
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
                let exec_term = append_ctx_capture_arg(term, callback_term, term_span)?;
                let lowered_items = lower_exec_stmt(
                    exec_term,
                    &mut self.buffered_functions,
                    ctx,
                    &mut self.builtin_imports,
                )?;
                self.emit_buffered_functions();
                for item in lowered_items {
                    self.enqueue_block_item(item, ctx);
                }
            }
        }
        self.emit_buffered_functions();
        Ok(())
    }

    fn emit_buffered_functions(&mut self) {
        for function in self.buffered_functions.drain(..) {
            self.ready.push_back(BlockItem::FunctionDef(function));
        }
    }

    fn enqueue_block_item(&mut self, mut item: BlockItem, ctx: &mut ctx::Context) {
        let mut prefix = Vec::new();
        match &mut item {
            BlockItem::Exec(exec) => {
                rewrite_args(&mut exec.args, &mut prefix, exec.span, ctx);
                rewrite_function_target(&mut exec.of, &mut prefix, exec.span, ctx);
            }
            BlockItem::ApplyDef(apply) => {
                rewrite_args(&mut apply.args, &mut prefix, apply.span, ctx);
                rewrite_function_target(&mut apply.of, &mut prefix, apply.span, ctx);
            }
            _ => {}
        }
        for prefix_item in prefix {
            self.ready.push_back(prefix_item);
        }
        self.ready.push_back(item);
    }
}

// TODO: we could just call lower_lambda here, it's identical.
pub fn lower_function(
    item: ast::BlockItem,
    outer_ctx: &mut ctx::Context,
    builtin_imports: &mut HashSet<String>,
) -> Result<(Function, Vec<Function>), CompileError> {
    let (name, ast_lambda, span) = match item {
        ast::BlockItem::FunctionDef { name, lambda, span } => (name, lambda, span),
        _ => unreachable!("lower_function expects function definitions"), // TODO: not ideal, can pass name, lambda and span separately and avoid this check
    };

    outer_ctx.insert_func(
        &name,
        ast_signature_to_hir_signature(&ast_lambda.params, span),
        span,
        true,
    )?;

    let mut ctx = outer_ctx.enter(&name);

    let params = lower_params(ast_lambda.params.clone(), &mut ctx)?; // TODO: can accept ctx and reg the params directly
    let mut nested = Vec::new();

    // TODO: How can lower_block function if it hasn't hasn't figured out the captured params?
    let body = lower_block(ast_lambda.body, &mut nested, &mut ctx, builtin_imports)?; // TODO: ctx should already be aware of the builtin_imports

    let normalized_body = normalize_block(body, &mut ctx); // TODO: either ctx or normalize_block can return the captured params

    // TODO: So it collects the params from the body and compares them to the params from the lambda.
    let nested_capture_params: Vec<ctx::CaptureParam> = Vec::new();
    let capture_params =
        collect_capture_params(&normalized_body, &params, &mut ctx, &nested_capture_params)?;

    let capture_sig_items: Vec<SigItem> = capture_params
        .iter()
        .map(|capture| capture.ty.clone())
        .collect();

    let mut all_params = capture_sig_items.clone();
    all_params.extend(params);

    if !capture_sig_items.is_empty() {
        outer_ctx.register_function_with_captures(&name, &capture_sig_items)?;
        outer_ctx.record_function_captures(&name, capture_params.clone());
    }

    Ok((
        Function {
            name,
            sig: Signature {
                items: all_params,
                span: ast_lambda.params.span,
            },
            body: normalized_body,
            span,
        },
        nested,
    ))
}

fn lower_params(sig: ast::Signature, ctx: &mut ctx::Context) -> Result<Vec<SigItem>, CompileError> {
    let mut used = HashSet::<String>::new();
    let mut counter = 0;

    let mut result = Vec::with_capacity(sig.items.len());

    for item in sig.items {
        // Determine or synthesize the name
        let name = if let Some(name) = item.name {
            if !used.insert(name.clone()) {
                return Err(CompileError::new(
                    CompileErrorCode::Parse,
                    format!("name '{}' is already used", name),
                    item.span,
                ));
            }
            name
        } else {
            // Generate a unique placeholder name
            loop {
                let candidate = format!("_{}", counter);
                counter += 1;

                if used.insert(candidate.clone()) {
                    break candidate;
                }
            }
        };

        let hir_kind = SigKind::from(&item.ty.kind);

        ctx.insert(
            &name,
            ctx::ContextEntry::Type {
                ty: hir_kind.clone(),
                span: item.span,
                is_signature: false,
            },
        )?;

        result.push(SigItem {
            name,
            ty: SigType {
                kind: hir_kind,
                span: item.ty.span,
            },
            span: item.span,
            is_variadic: item.is_variadic,
        });
    }

    Ok(result)
}

fn append_ctx_capture_arg(
    term: ast::Term,
    callback: ast::Term,
    span: Span,
) -> Result<ast::Term, CompileError> {
    match term {
        ast::Term::Ident(mut ident) => {
            ident.args.push(callback);
            Ok(ast::Term::Ident(ident))
        }
        ast::Term::Lambda(mut lambda) => {
            lambda.args.push(callback);
            Ok(ast::Term::Lambda(lambda))
        }
        _other => Err(CompileError::new(
            CompileErrorCode::Parse,
            "ctx capture target must be callable",
            span,
        )),
    }
}

fn collect_capture_params(
    block: &Block,
    params: &[SigItem],
    outer_ctx: &ctx::Context,
    nested_captures: &[ctx::CaptureParam],
) -> Result<Vec<ctx::CaptureParam>, CompileError> {
    let locals = gather_local_definitions(block, params);
    let mut captures = Vec::new();
    let mut seen = HashSet::new();

    for item in &block.items {
        match item {
            BlockItem::Exec(Exec { of, args, .. }) => {
                record_name_capture(of, &locals, outer_ctx, &mut captures, &mut seen);
                record_arg_captures(args, &locals, outer_ctx, &mut captures, &mut seen)?;
            }
            BlockItem::ApplyDef(Apply { of, args, .. }) => {
                record_name_capture(of, &locals, outer_ctx, &mut captures, &mut seen);
                record_arg_captures(args, &locals, outer_ctx, &mut captures, &mut seen)?;
            }
            _ => {}
        }
    }

    for capture in nested_captures {
        record_name_capture(&capture.name, &locals, outer_ctx, &mut captures, &mut seen);
    }

    return Ok(captures);
}

fn gather_local_definitions(block: &Block, params: &[SigItem]) -> HashSet<String> {
    let mut locals = HashSet::new();
    for param in params {
        locals.insert(param.name.clone());
    }
    for item in &block.items {
        match item {
            BlockItem::FunctionDef(function) => {
                locals.insert(function.name.clone());
            }
            BlockItem::StrDef(StrLiteral { name, .. })
            | BlockItem::IntDef(IntLiteral { name, .. }) => {
                locals.insert(name.clone());
            }
            BlockItem::ApplyDef(Apply { name, .. }) => {
                locals.insert(name.clone());
            }
            BlockItem::Exec(Exec {
                result: Some(name), ..
            }) => {
                locals.insert(name.clone());
            }
            _ => {}
        }
    }
    locals
}

fn record_arg_captures(
    args: &[Arg],
    locals: &HashSet<String>,
    outer_ctx: &ctx::Context,
    captures: &mut Vec<ctx::CaptureParam>,
    seen: &mut HashSet<String>,
) -> Result<(), CompileError> {
    for arg in args {
        let name = &arg.name;
        if locals.contains(name) || seen.contains(name) {
            continue;
        }
        if let Some(ctx_item) = outer_ctx.get(name) {
            if is_ctx_item_signature(ctx_item) {
                continue;
            }
            let (kind, span) = match ctx_item {
                ctx::ContextEntry::Type { ty, span, .. } => (ty, span),
                ctx::ContextEntry::Value { ty, span, .. } => (ty, span),
            };
            // TODO: too much cloning
            captures.push(ctx::CaptureParam {
                name: name.clone(),
                ty: SigItem {
                    name: name.clone(),
                    ty: SigType {
                        kind: kind.clone(),
                        span: span.clone(),
                    },
                    is_variadic: false, // TODO: No idea if actually false,
                    span: span.clone(),
                },
                span: *span,
            });

            seen.insert(name.clone());
        }
    }
    return Ok(());
}

fn record_name_capture(
    name: &str,
    locals: &HashSet<String>,
    outer_ctx: &ctx::Context,
    captures: &mut Vec<ctx::CaptureParam>,
    seen: &mut HashSet<String>,
) {
    if locals.contains(name) || seen.contains(name) {
        return;
    }
    if let Some(entry) = outer_ctx.get(name) {
        if is_ctx_item_signature(entry) {
            return;
        }
        let (kind, span) = match entry {
            ctx::ContextEntry::Type { ty, span, .. }
            | ctx::ContextEntry::Value { ty, span, .. } => (ty, span),
        };

        let captured_name = name.to_string();
        // TODO: too much cloning
        captures.push(ctx::CaptureParam {
            name: captured_name.clone(),
            ty: SigItem {
                name: captured_name.clone(),
                ty: SigType {
                    kind: kind.clone(),
                    span: span.clone(),
                },
                is_variadic: false, // TODO: No idea if actually false,
                span: span.clone(),
            },
            span: *span,
        });

        seen.insert(captured_name);
    }
}

// TODO: this function is suspicious, maybe remove it?
fn is_ctx_item_signature(entry: &ctx::ContextEntry) -> bool {
    matches!(
        entry,
        ctx::ContextEntry::Type {
            is_signature: true,
            ..
        }
    )
}

fn lower_exec_stmt(
    term: ast::Term,
    nested: &mut Vec<Function>,
    ctx: &mut ctx::Context,
    builtin_imports: &mut HashSet<String>,
) -> Result<Vec<BlockItem>, CompileError> {
    let mut items = Vec::new();
    let exec = match term {
        ast::Term::Ident(ident) => {
            lower_ident_as_exec(None, ident, nested, &mut items, ctx, builtin_imports)?
        }
        ast::Term::Lambda(lambda) => {
            lower_lambda_as_exec(None, lambda, nested, &mut items, ctx, builtin_imports)?
        }
        other => unreachable!("expected exec term, got {:?}", other),
    };
    items.push(BlockItem::Exec(exec));
    Ok(items)
}

fn lower_ident_as_exec(
    result: Option<String>,
    ast::Ident { name, args, span }: ast::Ident,
    nested: &mut Vec<Function>,
    items: &mut Vec<BlockItem>,
    ctx: &mut ctx::Context,
    builtin_imports: &mut HashSet<String>,
) -> Result<Exec, CompileError> {
    let resolved_name = resolve_function_name(&name, ctx);
    let sig = resolve_target_signature(&resolved_name, ctx);
    let builtin_expectations = builtin_arg_expectations(&resolved_name, args.len());
    let args = lower_terms_to_args(
        args,
        sig,
        builtin_expectations.as_deref(),
        nested,
        items,
        ctx,
        builtin_imports,
    )?;
    Ok(Exec {
        of: resolved_name,
        args,
        span,
        result,
    })
}

fn lower_ident_as_apply(
    name: String,
    ast::Ident {
        name: target,
        args,
        span,
    }: ast::Ident,
    nested: &mut Vec<Function>,
    items: &mut Vec<BlockItem>,
    ctx: &mut ctx::Context,
    builtin_imports: &mut HashSet<String>,
) -> Result<Apply, CompileError> {
    let resolved_target = resolve_function_name(&target, ctx);
    let sig = resolve_target_signature(&resolved_target, ctx);
    let args = lower_terms_to_args(args, sig, None, nested, items, ctx, builtin_imports)?;
    Ok(Apply {
        name,
        of: resolved_target,
        args,
        span,
    })
}

fn lower_lambda_as_exec(
    result: Option<String>,
    ast::Lambda {
        params,
        body,
        args,
        span,
    }: ast::Lambda,
    nested: &mut Vec<Function>,
    items: &mut Vec<BlockItem>,
    ctx: &mut ctx::Context,
    builtin_imports: &mut HashSet<String>,
) -> Result<Exec, CompileError> {
    let fn_name = ctx.new_name();

    let nested_item = ast::BlockItem::FunctionDef {
        name: fn_name.clone(),
        lambda: ast::Lambda {
            params,
            body,
            args: Vec::new(),
            span,
        },
        span,
    };

    let (lf, extra) = lower_function(nested_item, ctx, builtin_imports)?;

    nested.extend(extra);
    nested.push(lf);

    let lowered_args = lower_terms_to_args(args, None, None, nested, items, ctx, builtin_imports)?;
    Ok(Exec {
        of: fn_name,
        args: lowered_args,
        span,
        result,
    })
}

fn lower_terms_to_args(
    terms: Vec<ast::Term>,
    expected: Option<Signature>,
    builtin_expectations: Option<&[Option<SigKind>]>,
    nested: &mut Vec<Function>,
    items: &mut Vec<BlockItem>,
    ctx: &mut ctx::Context,
    builtin_imports: &mut HashSet<String>,
) -> Result<Vec<Arg>, CompileError> {
    let expected_hir_kinds = expected.map(|signature| {
        signature
            .items
            .iter()
            .map(|item| item.ty.kind.clone())
            .collect::<Vec<_>>()
    });

    terms
        .into_iter()
        .enumerate()
        .map(|(idx, term)| {
            let expected_ty = expected_hir_kinds.as_ref().and_then(|kinds| kinds.get(idx));
            let builtin_expected_ty = builtin_expectations
                .and_then(|expect| expect.get(idx))
                .and_then(|opt| opt.as_ref());
            let term = wrap_builtin_exec_in_lambda(
                term,
                expected_ty.or(builtin_expected_ty),
                ctx,
                builtin_imports,
            );
            lower_term_to_arg(term, nested, items, ctx, builtin_imports)
        })
        .collect()
}

fn lower_term_to_arg(
    term: ast::Term,
    nested: &mut Vec<Function>,
    items: &mut Vec<BlockItem>,
    ctx: &mut ctx::Context,
    builtin_imports: &mut HashSet<String>,
) -> Result<Arg, CompileError> {
    match term {
        ast::Term::Ident(ast_ident) => {
            if ast_ident.args.is_empty() {
                return Ok(Arg {
                    name: resolve_function_name(&ast_ident.name, ctx),
                    span: ast_ident.span,
                });
            }
            let args = lower_terms_to_args(
                ast_ident.args,
                None,
                None,
                nested,
                items,
                ctx,
                builtin_imports,
            )?;
            let temp_name = ctx.new_name_for(&ast_ident.name);
            items.push(BlockItem::Exec(Exec {
                of: resolve_function_name(&ast_ident.name, ctx),
                args,
                span: ast_ident.span,
                result: Some(temp_name.clone()),
            }));
            Ok(Arg {
                name: temp_name,
                span: ast_ident.span,
            })
        }
        ast::Term::Int(literal) => {
            let temp_name = ctx.new_name();
            items.push(BlockItem::IntDef(IntLiteral {
                name: temp_name.clone(),
                value: literal.value,
                span: literal.span,
            }));
            Ok(Arg {
                name: temp_name,
                span: literal.span,
            })
        }
        ast::Term::String(literal) => {
            let temp_name = ctx.new_name();
            items.push(BlockItem::StrDef(StrLiteral {
                name: temp_name.clone(),
                value: literal.value,
                span: literal.span,
            }));
            Ok(Arg {
                name: temp_name,
                span: literal.span,
            })
        }
        ast::Term::Lambda(lambda) => {
            let temp_name = ctx.new_name();
            let span = lambda.span;
            let exec = lower_lambda_as_exec(
                Some(temp_name.clone()),
                lambda,
                nested,
                items,
                ctx,
                builtin_imports,
            )?;
            items.push(BlockItem::Exec(exec));
            Ok(Arg {
                name: temp_name,
                span,
            })
        }
    }
}

fn wrap_builtin_exec_in_lambda(
    term: ast::Term,
    expected: Option<&SigKind>,
    ctx: &ctx::Context,
    builtin_imports: &HashSet<String>,
) -> ast::Term {
    if expected_is_function(expected, ctx) {
        if let ast::Term::Ident(ast_ident) = term {
            if builtin_imports.contains(&ast_ident.name) {
                let span = ast_ident.span;
                let block = ast::Block {
                    span,
                    items: vec![ast::BlockItem::Ident(ast_ident)],
                };
                return ast::Term::Lambda(ast::Lambda {
                    params: ast::Signature {
                        items: Vec::new(),
                        span,
                    },
                    body: block,
                    args: Vec::new(),
                    span,
                });
            }
            return ast::Term::Ident(ast_ident);
        }
    }
    term
}

fn builtin_arg_expectations(name: &str, arg_count: usize) -> Option<Vec<Option<SigKind>>> {
    if name == "printf" && arg_count > 0 {
        let mut expectations = vec![None; arg_count];
        expectations[arg_count - 1] = Some(SigKind::Tuple(Signature {
            items: Vec::new(),
            span: Span::unknown(),
        }));
        return Some(expectations);
    }
    None
}

fn expected_is_function(expected: Option<&SigKind>, ctx: &ctx::Context) -> bool {
    expected
        .and_then(|ty| {
            let mut visited = HashSet::new();
            signature_from_sig_kind(ty, ctx, &mut visited)
        })
        .is_some()
}

fn resolve_function_name(name: &str, ctx: &ctx::Context) -> String {
    let mut current = name.to_string();
    let mut seen = HashSet::new();
    while seen.insert(current.clone()) {
        if let Some(next) = alias_target(&current, ctx) {
            current = next;
            continue;
        }
        break;
    }
    current
}

fn alias_target(name: &str, ctx: &ctx::Context) -> Option<String> {
    if let Some(ctx::ContextEntry::Type {
        ty, is_signature, ..
    }) = ctx.get(name)
    {
        if *is_signature {
            if let SigKind::Ident(ident) = ty {
                return Some(ident.name.clone());
            }
        }
    }
    None
}

fn insert_alias(
    ctx: &mut ctx::Context,
    alias: &str,
    target: &str,
    span: Span,
) -> Result<(), CompileError> {
    ctx.insert_type(
        alias,
        SigKind::Ident(SigIdent {
            name: target.to_string(),
            has_bang: false,
        }),
        span,
        true,
    )
}

fn normalize_block(mut block: Block, ctx: &mut ctx::Context) -> Block {
    let mut items = rewrite_block_captures(block.items, ctx);

    loop {
        let use_sites = compute_use_sites(&items);

        if let Some((parent_idx, child_idx)) = find_apply_to_apply(&items, &use_sites) {
            inline_apply_into_apply(&mut items, parent_idx, child_idx);
            continue;
        }

        if let Some((parent_idx, inv_idx)) = find_apply_to_exec(&items, &use_sites) {
            inline_apply_into_exec(&mut items, parent_idx, inv_idx);
            continue;
        }

        break;
    }

    block.items = items;
    block
}

fn resolve_target_signature(target: &str, ctx: &ctx::Context) -> Option<Signature> {
    if let Some(entry) = ctx.get(target) {
        let mut visited = HashSet::new();
        if let Some(signature) = signature_from_ctx_item(entry, ctx, &mut visited) {
            return Some(signature);
        }
    }
    None
}

fn signature_from_ctx_item(
    entry: &ctx::ContextEntry,
    ctx: &ctx::Context,
    visited: &mut HashSet<String>,
) -> Option<Signature> {
    match entry {
        ctx::ContextEntry::Type { ty, .. } | ctx::ContextEntry::Value { ty, .. } => {
            signature_from_sig_kind(ty, ctx, visited)
        }
    }
}

fn signature_from_sig_kind(
    kind: &SigKind,
    ctx: &ctx::Context,
    visited: &mut HashSet<String>,
) -> Option<Signature> {
    match kind {
        SigKind::Tuple(signature) => Some(signature.clone()),
        SigKind::Ident(ident) => {
            let name = &ident.name;
            if !visited.insert(name.clone()) {
                return None;
            }
            let result = ctx
                .get(name)
                .and_then(|entry| signature_from_ctx_item(entry, ctx, visited)); // TODO: This recusion is not required, it's already normalized in ctx
            visited.remove(name);
            result
        }
        _ => None,
    }
}

fn rewrite_block_captures(items: Vec<BlockItem>, ctx: &mut ctx::Context) -> Vec<BlockItem> {
    let mut rewritten = Vec::new();

    for mut item in items {
        if let BlockItem::FunctionDef(function) = &mut item {
            function.body = Block {
                items: rewrite_block_captures(function.body.items.clone(), ctx),
                span: function.body.span,
            };
        }

        let mut prefix = Vec::new();
        match &mut item {
            BlockItem::Exec(exec) => {
                rewrite_args(&mut exec.args, &mut prefix, exec.span, ctx);
                rewrite_function_target(&mut exec.of, &mut prefix, exec.span, ctx);
            }
            BlockItem::ApplyDef(apply) => {
                rewrite_args(&mut apply.args, &mut prefix, apply.span, ctx);
                rewrite_function_target(&mut apply.of, &mut prefix, apply.span, ctx);
            }
            _ => {}
        }

        rewritten.extend(prefix);
        rewritten.push(item);
    }

    rewritten
}

fn rewrite_args(args: &mut [Arg], prefix: &mut Vec<BlockItem>, span: Span, ctx: &mut ctx::Context) {
    for arg in args.iter_mut() {
        rewrite_function_target(&mut arg.name, prefix, span, ctx);
    }
}

fn rewrite_function_target(
    target: &mut String,
    prefix: &mut Vec<BlockItem>,
    span: Span,
    ctx: &mut ctx::Context,
) {
    // TODO: ABC:, this is our target
    if let Some(captures) = ctx.function_captures(target) {
        let capture_defs = captures.to_vec();
        let temp_name = ctx.new_name_for(&target);
        let capture_args: Vec<Arg> = capture_defs
            .iter()
            .map(|capture| Arg {
                name: capture.name.clone(),
                span: capture.span,
            })
            .collect();
        prefix.push(BlockItem::ApplyDef(Apply {
            name: temp_name.clone(),
            of: target.clone(),
            args: capture_args,
            span,
        }));
        *target = temp_name;
    }
}

fn compute_use_sites(items: &[BlockItem]) -> HashMap<String, Vec<UseSite>> {
    let mut map = HashMap::new();

    for (idx, item) in items.iter().enumerate() {
        match item {
            BlockItem::ApplyDef(Apply { of, args, .. }) => {
                map.entry(of.clone())
                    .or_insert_with(Vec::new)
                    .push(UseSite::ApplyOf(idx));
                for arg in args {
                    map.entry(arg.name.clone())
                        .or_insert_with(Vec::new)
                        .push(UseSite::ExecArg);
                }
            }
            BlockItem::Exec(Exec { of, args, .. }) => {
                map.entry(of.clone())
                    .or_insert_with(Vec::new)
                    .push(UseSite::ExecOf(idx));
                for arg in args {
                    map.entry(arg.name.clone())
                        .or_insert_with(Vec::new)
                        .push(UseSite::ExecArg);
                }
            }
            _ => {}
        }
    }

    map
}

fn find_apply_to_apply(
    items: &[BlockItem],
    use_sites: &HashMap<String, Vec<UseSite>>,
) -> Option<(usize, usize)> {
    for (idx, item) in items.iter().enumerate() {
        if let BlockItem::ApplyDef(Apply { name, .. }) = item {
            if let Some(sites) = use_sites.get(name) {
                if sites.len() == 1 {
                    if let UseSite::ApplyOf(child_idx) = sites[0] {
                        return Some((idx, child_idx));
                    }
                }
            }
        }
    }
    None
}

fn find_apply_to_exec(
    items: &[BlockItem],
    use_sites: &HashMap<String, Vec<UseSite>>,
) -> Option<(usize, usize)> {
    for (idx, item) in items.iter().enumerate() {
        if let BlockItem::ApplyDef(Apply { name, .. }) = item {
            if let Some(sites) = use_sites.get(name) {
                if sites.len() == 1 {
                    if let UseSite::ExecOf(inv_idx) = sites[0] {
                        return Some((idx, inv_idx));
                    }
                }
            }
        }
    }
    None
}

fn inline_apply_into_apply(items: &mut Vec<BlockItem>, parent_idx: usize, child_idx: usize) {
    let parent = match items[parent_idx].clone() {
        BlockItem::ApplyDef(apply) => apply,
        _ => return,
    };

    let child_apply = match items[child_idx].clone() {
        BlockItem::ApplyDef(apply) => apply,
        _ => return,
    };

    let mut merged_args = parent.args.clone();
    merged_args.extend(child_apply.args.clone());

    let merged = Apply {
        name: child_apply.name.clone(),
        of: parent.of.clone(),
        args: merged_args,
        span: child_apply.span,
    };

    let child_idx = if parent_idx < child_idx {
        items.remove(parent_idx);
        child_idx - 1
    } else {
        items.remove(parent_idx);
        child_idx
    };

    items[child_idx] = BlockItem::ApplyDef(merged);
}

fn inline_apply_into_exec(items: &mut Vec<BlockItem>, parent_idx: usize, inv_idx: usize) {
    let parent = match items[parent_idx].clone() {
        BlockItem::ApplyDef(apply) => apply,
        _ => return,
    };

    let exec = match items[inv_idx].clone() {
        BlockItem::Exec(inv) => inv,
        _ => return,
    };

    let mut merged_args = parent.args.clone();
    merged_args.extend(exec.args.clone());

    let merged_exec = Exec {
        of: parent.of.clone(),
        args: merged_args,
        span: exec.span,
        result: exec.result.clone(),
    };

    let inv_idx = if parent_idx < inv_idx {
        items.remove(parent_idx);
        inv_idx - 1
    } else {
        items.remove(parent_idx);
        inv_idx
    };

    items[inv_idx] = BlockItem::Exec(merged_exec);
}

#[derive(Copy, Clone)]
enum UseSite {
    ApplyOf(usize),
    ExecOf(usize),
    ExecArg,
}
