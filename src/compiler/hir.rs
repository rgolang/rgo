use crate::compiler::ast;
use crate::compiler::builtins;
use crate::compiler::error::{CompileError, CompileErrorCode};
pub use crate::compiler::hir_ast::*;
use crate::compiler::hir_scope as scope;
use crate::compiler::span::Span;
use std::collections::{HashMap, HashSet, VecDeque};

fn ast_signature_to_hir_signature(params: &[ast::SigItem], span: Span) -> Signature {
    Signature {
        items: params
            .iter()
            .map(|item| SigItem {
                name: item.name.clone().unwrap_or_default(),
                ty: SigType {
                    kind: ast_type_ref_to_hir_type_ref(&item.ty.kind),
                    span: item.ty.span,
                },
                span: item.span,
                is_variadic: item.is_variadic,
            })
            .collect(),
        span,
    }
}

fn lower_block(
    block: ast::Block,
    nested: &mut Vec<Function>,
    scope: &mut scope::Scope,
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
                let exec_term = append_scope_capture_arg(term, callback_term, term_span)?;
                let lowered_items =
                    lower_exec_stmt(exec_term, nested, scope, builtin_imports)?;
                items.extend(lowered_items);
            }
            ast::BlockItem::Ident(term) => {
                items.extend(lower_exec_stmt(
                    ast::Term::Ident(term),
                    nested,
                    scope,
                    builtin_imports,
                )?);
            }
            ast::BlockItem::Lambda(lambda) => {
                items.extend(lower_exec_stmt(
                    ast::Term::Lambda(lambda),
                    nested,
                    scope,
                    builtin_imports,
                )?);
            }
            ast::BlockItem::FunctionDef {
                name,
                lambda,
                span,
            } => {
                let renamed = scope.new_name_for(&name);
                insert_alias(scope, &name, &renamed, span)?;
                let nested_item = ast::BlockItem::FunctionDef {
                    name: renamed.clone(),
                    lambda,
                    span,
                };
                let (function, extra) = lower_function(nested_item, scope, builtin_imports)?;
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
                scope.insert(
                    &name_clone,
                    scope::ScopeItem::Value {
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
                scope.insert(
                    &name_clone,
                    scope::ScopeItem::Value {
                        ty: SigKind::Int,
                        span,
                        constant: ConstantValue::Int(literal_value),
                    },
                )?;
            }
            ast::BlockItem::IdentDef { name, ident, span } => {
                if ident.args.is_empty() {
                    let target = resolve_function_name(&ident.name, scope);
                    insert_alias(scope, &name, &target, span)?;
                    idx += 1;
                    continue;
                }
                let mut lowered_items = Vec::new();
                let apply = lower_ident_as_apply(
                    name.clone(),
                    ident,
                    nested,
                    &mut lowered_items,
                    scope,
                    builtin_imports,
                )?;
                let result_type =
                    if let Some(signature) = resolve_target_signature(&apply.of, scope) {
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
                scope.insert_type(&name, result_type, span, false)?;
            }
            ast::BlockItem::Import { name, span } => {
                let name_clone = name.clone();
                items.push(BlockItem::Import {
                    name: name_clone.clone(),
                    span,
                });
                let recorded = builtins::register_import_scope(&name, span, scope)?;
                builtin_imports.extend(recorded);
            }
            ast::BlockItem::SigDef {
                name,
                term,
                span,
                generics,
            } => {
                let sig_def = lower_sig_def(name, term, span, generics, scope)?;
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
    scope: &mut scope::Scope,
) -> Result<BlockItem, CompileError> {
    let generics_for_block = generics.clone();
    scope.insert_type(&name, ast_type_ref_to_hir_type_ref(&term), span, false)?;
    let kind = match scope.get(&name) {
        Some(scope::ScopeItem::Type { ty, .. }) => ty.clone(),
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

    pub fn consume(
        &mut self,
        item: ast::BlockItem,
        scope: &mut scope::Scope,
    ) -> Result<(), CompileError> {
        self.process_item(item, scope)?;
        Ok(())
    }

    pub fn produce(&mut self) -> Option<BlockItem> {
        self.ready.pop_front()
    }

    pub fn finish(&mut self) -> Result<(), CompileError> {
        Ok(())
    }

    fn process_item(
        &mut self,
        stmt: ast::BlockItem,
        scope: &mut scope::Scope,
    ) -> Result<(), CompileError> {
        match stmt {
            ast::BlockItem::Import { name, span } => {
                let name_clone = name.clone();
                self.enqueue_block_item(
                    BlockItem::Import {
                        name: name_clone.clone(),
                        span,
                    },
                    scope,
                );
                let recorded = builtins::register_import_scope(&name, span, scope)?;
                self.builtin_imports.extend(recorded);
            }
            ast::BlockItem::FunctionDef { .. } => {
                let (function, mut extra) = lower_function(stmt, scope, &mut self.builtin_imports)?;
                self.enqueue_block_item(BlockItem::FunctionDef(function), scope);
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
                self.enqueue_block_item(BlockItem::StrDef(literal_item), scope);
                scope.insert(
                    &name,
                    scope::ScopeItem::Value {
                        ty: SigKind::Str,
                        span,
                        constant: scope::ConstantValue::Str(value),
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
                self.enqueue_block_item(BlockItem::IntDef(literal_item), scope);
                scope.insert(
                    &name,
                    scope::ScopeItem::Value {
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
                let sig_def = lower_sig_def(name, term, span, generics, scope)?;
                self.enqueue_block_item(sig_def, scope);
            }
            ast::BlockItem::Ident(term) => {
                let lowered_items = lower_exec_stmt(
                    ast::Term::Ident(term),
                    &mut self.buffered_functions,
                    scope,
                    &mut self.builtin_imports,
                )?;
                self.emit_buffered_functions();
                for item in lowered_items {
                    self.enqueue_block_item(item, scope);
                }
            }
            ast::BlockItem::Lambda(lambda) => {
                let lowered_items = lower_exec_stmt(
                    ast::Term::Lambda(lambda),
                    &mut self.buffered_functions,
                    scope,
                    &mut self.builtin_imports,
                )?;
                self.emit_buffered_functions();
                for item in lowered_items {
                    self.enqueue_block_item(item, scope);
                }
            }
            ast::BlockItem::IdentDef { name, ident, span } => {
                if ident.args.is_empty() {
                    let target = resolve_function_name(&ident.name, scope);
                    insert_alias(scope, &name, &target, span)?;
                } else {
                    let mut lowered_items = Vec::new();
                    let apply = lower_ident_as_apply(
                        name.clone(),
                        ident,
                        &mut self.buffered_functions,
                        &mut lowered_items,
                        scope,
                        &mut self.builtin_imports,
                    )?;
                    let result_type =
                        if let Some(signature) = resolve_target_signature(&apply.of, scope) {
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
                        self.enqueue_block_item(item, scope);
                    }
                    self.enqueue_block_item(BlockItem::ApplyDef(apply), scope);
                    scope.insert_type(&name, result_type, span, false)?;
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
                let exec_term = append_scope_capture_arg(term, callback_term, term_span)?;
                let lowered_items = lower_exec_stmt(
                    exec_term,
                    &mut self.buffered_functions,
                    scope,
                    &mut self.builtin_imports,
                )?;
                self.emit_buffered_functions();
                for item in lowered_items {
                    self.enqueue_block_item(item, scope);
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

    fn enqueue_block_item(&mut self, mut item: BlockItem, scope: &mut scope::Scope) {
        let mut prefix = Vec::new();
        match &mut item {
            BlockItem::Exec(exec) => {
                rewrite_args(&mut exec.args, &mut prefix, exec.span, scope);
                rewrite_function_target(&mut exec.of, &mut prefix, exec.span, scope);
            }
            BlockItem::ApplyDef(apply) => {
                rewrite_args(&mut apply.args, &mut prefix, apply.span, scope);
                rewrite_function_target(&mut apply.of, &mut prefix, apply.span, scope);
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
    outer_scope: &mut scope::Scope,
    builtin_imports: &mut HashSet<String>,
) -> Result<(Function, Vec<Function>), CompileError> {
    let (name, ast_lambda, span) = match item {
        ast::BlockItem::FunctionDef { name, lambda, span } => (name, lambda, span),
        _ => unreachable!("lower_function expects function definitions"), // TODO: not ideal, can pass name, lambda and span separately and avoid this check
    };

    outer_scope.insert_func(
        &name,
        ast_signature_to_hir_signature(&ast_lambda.params.items, span),
        span,
        true,
    )?;

    let mut scope = outer_scope.enter(&name);

    let params = lower_params(ast_lambda.params.clone(), &mut scope)?; // TODO: can accept scope and reg the params directly
    let mut nested = Vec::new();

    // TODO: How can lower_block function if it hasn't hasn't figured out the captured params?
    let body = lower_block(ast_lambda.body, &mut nested, &mut scope, builtin_imports)?; // TODO: scope should already be aware of the builtin_imports

    let normalized_body = normalize_block(body, &mut scope); // TODO: either scope or normalize_block can return the captured params

    // TODO: So it collects the params from the body and compares them to the params from the lambda.
    let nested_capture_params: Vec<scope::CaptureParam> = Vec::new();
    let capture_params = collect_capture_params(
        &normalized_body,
        &params,
        &mut scope,
        &nested_capture_params,
    )?;

    let capture_sig_items: Vec<SigItem> = capture_params
        .iter()
        .map(|capture| SigItem {
            name: capture.name.clone(),
            ty: SigType {
                kind: ast_type_ref_to_hir_type_ref(&capture.ty.ty.kind),
                span: capture.ty.ty.span,
            },
            span: capture.span,
            is_variadic: capture.ty.is_variadic,
        })
        .collect();

    let mut all_params = capture_sig_items.clone();
    all_params.extend(params);

    if !capture_sig_items.is_empty() {
        outer_scope.register_function_with_captures(&name, &capture_sig_items)?;
        outer_scope.record_function_captures(&name, capture_params.clone());
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

fn lower_params(
    sig: ast::Signature,
    scope: &mut scope::Scope,
) -> Result<Vec<SigItem>, CompileError> {
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

        let hir_kind = ast_type_ref_to_hir_type_ref(&item.ty.kind);

        scope.insert(
            &name,
            scope::ScopeItem::Type {
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

fn append_scope_capture_arg(
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
            "scope capture target must be callable",
            span,
        )),
    }
}

fn collect_capture_params(
    block: &Block,
    params: &[SigItem],
    outer_scope: &scope::Scope,
    nested_captures: &[scope::CaptureParam],
) -> Result<Vec<scope::CaptureParam>, CompileError> {
    let locals = gather_local_definitions(block, params);
    let mut captures = Vec::new();
    let mut seen = HashSet::new();

    for item in &block.items {
        match item {
            BlockItem::Exec(Exec { of, args, .. }) => {
                record_name_capture(of, &locals, outer_scope, &mut captures, &mut seen);
                record_arg_captures(args, &locals, outer_scope, &mut captures, &mut seen)?;
            }
            BlockItem::ApplyDef(Apply { of, args, .. }) => {
                record_name_capture(of, &locals, outer_scope, &mut captures, &mut seen);
                record_arg_captures(args, &locals, outer_scope, &mut captures, &mut seen)?;
            }
            _ => {}
        }
    }

    for capture in nested_captures {
        record_name_capture(
            &capture.name,
            &locals,
            outer_scope,
            &mut captures,
            &mut seen,
        );
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
    outer_scope: &scope::Scope,
    captures: &mut Vec<scope::CaptureParam>,
    seen: &mut HashSet<String>,
) -> Result<(), CompileError> {
    for arg in args {
        let name = &arg.name;
        if locals.contains(name) || seen.contains(name) {
            continue;
        }
        if let Some(scope_item) = outer_scope.get(name) {
            if is_scope_item_signature(scope_item) {
                continue;
            }
            let (ty, span) = match scope_item {
                scope::ScopeItem::Type { ty, span, .. } => (ty, span),
                scope::ScopeItem::Value { ty, span, .. } => (ty, span),
            };
            let ast_typ = ast::SigItem {
                name: Some(name.clone()),
                ty: ast::SigType {
                    kind: hir_type_ref_to_ast_type_ref(ty),
                    span: *span,
                },
                is_variadic: false,
                span: *span,
            };
            captures.push(scope::CaptureParam {
                name: name.clone(),
                ty: ast_typ,
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
    outer_scope: &scope::Scope,
    captures: &mut Vec<scope::CaptureParam>,
    seen: &mut HashSet<String>,
) {
    if locals.contains(name) || seen.contains(name) {
        return;
    }
    if let Some(entry) = outer_scope.get(name) {
        if is_scope_item_signature(entry) {
            return;
        }
        let (ty, span) = match entry {
            scope::ScopeItem::Type { ty, span, .. } | scope::ScopeItem::Value { ty, span, .. } => {
                (ty, span)
            }
        };

        let captured_name = name.to_string();
        let ast_typ = ast::SigItem {
            name: Some(captured_name.clone()),
            ty: ast::SigType {
                kind: hir_type_ref_to_ast_type_ref(ty),
                span: *span,
            },
            is_variadic: false,
            span: *span,
        };
        captures.push(scope::CaptureParam {
            name: captured_name.clone(),
            ty: ast_typ,
            span: *span,
        });

        seen.insert(captured_name);
    }
}

fn is_scope_item_signature(entry: &scope::ScopeItem) -> bool {
    matches!(
        entry,
        scope::ScopeItem::Type {
            is_signature: true,
            ..
        }
    )
}

fn lower_exec_stmt(
    term: ast::Term,
    nested: &mut Vec<Function>,
    scope: &mut scope::Scope,
    builtin_imports: &mut HashSet<String>,
) -> Result<Vec<BlockItem>, CompileError> {
    let mut items = Vec::new();
    let exec = match term {
        ast::Term::Ident(ident) => lower_ident_as_exec(
            None,
            ident,
            nested,
            &mut items,
            scope,
            builtin_imports,
        )?,
        ast::Term::Lambda(lambda) => lower_lambda_as_exec(
            None,
            lambda,
            nested,
            &mut items,
            scope,
            builtin_imports,
        )?,
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
    scope: &mut scope::Scope,
    builtin_imports: &mut HashSet<String>,
) -> Result<Exec, CompileError> {
    let resolved_name = resolve_function_name(&name, scope);
    let sig = resolve_target_signature(&resolved_name, scope);
    let builtin_expectations = builtin_arg_expectations(&resolved_name, args.len());
    let args = lower_terms_to_args(
        args,
        sig,
        builtin_expectations.as_deref(),
        nested,
        items,
        scope,
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
    scope: &mut scope::Scope,
    builtin_imports: &mut HashSet<String>,
) -> Result<Apply, CompileError> {
    let resolved_target = resolve_function_name(&target, scope);
    let sig = resolve_target_signature(&resolved_target, scope);
    let args = lower_terms_to_args(
        args,
        sig,
        None,
        nested,
        items,
        scope,
        builtin_imports,
    )?;
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
    scope: &mut scope::Scope,
    builtin_imports: &mut HashSet<String>,
) -> Result<Exec, CompileError> {
    let fn_name = scope.new_name();

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

    let (lf, extra) = lower_function(nested_item, scope, builtin_imports)?;

    nested.extend(extra);
    nested.push(lf);

    let lowered_args = lower_terms_to_args(
        args,
        None,
        None,
        nested,
        items,
        scope,
        builtin_imports,
    )?;
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
    builtin_expectations: Option<&[Option<ast::SigKind>]>,
    nested: &mut Vec<Function>,
    items: &mut Vec<BlockItem>,
    scope: &mut scope::Scope,
    builtin_imports: &mut HashSet<String>,
) -> Result<Vec<Arg>, CompileError> {
    let expected_ast_kinds = expected.map(|signature| {
        signature
            .items
            .iter()
            .map(|item| hir_type_ref_to_ast_type_ref(&item.ty.kind))
            .collect::<Vec<_>>()
    });

    terms
        .into_iter()
        .enumerate()
        .map(|(idx, term)| {
            let expected_ty = expected_ast_kinds.as_ref().and_then(|kinds| kinds.get(idx));
            let builtin_expected_ty = builtin_expectations
                .and_then(|expect| expect.get(idx))
                .and_then(|opt| opt.as_ref());
            let term = wrap_builtin_exec_in_lambda(
                term,
                expected_ty.or(builtin_expected_ty),
                scope,
                builtin_imports,
            );
            lower_term_to_arg(term, nested, items, scope, builtin_imports)
        })
        .collect()
}

fn lower_term_to_arg(
    term: ast::Term,
    nested: &mut Vec<Function>,
    items: &mut Vec<BlockItem>,
    scope: &mut scope::Scope,
    builtin_imports: &mut HashSet<String>,
) -> Result<Arg, CompileError> {
    match term {
        ast::Term::Ident(ast_ident) => {
            if ast_ident.args.is_empty() {
                return Ok(Arg {
                    name: resolve_function_name(&ast_ident.name, scope),
                    span: ast_ident.span,
                });
            }
            let args = lower_terms_to_args(
                ast_ident.args,
                None,
                None,
                nested,
                items,
                scope,
                builtin_imports,
            )?;
            let temp_name = scope.new_name_for(&ast_ident.name);
            items.push(BlockItem::Exec(Exec {
                of: resolve_function_name(&ast_ident.name, scope),
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
            let temp_name = scope.new_name();
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
            let temp_name = scope.new_name();
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
            let temp_name = scope.new_name();
            let span = lambda.span;
            let exec = lower_lambda_as_exec(
                Some(temp_name.clone()),
                lambda,
                nested,
                items,
                scope,
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
    expected: Option<&ast::SigKind>,
    scope: &scope::Scope,
    builtin_imports: &HashSet<String>,
) -> ast::Term {
    let expected_hir = expected.map(|ty| ast_type_ref_to_hir_type_ref(ty));
    if expected_is_function(expected_hir.as_ref(), scope) {
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

fn builtin_arg_expectations(name: &str, arg_count: usize) -> Option<Vec<Option<ast::SigKind>>> {
    if name == "printf" && arg_count > 0 {
        let mut expectations = vec![None; arg_count];
        expectations[arg_count - 1] = Some(ast::SigKind::Tuple(ast::Signature::from_kinds(
            Vec::new(),
            Span::unknown(),
        )));
        return Some(expectations);
    }
    None
}

fn expected_is_function(expected: Option<&SigKind>, scope: &scope::Scope) -> bool {
    expected
        .and_then(|ty| {
            let mut visited = HashSet::new();
            signature_from_sig_kind(ty, scope, &mut visited)
        })
        .is_some()
}

fn resolve_function_name(name: &str, scope: &scope::Scope) -> String {
    let mut current = name.to_string();
    let mut seen = HashSet::new();
    while seen.insert(current.clone()) {
        if let Some(next) = alias_target(&current, scope) {
            current = next;
            continue;
        }
        break;
    }
    current
}

fn alias_target(name: &str, scope: &scope::Scope) -> Option<String> {
    if let Some(scope::ScopeItem::Type { ty, is_signature, .. }) = scope.get(name) {
        if *is_signature {
            if let SigKind::Ident(ident) = ty {
                return Some(ident.name.clone());
            }
        }
    }
    None
}

fn insert_alias(
    scope: &mut scope::Scope,
    alias: &str,
    target: &str,
    span: Span,
) -> Result<(), CompileError> {
    scope.insert_type(
        alias,
        SigKind::Ident(SigIdent {
            name: target.to_string(),
            has_bang: false,
        }),
        span,
        true,
    )
}

fn normalize_block(mut block: Block, scope: &mut scope::Scope) -> Block {
    let mut items = rewrite_block_captures(block.items, scope);

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

// TODO: Probably should not need this
pub fn ast_type_ref_to_hir_type_ref(ty: &ast::SigKind) -> SigKind {
    match ty {
        ast::SigKind::Int => SigKind::Int,
        ast::SigKind::Str => SigKind::Str,
        ast::SigKind::CompileTimeInt => SigKind::CompileTimeInt,
        ast::SigKind::CompileTimeStr => SigKind::CompileTimeStr,

        ast::SigKind::Tuple(sig) => SigKind::Tuple(Signature {
            items: sig
                .items
                .iter()
                .map(|item| SigItem {
                    name: item.name.clone().unwrap_or_default(),
                    ty: SigType {
                        kind: ast_type_ref_to_hir_type_ref(&item.ty.kind),
                        span: Span::unknown(),
                    },
                    span: Span::unknown(),
                    is_variadic: false,
                })
                .collect(),
            span: Span::unknown(),
        }),

        ast::SigKind::Ident(ident) => SigKind::Ident(SigIdent {
            name: ident.name.clone(),
            has_bang: ident.has_bang,
        }),

        ast::SigKind::GenericInst { name, args } => SigKind::GenericInst {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| ast_type_ref_to_hir_type_ref(&arg.kind))
                .collect(),
        },

        ast::SigKind::Generic(name) => SigKind::Generic(name.clone()),
    }
}

// TOOD: should also be removed

pub fn hir_type_ref_to_ast_type_ref(ty: &SigKind) -> ast::SigKind {
    match ty {
        SigKind::Int => ast::SigKind::Int,
        SigKind::Str => ast::SigKind::Str,
        SigKind::CompileTimeInt => ast::SigKind::CompileTimeInt,
        SigKind::CompileTimeStr => ast::SigKind::CompileTimeStr,

        // ----- Tuple -----
        SigKind::Tuple(sig) => {
            let sig_items: Vec<ast::SigItem> = sig
                .items
                .iter()
                .map(|hir_item| {
                    let kind = hir_type_ref_to_ast_type_ref(&hir_item.ty.kind);

                    ast::SigItem {
                        name: Some(hir_item.name.clone()),
                        ty: ast::SigType {
                            kind,
                            span: Span::unknown(), // TODO: improve span
                        },
                        is_variadic: hir_item.is_variadic, // preserve variadic marker too
                        span: Span::unknown(),             // TODO
                    }
                })
                .collect();

            ast::SigKind::Tuple(ast::Signature {
                items: sig_items,
                span: Span::unknown(), // TODO if you track tuple span
            })
        }

        // ----- Alias -----
        SigKind::Ident(ident) => ast::SigKind::Ident(ast::SigIdent {
            name: ident.name.clone(),
            has_bang: false,
            span: Span::unknown(),
        }),

        // ----- GenericInst -----
        SigKind::GenericInst { name, args } => {
            let arg_types: Vec<ast::SigType> = args
                .iter()
                .map(|arg| {
                    let kind = hir_type_ref_to_ast_type_ref(arg);
                    ast::SigType {
                        kind,
                        span: Span::unknown(), // TODO
                    }
                })
                .collect();

            ast::SigKind::GenericInst {
                name: name.clone(),
                args: arg_types,
            }
        }

        // ----- Generic -----
        SigKind::Generic(name) => ast::SigKind::Generic(name.clone()),
    }
}

fn resolve_target_signature(target: &str, scope: &scope::Scope) -> Option<Signature> {
    if let Some(entry) = scope.get(target) {
        let mut visited = HashSet::new();
        if let Some(signature) = signature_from_scope_item(entry, scope, &mut visited) {
            return Some(signature);
        }
    }
    None
}

fn signature_from_scope_item(
    entry: &scope::ScopeItem,
    scope: &scope::Scope,
    visited: &mut HashSet<String>,
) -> Option<Signature> {
    match entry {
        scope::ScopeItem::Type { ty, .. } | scope::ScopeItem::Value { ty, .. } => {
            signature_from_sig_kind(ty, scope, visited)
        }
    }
}

fn signature_from_sig_kind(
    kind: &SigKind,
    scope: &scope::Scope,
    visited: &mut HashSet<String>,
) -> Option<Signature> {
    match kind {
        SigKind::Tuple(signature) => Some(signature.clone()),
        SigKind::Ident(ident) => {
            let name = &ident.name;
            if !visited.insert(name.clone()) {
                return None;
            }
            let result = scope
                .get(name)
                .and_then(|entry| signature_from_scope_item(entry, scope, visited));
            visited.remove(name);
            result
        }
        _ => None,
    }
}

fn rewrite_block_captures(items: Vec<BlockItem>, scope: &mut scope::Scope) -> Vec<BlockItem> {
    let mut rewritten = Vec::new();

    for mut item in items {
        if let BlockItem::FunctionDef(function) = &mut item {
            function.body = Block {
                items: rewrite_block_captures(function.body.items.clone(), scope),
                span: function.body.span,
            };
        }

        let mut prefix = Vec::new();
        match &mut item {
            BlockItem::Exec(exec) => {
                rewrite_args(&mut exec.args, &mut prefix, exec.span, scope);
                rewrite_function_target(&mut exec.of, &mut prefix, exec.span, scope);
            }
            BlockItem::ApplyDef(apply) => {
                rewrite_args(&mut apply.args, &mut prefix, apply.span, scope);
                rewrite_function_target(&mut apply.of, &mut prefix, apply.span, scope);
            }
            _ => {}
        }

        rewritten.extend(prefix);
        rewritten.push(item);
    }

    rewritten
}

fn rewrite_args(
    args: &mut [Arg],
    prefix: &mut Vec<BlockItem>,
    span: Span,
    scope: &mut scope::Scope,
) {
    for arg in args.iter_mut() {
        rewrite_function_target(&mut arg.name, prefix, span, scope);
    }
}

fn rewrite_function_target(
    target: &mut String,
    prefix: &mut Vec<BlockItem>,
    span: Span,
    scope: &mut scope::Scope,
) {
    // TODO: ABC:, this is our target
    if let Some(captures) = scope.function_captures(target) {
        let capture_defs = captures.to_vec();
        let temp_name = scope.new_name_for(&target);
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
