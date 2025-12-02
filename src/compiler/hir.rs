use crate::compiler::ast::{
    Block as AstBlock, Ident as AstIdent, Item as AstItem, Lambda as AstLambda, Param as AstParam,
    Params as AstParams, Term as AstTerm, TypeRef,
};
use crate::compiler::error::{CompileError, ParseError};
use crate::compiler::span::Span;
use crate::compiler::symbol::{self, CaptureParam, FunctionSig, SymbolRegistry};
use std::collections::{HashMap, HashSet};

pub type Env = HashMap<String, EnvEntry>;

#[derive(Clone, Debug)]
pub enum ConstantValue {
    Str(String),
    Int(i64),
}

#[derive(Clone)]
pub struct EnvEntry {
    pub ty: TypeRef,
    pub span: Span,
    pub constant: Option<ConstantValue>,
}

pub const ENTRY_FUNCTION_NAME: &str = "_start";

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeRef,
    pub span: Span,
    pub is_variadic: bool,
}

impl Param {
    pub fn is_variadic(&self) -> bool {
        self.is_variadic
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub items: Vec<BlockItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum BlockItem {
    FunctionDef(Function),
    StrDef(StrLiteral),
    IntDef(IntLiteral),
    ApplyDef(Apply),
    Invocation(Invocation),
    ReleaseEnv(ReleaseEnv),
}

#[derive(Debug, Clone)]
pub struct StrLiteral {
    pub name: String,
    pub value: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IntLiteral {
    pub name: String,
    pub value: i64,
    pub span: Span,
}

impl BlockItem {
    pub fn span(&self) -> Span {
        match self {
            BlockItem::FunctionDef(function) => function.span,
            BlockItem::StrDef(StrLiteral { span, .. })
            | BlockItem::IntDef(IntLiteral { span, .. })
            | BlockItem::ApplyDef(Apply { span, .. })
            | BlockItem::Invocation(Invocation { span, .. }) => *span,
            BlockItem::ReleaseEnv(ReleaseEnv { span, .. }) => *span,
        }
    }

    pub fn binding_info(&self) -> Option<(&String, Span)> {
        match self {
            BlockItem::StrDef(literal) => Some((&literal.name, literal.span)),
            BlockItem::IntDef(literal) => Some((&literal.name, literal.span)),
            BlockItem::Invocation(Invocation {
                result: Some(name),
                span,
                ..
            }) => Some((name, *span)),
            BlockItem::ApplyDef(Apply { name, span, .. }) => Some((name, *span)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Invocation {
    pub of: String,
    pub args: Vec<Arg>,
    pub span: Span,
    pub result: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Apply {
    pub name: String,
    pub of: String,
    pub args: Vec<Arg>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ReleaseEnv {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ArrayAlloc {
    pub name: String,
    pub array_ty: TypeRef,
    pub element_ty: TypeRef,
    pub elements: Vec<Arg>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Arg {
    pub name: String,
    pub span: Span,
}

fn assign_param_names(params: Vec<AstParam>) -> Vec<Param> {
    let mut used = HashSet::new();
    for param in &params {
        if let Some(name) = param.name() {
            used.insert(name.to_string());
        }
    }

    let mut counter = 0;
    params
        .into_iter()
        .map(|param| match param {
            AstParam::NameAndType {
                name,
                ty,
                span,
                is_variadic,
            } => Param {
                name,
                ty,
                span,
                is_variadic: is_variadic,
            },
            AstParam::TypeOnly {
                ty,
                span,
                is_variadic,
            } => {
                let mut candidate;
                loop {
                    candidate = format!("_{}", counter);
                    counter += 1;
                    if used.insert(candidate.clone()) {
                        break;
                    }
                }
                Param {
                    name: candidate,
                    ty,
                    span,
                    is_variadic,
                }
            }
            AstParam::NameOnly {
                name,
                span,
                is_variadic: _,
            } => {
                panic!("lambda parameter '{}' missing type at {:?}", name, span);
            }
        })
        .collect()
}

pub fn lower_function(
    item: AstItem,
    symbols: &mut SymbolRegistry,
    outer_env: &Env,
) -> Result<(Function, Vec<Function>), CompileError> {
    let (name, ast_lambda, span) = match item {
        AstItem::FunctionDef { name, lambda, span } => (name, lambda, span),
        _ => unreachable!("lower_function expects function definitions"),
    };

    let mut params = assign_param_names(ast_lambda.params.items.clone());
    for param in &mut params {
        let variadic_flags = symbols
            .get_type_variadic(&param.ty)
            .cloned()
            .unwrap_or_default();
        param.ty = symbols.normalize_top_level_type(param.ty.clone());
        symbols.record_type_variadic(param.ty.clone(), variadic_flags);
    }
    let normalized_params: Vec<TypeRef> = params.iter().map(|param| param.ty.clone()).collect();
    if symbols.get_function(&name).is_some() {
        let normalized_variadic = params.iter().map(|param| param.is_variadic).collect();
        symbols
            .update_function_signature(&name, normalized_params.clone(), normalized_variadic)
            .map_err(CompileError::from)?;
    }

    let mut nested = Vec::new();
    let mut nested_capture_params = Vec::new();
    let mut env_for_block = outer_env.clone();
    for param in &params {
        env_for_block.insert(
            param.name.clone(),
            EnvEntry {
                ty: param.ty.clone(),
                span: param.span,
                constant: None,
            },
        );
    }
    let env_snapshot = env_for_block.clone();
    let reserved_names = params
        .iter()
        .map(|param| param.name.clone())
        .collect::<HashSet<_>>();

    let body = lower_block(
        ast_lambda.body,
        &mut nested,
        symbols,
        &env_for_block,
        &reserved_names,
        &mut nested_capture_params,
    )?;
    let capture_params = collect_capture_params(&body, &params, outer_env, &nested_capture_params);

    let mut all_params = Vec::new();
    for capture in &capture_params {
        all_params.push(Param {
            name: capture.name.clone(),
            ty: capture.ty.clone(),
            span: capture.span,
            is_variadic: false,
        });
    }
    all_params.extend(params);

    if !capture_params.is_empty() {
        symbols.register_function_capture(name.clone(), capture_params.clone());
    }

    let mut normalized_body = normalize_block(body, symbols);
    let closure_params = closure_param_names(&all_params, symbols);
    insert_release_before_invocation(&mut normalized_body, symbols, &closure_params);

    validate_block_signatures(&normalized_body, &env_snapshot, symbols)?;

    Ok((
        Function {
            name,
            params: all_params,
            body: normalized_body,
            span,
        },
        nested,
    ))
}

pub fn lower_entry(
    mut block_items: Vec<AstItem>,
    entry_items: Vec<AstItem>,
    span: Span,
    symbols: &mut SymbolRegistry,
) -> Result<Option<Vec<AstItem>>, CompileError> {
    for entry_item in entry_items {
        match entry_item {
            item @ (AstItem::Ident(_) | AstItem::Lambda(_) | AstItem::ScopeCapture { .. }) => {
                block_items.push(item);
            }
            _invalid => {
                return Err(ParseError::new("top-level term must be an invocation", span).into());
            }
        }
    }

    let main_sig = symbol::FunctionSig {
        name: ENTRY_FUNCTION_NAME.to_string(),
        params: Vec::new(),
        is_variadic: Vec::new(),
        span,
    };
    symbols.declare_function(main_sig)?;

    let body = AstBlock {
        items: block_items,
        span,
    };

    let entry = AstItem::FunctionDef {
        name: ENTRY_FUNCTION_NAME.to_string(),
        lambda: AstLambda {
            params: AstParams {
                items: Vec::new(),
                span,
            },
            body,
            args: Vec::new(),
            span,
        },
        span,
    };

    Ok(Some(vec![entry]))
}

pub fn normalize_type_alias(name: &str, symbols: &mut SymbolRegistry) -> Result<(), ParseError> {
    if let Some(info) = symbols.get_type_info(name).cloned() {
        let normalized = symbols.normalize_top_level_type(info.target.clone());
        symbols.update_type(name, normalized, info.variadic)?;
    }
    Ok(())
}

fn lower_block(
    block: AstBlock,
    nested: &mut Vec<Function>,
    symbols: &mut SymbolRegistry,
    env: &Env,
    reserved_names: &HashSet<String>,
    nested_captures: &mut Vec<CaptureParam>,
) -> Result<Block, CompileError> {
    let len = block.items.len();
    let mut items = Vec::with_capacity(len);
    let mut alias_map = HashMap::new();
    let mut current_env = env.clone();
    let mut defined_names = reserved_names.clone();
    let mut ast_items = block.items;
    let mut idx = 0;

    while idx < ast_items.len() {
        let stmt = ast_items[idx].clone();
        match stmt {
            AstItem::ScopeCapture { params, term, span } => {
                let body_items = ast_items.split_off(idx + 1);
                let lambda = AstLambda {
                    params,
                    body: AstBlock {
                        items: body_items,
                        span,
                    },
                    args: Vec::new(),
                    span,
                };
                let callback_term = AstTerm::Lambda(lambda);
                let term_span = term.span();
                let invocation_term = append_scope_capture_arg(term, callback_term, term_span)?;
                let lowered_items = lower_invocation_stmt(
                    invocation_term,
                    nested,
                    symbols,
                    &alias_map,
                    &current_env,
                    nested_captures,
                )?;
                items.extend(lowered_items);
                break;
            }
            AstItem::Ident(term) => {
                items.extend(lower_invocation_stmt(
                    AstTerm::Ident(term),
                    nested,
                    symbols,
                    &alias_map,
                    &current_env,
                    nested_captures,
                )?);
            }
            AstItem::Lambda(lambda) => {
                items.extend(lower_invocation_stmt(
                    AstTerm::Lambda(lambda),
                    nested,
                    symbols,
                    &alias_map,
                    &current_env,
                    nested_captures,
                )?);
            }
            AstItem::FunctionDef { name, lambda, span } => {
                enforce_unique_label(&mut defined_names, &name, span)?;
                let ast_item = AstItem::FunctionDef { name, lambda, span };
                let (function, extra) = lower_function(ast_item, symbols, &current_env)?;
                nested.extend(extra);
                nested.push(function.clone());
                items.push(BlockItem::FunctionDef(function));
            }
            AstItem::StrDef {
                name,
                literal,
                span,
            } => {
                enforce_unique_label(&mut defined_names, &name, span)?;
                let name_clone = name.clone();
                let literal_value = literal.value.clone();
                items.push(BlockItem::StrDef(StrLiteral {
                    name: name_clone.clone(),
                    value: literal.value,
                    span,
                }));
                current_env.insert(
                    name_clone,
                    EnvEntry {
                        ty: TypeRef::Str,
                        span,
                        constant: Some(ConstantValue::Str(literal_value)),
                    },
                );
            }
            AstItem::IntDef {
                name,
                literal,
                span,
            } => {
                enforce_unique_label(&mut defined_names, &name, span)?;
                let name_clone = name.clone();
                let literal_value = literal.value;
                items.push(BlockItem::IntDef(IntLiteral {
                    name: name_clone.clone(),
                    value: literal.value,
                    span,
                }));
                current_env.insert(
                    name_clone,
                    EnvEntry {
                        ty: TypeRef::Int,
                        span,
                        constant: Some(ConstantValue::Int(literal_value)),
                    },
                );
            }
            AstItem::IdentDef { name, ident, span } => {
                enforce_unique_label(&mut defined_names, &name, span)?;
                if ident.args.is_empty() {
                    let target = resolve_alias_name(&ident.name, &alias_map);
                    alias_map.insert(name, target);
                    idx += 1;
                    continue;
                }
                let mut lowered_items = Vec::new();
                let apply = lower_ident_as_apply(
                    name,
                    ident,
                    nested,
                    symbols,
                    &mut lowered_items,
                    &alias_map,
                    &current_env,
                    nested_captures,
                )?;
                items.extend(lowered_items);
                items.push(BlockItem::ApplyDef(apply));
            }
            other => unreachable!("unexpected block item: {:?}", other),
        }
        idx += 1;
    }

    Ok(Block {
        items,
        span: block.span,
    })
}

fn append_scope_capture_arg(
    term: AstTerm,
    callback: AstTerm,
    span: Span,
) -> Result<AstTerm, CompileError> {
    match term {
        AstTerm::Ident(mut ident) => {
            ident.args.push(callback);
            Ok(AstTerm::Ident(ident))
        }
        AstTerm::Lambda(mut lambda) => {
            lambda.args.push(callback);
            Ok(AstTerm::Lambda(lambda))
        }
        _other => Err(ParseError::new("scope capture target must be callable", span).into()),
    }
}

fn enforce_unique_label(
    defined: &mut HashSet<String>,
    name: &str,
    span: Span,
) -> Result<(), CompileError> {
    if !defined.insert(name.to_string()) {
        return Err(ParseError::new(
            format!("cannot re-label '{}' in the same scope", name),
            span,
        )
        .into());
    }
    Ok(())
}

fn collect_capture_params(
    block: &Block,
    params: &[Param],
    outer_env: &Env,
    nested_captures: &[CaptureParam],
) -> Vec<CaptureParam> {
    let locals = gather_local_definitions(block, params);
    let mut captures = Vec::new();
    let mut seen = HashSet::new();

    for item in &block.items {
        match item {
            BlockItem::Invocation(Invocation { of, args, .. }) => {
                record_name_capture(of, &locals, outer_env, &mut captures, &mut seen);
                record_arg_captures(args, &locals, outer_env, &mut captures, &mut seen);
            }
            BlockItem::ApplyDef(Apply { of, args, .. }) => {
                record_name_capture(of, &locals, outer_env, &mut captures, &mut seen);
                record_arg_captures(args, &locals, outer_env, &mut captures, &mut seen);
            }
            _ => {}
        }
    }

    for capture in nested_captures {
        record_name_capture(&capture.name, &locals, outer_env, &mut captures, &mut seen);
    }

    captures
}

fn gather_local_definitions(block: &Block, params: &[Param]) -> HashSet<String> {
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
            BlockItem::Invocation(Invocation {
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
    outer_env: &Env,
    captures: &mut Vec<CaptureParam>,
    seen: &mut HashSet<String>,
) {
    for arg in args {
        let name = &arg.name;
        if locals.contains(name) || seen.contains(name) {
            continue;
        }
        if let Some(entry) = outer_env.get(name) {
            captures.push(CaptureParam {
                name: name.clone(),
                ty: entry.ty.clone(),
                span: entry.span,
            });
            seen.insert(name.clone());
        }
    }
}

fn record_name_capture(
    name: &str,
    locals: &HashSet<String>,
    outer_env: &Env,
    captures: &mut Vec<CaptureParam>,
    seen: &mut HashSet<String>,
) {
    if locals.contains(name) || seen.contains(name) {
        return;
    }
    if let Some(entry) = outer_env.get(name) {
        let captured_name = name.to_string();
        captures.push(CaptureParam {
            name: captured_name.clone(),
            ty: entry.ty.clone(),
            span: entry.span,
        });
        seen.insert(captured_name);
    }
}

fn lower_invocation_stmt(
    term: AstTerm,
    nested: &mut Vec<Function>,
    symbols: &mut SymbolRegistry,
    alias_map: &HashMap<String, String>,
    env: &Env,
    nested_captures: &mut Vec<CaptureParam>,
) -> Result<Vec<BlockItem>, CompileError> {
    let mut items = Vec::new();
    let invocation = match term {
        AstTerm::Ident(ident) => lower_ident_as_invocation(
            None,
            ident,
            nested,
            symbols,
            &mut items,
            alias_map,
            env,
            nested_captures,
        )?,
        AstTerm::Lambda(lambda) => lower_lambda_as_invocation(
            None,
            lambda,
            nested,
            symbols,
            &mut items,
            alias_map,
            env,
            nested_captures,
        )?,
        other => unreachable!("expected invocation term, got {:?}", other),
    };
    items.push(BlockItem::Invocation(invocation));
    Ok(items)
}

fn lower_ident_as_invocation(
    result: Option<String>,
    AstIdent { name, args, span }: AstIdent,
    nested: &mut Vec<Function>,
    symbols: &mut SymbolRegistry,
    items: &mut Vec<BlockItem>,
    alias_map: &HashMap<String, String>,
    env: &Env,
    nested_captures: &mut Vec<CaptureParam>,
) -> Result<Invocation, CompileError> {
    let resolved_name = resolve_alias_name(&name, alias_map);
    let signature = resolve_target_signature(&resolved_name, env, symbols);
    let builtin_expectations = builtin_arg_expectations(&resolved_name, args.len());
    let variadic_flags = resolve_variadic_flags(&resolved_name, env, symbols);
    let args = lower_terms_to_args(
        args,
        signature.as_deref(),
        variadic_flags.as_deref(),
        builtin_expectations.as_deref(),
        nested,
        symbols,
        items,
        alias_map,
        env,
        nested_captures,
    )?;
    Ok(Invocation {
        of: resolved_name,
        args,
        span,
        result,
    })
}

fn lower_ident_as_apply(
    name: String,
    AstIdent {
        name: target,
        args,
        span,
    }: AstIdent,
    nested: &mut Vec<Function>,
    symbols: &mut SymbolRegistry,
    items: &mut Vec<BlockItem>,
    alias_map: &HashMap<String, String>,
    env: &Env,
    nested_captures: &mut Vec<CaptureParam>,
) -> Result<Apply, CompileError> {
    let resolved_target = resolve_alias_name(&target, alias_map);
    let signature = resolve_target_signature(&resolved_target, env, symbols);
    let variadic_flags = resolve_variadic_flags(&resolved_target, env, symbols);
    let args = lower_terms_to_args(
        args,
        signature.as_deref(),
        variadic_flags.as_deref(),
        None,
        nested,
        symbols,
        items,
        alias_map,
        env,
        nested_captures,
    )?;
    Ok(Apply {
        name,
        of: resolved_target,
        args,
        span,
    })
}

fn resolve_variadic_flags(target: &str, env: &Env, symbols: &SymbolRegistry) -> Option<Vec<bool>> {
    if let Some(sig) = symbols.get_function(target) {
        return Some(sig.is_variadic.clone());
    }
    env.get(target)
        .and_then(|entry| symbols.get_type_variadic(&entry.ty))
        .cloned()
}

fn lower_lambda_as_invocation(
    result: Option<String>,
    AstLambda {
        params,
        body,
        args,
        span,
    }: AstLambda,
    nested: &mut Vec<Function>,
    symbols: &mut SymbolRegistry,
    items: &mut Vec<BlockItem>,
    alias_map: &HashMap<String, String>,
    env: &Env,
    nested_captures: &mut Vec<CaptureParam>,
) -> Result<Invocation, CompileError> {
    let fn_name = symbols.fresh_lambda_name(span);

    let nested_item = AstItem::FunctionDef {
        name: fn_name.clone(),
        lambda: AstLambda {
            params,
            body,
            args: Vec::new(),
            span,
        },
        span,
    };

    let (lf, extra) = lower_function(nested_item, symbols, env)?;
    symbols.declare_function(FunctionSig {
        name: fn_name.clone(),
        params: lf.params.iter().map(|p| p.ty.clone()).collect(),
        is_variadic: lf.params.iter().map(|p| p.is_variadic).collect(),
        span,
    })?;

    nested.push(lf);
    nested.extend(extra);

    let lowered_args = lower_terms_to_args(
        args,
        None,
        None,
        None,
        nested,
        symbols,
        items,
        alias_map,
        env,
        nested_captures,
    )?;
    Ok(Invocation {
        of: fn_name,
        args: lowered_args,
        span,
        result,
    })
}

fn lower_terms_to_args(
    terms: Vec<AstTerm>,
    expected: Option<&[TypeRef]>,
    _is_variadic: Option<&[bool]>,
    builtin_expectations: Option<&[Option<TypeRef>]>,
    nested: &mut Vec<Function>,
    symbols: &mut SymbolRegistry,
    items: &mut Vec<BlockItem>,
    alias_map: &HashMap<String, String>,
    env: &Env,
    nested_captures: &mut Vec<CaptureParam>,
) -> Result<Vec<Arg>, CompileError> {
    lower_terms_to_args_default(
        terms,
        expected,
        builtin_expectations,
        nested,
        symbols,
        items,
        alias_map,
        env,
        nested_captures,
    )
}

fn lower_terms_to_args_default(
    terms: Vec<AstTerm>,
    expected: Option<&[TypeRef]>,
    builtin_expectations: Option<&[Option<TypeRef>]>,
    nested: &mut Vec<Function>,
    symbols: &mut SymbolRegistry,
    items: &mut Vec<BlockItem>,
    alias_map: &HashMap<String, String>,
    env: &Env,
    nested_captures: &mut Vec<CaptureParam>,
) -> Result<Vec<Arg>, CompileError> {
    terms
        .into_iter()
        .enumerate()
        .map(|(idx, term)| {
            let expected_ty = expected.and_then(|types| types.get(idx));
            let builtin_expected_ty = builtin_expectations
                .and_then(|expect| expect.get(idx))
                .and_then(|opt| opt.as_ref());
            let term = wrap_builtin_invocation_in_lambda(
                term,
                expected_ty.or(builtin_expected_ty),
                symbols,
            );
            lower_term_to_arg(
                term,
                nested,
                symbols,
                items,
                alias_map,
                env,
                nested_captures,
            )
        })
        .collect()
}

fn lower_term_to_arg(
    term: AstTerm,
    nested: &mut Vec<Function>,
    symbols: &mut SymbolRegistry,
    items: &mut Vec<BlockItem>,
    alias_map: &HashMap<String, String>,
    env: &Env,
    nested_captures: &mut Vec<CaptureParam>,
) -> Result<Arg, CompileError> {
    match term {
        AstTerm::Ident(ast_ident) => {
            if ast_ident.args.is_empty() {
                return Ok(Arg {
                    name: resolve_alias_name(&ast_ident.name, alias_map),
                    span: ast_ident.span,
                });
            }
            let args = lower_terms_to_args(
                ast_ident.args,
                None,
                None,
                None,
                nested,
                symbols,
                items,
                alias_map,
                env,
                nested_captures,
            )?;
            let temp_name = symbols.fresh_temp_name();
            items.push(BlockItem::Invocation(Invocation {
                of: resolve_alias_name(&ast_ident.name, alias_map),
                args,
                span: ast_ident.span,
                result: Some(temp_name.clone()),
            }));
            Ok(Arg {
                name: temp_name,
                span: ast_ident.span,
            })
        }
        AstTerm::Int(literal) => {
            let temp_name = symbols.fresh_temp_name();
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
        AstTerm::String(literal) => {
            let temp_name = symbols.fresh_temp_name();
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
        AstTerm::Lambda(lambda) => {
            let temp_name = symbols.fresh_temp_name();
            let span = lambda.span;
            let invocation = lower_lambda_as_invocation(
                Some(temp_name.clone()),
                lambda,
                nested,
                symbols,
                items,
                alias_map,
                env,
                nested_captures,
            )?;
            if let Some(captures) = symbols.function_captures(&invocation.of) {
                nested_captures.extend(captures.iter().cloned());
            }
            items.push(BlockItem::Invocation(invocation));
            Ok(Arg {
                name: temp_name,
                span,
            })
        }
    }
}

fn wrap_builtin_invocation_in_lambda(
    term: AstTerm,
    expected: Option<&TypeRef>,
    symbols: &SymbolRegistry,
) -> AstTerm {
    if expected_is_function(expected, symbols) {
        if let AstTerm::Ident(ast_ident) = term {
            if symbols.builtin_imports().contains(&ast_ident.name) {
                let span = ast_ident.span;
                let block = AstBlock {
                    span,
                    items: vec![AstItem::Ident(ast_ident)],
                };
                return AstTerm::Lambda(AstLambda {
                    params: AstParams {
                        items: Vec::new(),
                        span,
                    },
                    body: block,
                    args: Vec::new(),
                    span,
                });
            }
            return AstTerm::Ident(ast_ident);
        }
    }
    term
}

fn builtin_arg_expectations(name: &str, arg_count: usize) -> Option<Vec<Option<TypeRef>>> {
    if name == "printf" && arg_count > 0 {
        let mut expectations = vec![None; arg_count];
        expectations[arg_count - 1] = Some(TypeRef::Type(Vec::new()));
        return Some(expectations);
    }
    None
}

fn expected_is_function(expected: Option<&TypeRef>, symbols: &SymbolRegistry) -> bool {
    expected
        .and_then(|ty| resolve_type_signature(ty, symbols))
        .is_some()
}

fn resolve_alias_name<'a>(name: &'a str, alias_map: &'a HashMap<String, String>) -> String {
    let mut current: &'a str = name;
    let mut seen = HashSet::new();
    while seen.insert(current) {
        if let Some(next) = alias_map.get(current) {
            current = next;
        } else {
            break;
        }
    }
    current.to_string()
}

fn normalize_block(mut block: Block, symbols: &mut SymbolRegistry) -> Block {
    let mut items = rewrite_block_captures(block.items, symbols);

    loop {
        let use_sites = compute_use_sites(&items);

        if let Some((parent_idx, child_idx)) = find_apply_to_apply(&items, &use_sites) {
            inline_apply_into_apply(&mut items, parent_idx, child_idx);
            continue;
        }

        if let Some((parent_idx, inv_idx)) = find_apply_to_invocation(&items, &use_sites) {
            inline_apply_into_invocation(&mut items, parent_idx, inv_idx);
            continue;
        }

        break;
    }

    block.items = items;
    block
}

fn validate_block_signatures(
    block: &Block,
    env: &Env,
    symbols: &SymbolRegistry,
) -> Result<(), CompileError> {
    let mut type_env = env.clone();

    for item in &block.items {
        match item {
            BlockItem::StrDef(StrLiteral { name, value, span }) => {
                type_env.insert(
                    name.clone(),
                    EnvEntry {
                        ty: TypeRef::Str,
                        span: *span,
                        constant: Some(ConstantValue::Str(value.clone())),
                    },
                );
            }
            BlockItem::IntDef(IntLiteral { name, value, span }) => {
                type_env.insert(
                    name.clone(),
                    EnvEntry {
                        ty: TypeRef::Int,
                        span: *span,
                        constant: Some(ConstantValue::Int(*value)),
                    },
                );
            }
            BlockItem::FunctionDef(function) => {
                if let Some(sig) = symbols.get_function(&function.name) {
                    type_env.insert(
                        function.name.clone(),
                        EnvEntry {
                            ty: TypeRef::Type(sig.params.clone()),
                            span: function.span,
                            constant: None,
                        },
                    );
                }
            }
            BlockItem::ApplyDef(apply) => {
                if let Some(ty) = validate_apply(apply, &type_env, symbols)? {
                    type_env.insert(
                        apply.name.clone(),
                        EnvEntry {
                            ty,
                            span: apply.span,
                            constant: None,
                        },
                    );
                }
            }
            BlockItem::Invocation(invocation) => {
                if let Some(sig) = validate_invocation(invocation, &type_env, symbols)? {
                    if let Some(result) = &invocation.result {
                        let remaining = sig.iter().skip(invocation.args.len()).cloned().collect();
                        type_env.insert(
                            result.clone(),
                            EnvEntry {
                                ty: TypeRef::Type(remaining),
                                span: invocation.span,
                                constant: None,
                            },
                        );
                    }
                }
            }
            BlockItem::ReleaseEnv(_) => {}
        }
    }

    Ok(())
}

fn insert_release_before_invocation(
    block: &mut Block,
    symbols: &SymbolRegistry,
    closure_params: &[String],
) {
    let last_tail_call_idx = block
        .items
        .iter()
        .rposition(|item| matches!(item, BlockItem::Invocation(inv) if inv.result.is_none()));
    let items: Vec<_> = std::mem::take(&mut block.items);
    let captured_params = captured_closure_params(&items, closure_params);
    let mut rewritten = Vec::with_capacity(items.len());

    for (idx, mut item) in items.into_iter().enumerate() {
        if let BlockItem::FunctionDef(function) = &mut item {
            let nested_closures = closure_param_names(&function.params, symbols);
            insert_release_before_invocation(&mut function.body, symbols, &nested_closures);
        }

        if Some(idx) == last_tail_call_idx {
            if let BlockItem::Invocation(invocation) = &item {
                for param in closure_params {
                    if captured_params.contains(param) {
                        continue;
                    }
                    if should_release_closure_param(param, invocation) {
                        rewritten.push(BlockItem::ReleaseEnv(ReleaseEnv {
                            name: param.clone(),
                            span: invocation.span,
                        }));
                    }
                }
            }
        }

        rewritten.push(item);
    }

    block.items = rewritten;
}

fn captured_closure_params(items: &[BlockItem], closure_params: &[String]) -> HashSet<String> {
    let closure_names: HashSet<&str> = closure_params.iter().map(|name| name.as_str()).collect();
    let mut captured = HashSet::new();

    for item in items {
        match item {
            BlockItem::ApplyDef(Apply { of, args, .. }) => {
                if closure_names.contains(of.as_str()) {
                    captured.insert(of.clone());
                }
                for arg in args {
                    if closure_names.contains(arg.name.as_str()) {
                        captured.insert(arg.name.clone());
                    }
                }
            }
            BlockItem::Invocation(invocation) => {
                if closure_names.contains(invocation.of.as_str()) {
                    captured.insert(invocation.of.clone());
                }
                for arg in &invocation.args {
                    if closure_names.contains(arg.name.as_str()) {
                        captured.insert(arg.name.clone());
                    }
                }
            }
            _ => {}
        }
    }

    captured
}

fn closure_param_names(params: &[Param], symbols: &SymbolRegistry) -> Vec<String> {
    params
        .iter()
        .filter(|param| is_closure_param(&param.ty, symbols))
        .map(|param| param.name.clone())
        .collect()
}

fn should_release_closure_param(param_name: &str, invocation: &Invocation) -> bool {
    if invocation.of == param_name {
        return false;
    }
    !invocation.args.iter().any(|arg| arg.name == param_name)
}

fn is_closure_param(ty: &TypeRef, symbols: &SymbolRegistry) -> bool {
    let mut visited = HashSet::new();
    is_closure_type(ty, symbols, &mut visited)
}

fn is_closure_type(ty: &TypeRef, symbols: &SymbolRegistry, visited: &mut HashSet<String>) -> bool {
    match ty {
        TypeRef::Int | TypeRef::Str | TypeRef::CompileTimeInt | TypeRef::CompileTimeStr => false,
        TypeRef::Type(_) => true,
        TypeRef::Alias(name) => {
            if visited.contains(name) {
                return true;
            }
            if let Some(info) = symbols.get_type_info(name) {
                visited.insert(name.clone());
                let result = is_closure_type(&info.target, symbols, visited);
                visited.remove(name);
                return result;
            }
            false
        }
    }
}

fn validate_apply(
    apply: &Apply,
    type_env: &Env,
    symbols: &SymbolRegistry,
) -> Result<Option<TypeRef>, CompileError> {
    if let Some(sig) = resolve_target_signature(&apply.of, type_env, symbols) {
        if apply.args.len() > sig.len() {
            return Err(ParseError::new(
                format!(
                    "function '{}' expects at most {} arguments but got {}",
                    apply.of,
                    sig.len(),
                    apply.args.len()
                ),
                apply.span,
            )
            .into());
        }
        for (idx, (arg, param_ty)) in apply.args.iter().zip(sig.iter()).enumerate() {
            let arg_ty = lookup_arg_type(arg, type_env, symbols).ok_or_else(|| {
                ParseError::new(
                    format!("unknown type for argument {} to '{}'", idx + 1, apply.of),
                    arg.span,
                )
            })?;
            if !type_satisfies(param_ty, &arg_ty, symbols) {
                return Err(ParseError::new(
                    format!(
                        "argument {} to '{}' has type {} but expected {}",
                        idx + 1,
                        apply.of,
                        format_type_ref(&arg_ty),
                        format_type_ref(param_ty),
                    ),
                    arg.span,
                )
                .into());
            }
            ensure_compile_time_requirement(
                param_ty, arg, &arg_ty, type_env, symbols, &apply.of, idx,
            )?;
        }
        let remaining = sig.iter().skip(apply.args.len()).cloned().collect();
        Ok(Some(TypeRef::Type(remaining)))
    } else {
        Ok(None)
    }
}

fn validate_invocation(
    invocation: &Invocation,
    type_env: &Env,
    symbols: &SymbolRegistry,
) -> Result<Option<Vec<TypeRef>>, CompileError> {
    if let Some(sig) = resolve_target_signature(&invocation.of, type_env, symbols) {
        if invocation.result.is_some() {
            if invocation.args.len() > sig.len() {
                return Err(ParseError::new(
                    format!(
                        "function '{}' expects at most {} arguments but got {}",
                        invocation.of,
                        sig.len(),
                        invocation.args.len(),
                    ),
                    invocation.span,
                )
                .into());
            }
        } else if invocation.args.len() != sig.len() {
            return Err(ParseError::new(
                format!(
                    "function '{}' expects {} arguments but got {}",
                    invocation.of,
                    sig.len(),
                    invocation.args.len(),
                ),
                invocation.span,
            )
            .into());
        }

        for (idx, (arg, param_ty)) in invocation.args.iter().zip(sig.iter()).enumerate() {
            let arg_ty = lookup_arg_type(arg, type_env, symbols).ok_or_else(|| {
                ParseError::new(
                    format!(
                        "unknown type for argument {} to '{}'",
                        idx + 1,
                        invocation.of
                    ),
                    arg.span,
                )
            })?;
            if !type_satisfies(param_ty, &arg_ty, symbols) {
                return Err(ParseError::new(
                    format!(
                        "argument {} to '{}' has type {} but expected {}",
                        idx + 1,
                        invocation.of,
                        format_type_ref(&arg_ty),
                        format_type_ref(param_ty),
                    ),
                    arg.span,
                )
                .into());
            }
            ensure_compile_time_requirement(
                param_ty,
                arg,
                &arg_ty,
                type_env,
                symbols,
                &invocation.of,
                idx,
            )?;
        }

        return Ok(Some(sig));
    }
    Ok(None)
}

fn lookup_arg_type(arg: &Arg, type_env: &Env, symbols: &SymbolRegistry) -> Option<TypeRef> {
    if let Some(entry) = type_env.get(&arg.name) {
        return Some(entry.ty.clone());
    }
    symbols
        .get_function(&arg.name)
        .map(|sig| TypeRef::Type(sig.params.clone()))
}

fn resolve_target_signature(
    target: &str,
    type_env: &Env,
    symbols: &SymbolRegistry,
) -> Option<Vec<TypeRef>> {
    if let Some(sig) = symbols.get_function(target) {
        return Some(sig.params.clone());
    }
    if let Some(entry) = type_env.get(target) {
        if let Some(params) = resolve_type_signature(&entry.ty, symbols) {
            return Some(params);
        }
    }
    None
}

fn resolve_type_signature(ty: &TypeRef, symbols: &SymbolRegistry) -> Option<Vec<TypeRef>> {
    let mut visited = HashSet::new();
    resolve_alias_signature(ty, symbols, &mut visited)
}

fn resolve_alias_signature(
    ty: &TypeRef,
    symbols: &SymbolRegistry,
    visited: &mut HashSet<String>,
) -> Option<Vec<TypeRef>> {
    match ty {
        TypeRef::Type(params) => Some(params.clone()),
        TypeRef::Alias(name) => {
            if visited.contains(name) {
                return None;
            }
            if let Some(info) = symbols.get_type_info(name) {
                visited.insert(name.clone());
                let result = resolve_alias_signature(&info.target, symbols, visited);
                visited.remove(name);
                return result;
            }
            None
        }
        _ => None,
    }
}

fn format_type_ref(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Int => "int".to_string(),
        TypeRef::Str => "str".to_string(),
        TypeRef::CompileTimeInt => "int!".to_string(),
        TypeRef::CompileTimeStr => "str!".to_string(),
        TypeRef::Alias(name) => name.clone(),
        TypeRef::Type(inner) => format!(
            "({})",
            inner
                .iter()
                .map(|ty| format_type_ref(ty))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn type_satisfies(expected: &TypeRef, actual: &TypeRef, symbols: &SymbolRegistry) -> bool {
    let mut visited = HashSet::new();
    type_satisfies_inner(expected, actual, symbols, &mut visited)
}

fn type_satisfies_inner(
    expected: &TypeRef,
    actual: &TypeRef,
    symbols: &SymbolRegistry,
    visited: &mut HashSet<String>,
) -> bool {
    if expected == actual {
        return true;
    }
    match (expected, actual) {
        (TypeRef::CompileTimeStr, TypeRef::Str)
        | (TypeRef::Str, TypeRef::CompileTimeStr)
        | (TypeRef::CompileTimeInt, TypeRef::Int)
        | (TypeRef::Int, TypeRef::CompileTimeInt) => {
            return true;
        }
        _ => {}
    }
    match (expected, actual) {
        (TypeRef::Int, TypeRef::Int) | (TypeRef::Str, TypeRef::Str) => true,
        (TypeRef::Alias(name), TypeRef::Alias(other)) => name == other,
        (TypeRef::Alias(name), other) => {
            if visited.contains(name) {
                return false;
            }
            if let Some(info) = symbols.get_type_info(name) {
                visited.insert(name.clone());
                let result = type_satisfies_inner(&info.target, other, symbols, visited);
                visited.remove(name);
                return result;
            }
            false
        }
        (other, TypeRef::Alias(name)) => {
            if visited.contains(name) {
                return false;
            }
            if let Some(info) = symbols.get_type_info(name) {
                visited.insert(name.clone());
                let result = type_satisfies_inner(other, &info.target, symbols, visited);
                visited.remove(name);
                return result;
            }
            false
        }
        (TypeRef::Type(expected_params), TypeRef::Type(actual_params)) => {
            let expected_trimmed = trim_unit_continuation(expected_params, symbols);
            let actual_trimmed = trim_unit_continuation(actual_params, symbols);
            if expected_trimmed.len() != actual_trimmed.len() {
                return false;
            }
            expected_trimmed
                .iter()
                .zip(actual_trimmed.iter())
                .all(|(expected, actual)| type_satisfies_inner(expected, actual, symbols, visited))
        }
        _ => false,
    }
}

#[derive(Copy, Clone, PartialEq)]
enum CompileTimeRequirement {
    Str,
    Int,
}

fn compile_time_requirement(
    ty: &TypeRef,
    symbols: &SymbolRegistry,
) -> Option<CompileTimeRequirement> {
    let mut visited = HashSet::new();
    compile_time_requirement_inner(ty, symbols, &mut visited)
}

fn compile_time_requirement_inner(
    ty: &TypeRef,
    symbols: &SymbolRegistry,
    visited: &mut HashSet<String>,
) -> Option<CompileTimeRequirement> {
    match ty {
        TypeRef::CompileTimeStr => Some(CompileTimeRequirement::Str),
        TypeRef::CompileTimeInt => Some(CompileTimeRequirement::Int),
        TypeRef::Alias(name) => {
            if visited.contains(name) {
                None
            } else if let Some(info) = symbols.get_type_info(name) {
                visited.insert(name.clone());
                let result = compile_time_requirement_inner(&info.target, symbols, visited);
                visited.remove(name);
                result
            } else {
                None
            }
        }
        _ => None,
    }
}

fn ensure_compile_time_requirement(
    param_ty: &TypeRef,
    arg: &Arg,
    arg_ty: &TypeRef,
    type_env: &Env,
    symbols: &SymbolRegistry,
    target: &str,
    arg_index: usize,
) -> Result<(), CompileError> {
    if let Some(requirement) = compile_time_requirement(param_ty, symbols) {
        let satisfied = type_env
            .get(&arg.name)
            .and_then(|entry| entry.constant.as_ref())
            .map(|constant| match (requirement, constant) {
                (CompileTimeRequirement::Str, ConstantValue::Str(_)) => true,
                (CompileTimeRequirement::Int, ConstantValue::Int(_)) => true,
                _ => false,
            })
            .unwrap_or(false);
        let type_satisfied = compile_time_requirement(arg_ty, symbols) == Some(requirement);
        if satisfied || type_satisfied {
            Ok(())
        } else {
            Err(ParseError::new(
                format!(
                    "argument {} to '{}' must be {}",
                    arg_index + 1,
                    target,
                    format_type_ref(param_ty),
                ),
                arg.span,
            )
            .into())
        }
    } else {
        Ok(())
    }
}

fn trim_unit_continuation<'a>(params: &'a [TypeRef], symbols: &SymbolRegistry) -> &'a [TypeRef] {
    if params.is_empty() {
        return params;
    }
    let mut visited = HashSet::new();
    if is_unit_type(params.last().unwrap(), symbols, &mut visited) {
        return &params[..params.len() - 1];
    }
    params
}

fn is_unit_type(ty: &TypeRef, symbols: &SymbolRegistry, visited: &mut HashSet<String>) -> bool {
    match ty {
        TypeRef::Type(inner) => inner.is_empty(),
        TypeRef::Alias(name) => {
            if visited.contains(name) {
                return false;
            }
            if let Some(info) = symbols.get_type_info(name) {
                visited.insert(name.clone());
                let result = is_unit_type(&info.target, symbols, visited);
                visited.remove(name);
                return result;
            }
            false
        }
        _ => false,
    }
}

fn rewrite_block_captures(items: Vec<BlockItem>, symbols: &mut SymbolRegistry) -> Vec<BlockItem> {
    let mut rewritten = Vec::new();

    for mut item in items {
        if let BlockItem::FunctionDef(function) = &mut item {
            function.body = Block {
                items: rewrite_block_captures(function.body.items.clone(), symbols),
                span: function.body.span,
            };
        }

        let mut prefix = Vec::new();
        match &mut item {
            BlockItem::Invocation(invocation) => {
                rewrite_args(&mut invocation.args, &mut prefix, symbols, invocation.span);
                rewrite_function_target(&mut invocation.of, &mut prefix, symbols, invocation.span);
            }
            BlockItem::ApplyDef(apply) => {
                rewrite_args(&mut apply.args, &mut prefix, symbols, apply.span);
                rewrite_function_target(&mut apply.of, &mut prefix, symbols, apply.span);
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
    symbols: &mut SymbolRegistry,
    span: Span,
) {
    for arg in args.iter_mut() {
        rewrite_function_target(&mut arg.name, prefix, symbols, span);
    }
}

fn rewrite_function_target(
    target: &mut String,
    prefix: &mut Vec<BlockItem>,
    symbols: &mut SymbolRegistry,
    span: Span,
) {
    if let Some(captures) = symbols.function_captures(target) {
        let capture_defs = captures.to_vec();
        let temp_name = symbols.fresh_temp_name();
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
                        .push(UseSite::InvocationArg);
                }
            }
            BlockItem::Invocation(Invocation { of, args, .. }) => {
                map.entry(of.clone())
                    .or_insert_with(Vec::new)
                    .push(UseSite::InvocationOf(idx));
                for arg in args {
                    map.entry(arg.name.clone())
                        .or_insert_with(Vec::new)
                        .push(UseSite::InvocationArg);
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

fn find_apply_to_invocation(
    items: &[BlockItem],
    use_sites: &HashMap<String, Vec<UseSite>>,
) -> Option<(usize, usize)> {
    for (idx, item) in items.iter().enumerate() {
        if let BlockItem::ApplyDef(Apply { name, .. }) = item {
            if let Some(sites) = use_sites.get(name) {
                if sites.len() == 1 {
                    if let UseSite::InvocationOf(inv_idx) = sites[0] {
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

fn inline_apply_into_invocation(items: &mut Vec<BlockItem>, parent_idx: usize, inv_idx: usize) {
    let parent = match items[parent_idx].clone() {
        BlockItem::ApplyDef(apply) => apply,
        _ => return,
    };

    let invocation = match items[inv_idx].clone() {
        BlockItem::Invocation(inv) => inv,
        _ => return,
    };

    let mut merged_args = parent.args.clone();
    merged_args.extend(invocation.args.clone());

    let merged_invocation = Invocation {
        of: parent.of.clone(),
        args: merged_args,
        span: invocation.span,
        result: invocation.result.clone(),
    };

    let inv_idx = if parent_idx < inv_idx {
        items.remove(parent_idx);
        inv_idx - 1
    } else {
        items.remove(parent_idx);
        inv_idx
    };

    items[inv_idx] = BlockItem::Invocation(merged_invocation);
}

#[derive(Copy, Clone)]
enum UseSite {
    ApplyOf(usize),
    InvocationOf(usize),
    InvocationArg,
}
