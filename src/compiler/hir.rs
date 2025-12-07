use crate::compiler::ast::{
    Block as AstBlock, Ident as AstIdent, Item as AstItem, Lambda as AstLambda, Param as AstParam,
    Params as AstParams, Term as AstTerm, TypeRef,
};
use crate::compiler::error::{CompileError, ParseError};
use crate::compiler::span::Span;
use crate::compiler::symbol::{CaptureParam, FunctionSig, SymbolRegistry};
use crate::compiler::type_utils::expand_alias_chain;
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
    Exec(Exec),
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
            | BlockItem::Exec(Exec { span, .. }) => *span,
        }
    }

    pub fn binding_info(&self) -> Option<(&String, Span)> {
        match self {
            BlockItem::FunctionDef(function) => Some((&function.name, function.span)),
            BlockItem::StrDef(literal) => Some((&literal.name, literal.span)),
            BlockItem::IntDef(literal) => Some((&literal.name, literal.span)),
            BlockItem::Exec(Exec {
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
pub struct Exec {
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
        let mut expand_visited = HashSet::new();
        param.ty = expand_alias_chain(&param.ty, symbols, &mut expand_visited);
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
    let normalized_body = normalize_block(body, symbols);
    let capture_params =
        collect_capture_params(&normalized_body, &params, outer_env, &nested_capture_params);

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
                let exec_term = append_scope_capture_arg(term, callback_term, term_span)?;
                let lowered_items = lower_exec_stmt(
                    exec_term,
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
                items.extend(lower_exec_stmt(
                    AstTerm::Ident(term),
                    nested,
                    symbols,
                    &alias_map,
                    &current_env,
                    nested_captures,
                )?);
            }
            AstItem::Lambda(lambda) => {
                items.extend(lower_exec_stmt(
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
                    name.clone(),
                    ident,
                    nested,
                    symbols,
                    &mut lowered_items,
                    &alias_map,
                    &current_env,
                    nested_captures,
                )?;
                let result_type = if let Some((sig, _)) =
                    resolve_target_signature(&apply.of, &current_env, symbols)
                {
                    let remaining = sig
                        .iter()
                        .skip(apply.args.len())
                        .cloned()
                        .collect::<Vec<_>>();
                    TypeRef::Type(remaining)
                } else {
                    TypeRef::Type(Vec::new())
                };
                items.extend(lowered_items);
                items.push(BlockItem::ApplyDef(apply));
                current_env.insert(
                    name.clone(),
                    EnvEntry {
                        ty: result_type,
                        span,
                        constant: None,
                    },
                );
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
            BlockItem::Exec(Exec { of, args, .. }) => {
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

fn lower_exec_stmt(
    term: AstTerm,
    nested: &mut Vec<Function>,
    symbols: &mut SymbolRegistry,
    alias_map: &HashMap<String, String>,
    env: &Env,
    nested_captures: &mut Vec<CaptureParam>,
) -> Result<Vec<BlockItem>, CompileError> {
    let mut items = Vec::new();
    let exec = match term {
        AstTerm::Ident(ident) => lower_ident_as_exec(
            None,
            ident,
            nested,
            symbols,
            &mut items,
            alias_map,
            env,
            nested_captures,
        )?,
        AstTerm::Lambda(lambda) => lower_lambda_as_exec(
            None,
            lambda,
            nested,
            symbols,
            &mut items,
            alias_map,
            env,
            nested_captures,
        )?,
        other => unreachable!("expected exec term, got {:?}", other),
    };
    items.push(BlockItem::Exec(exec));
    Ok(items)
}

fn lower_ident_as_exec(
    result: Option<String>,
    AstIdent { name, args, span }: AstIdent,
    nested: &mut Vec<Function>,
    symbols: &mut SymbolRegistry,
    items: &mut Vec<BlockItem>,
    alias_map: &HashMap<String, String>,
    env: &Env,
    nested_captures: &mut Vec<CaptureParam>,
) -> Result<Exec, CompileError> {
    let resolved_name = resolve_alias_name(&name, alias_map);
    let signature = resolve_target_signature(&resolved_name, env, symbols);
    let builtin_expectations = builtin_arg_expectations(&resolved_name, args.len());
    let args = lower_terms_to_args(
        args,
        signature.as_ref().map(|(params, _)| params.as_slice()),
        signature.as_ref().map(|(_, flags)| flags.as_slice()),
        builtin_expectations.as_deref(),
        nested,
        symbols,
        items,
        alias_map,
        env,
        nested_captures,
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
    let args = lower_terms_to_args(
        args,
        signature.as_ref().map(|(params, _)| params.as_slice()),
        signature.as_ref().map(|(_, flags)| flags.as_slice()),
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

fn lower_lambda_as_exec(
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
) -> Result<Exec, CompileError> {
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
    Ok(Exec {
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
            let term =
                wrap_builtin_exec_in_lambda(term, expected_ty.or(builtin_expected_ty), symbols);
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
            items.push(BlockItem::Exec(Exec {
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
            let exec = lower_lambda_as_exec(
                Some(temp_name.clone()),
                lambda,
                nested,
                symbols,
                items,
                alias_map,
                env,
                nested_captures,
            )?;
            if let Some(captures) = symbols.function_captures(&exec.of) {
                nested_captures.extend(captures.iter().cloned());
            }
            items.push(BlockItem::Exec(exec));
            Ok(Arg {
                name: temp_name,
                span,
            })
        }
    }
}

fn wrap_builtin_exec_in_lambda(
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

        if let Some((parent_idx, inv_idx)) = find_apply_to_exec(&items, &use_sites) {
            inline_apply_into_exec(&mut items, parent_idx, inv_idx);
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
            BlockItem::Exec(exec) => {
                if let Some(remaining) = validate_exec(exec, &type_env, symbols)? {
                    if let Some(result) = &exec.result {
                        type_env.insert(
                            result.clone(),
                            EnvEntry {
                                ty: TypeRef::Type(remaining),
                                span: exec.span,
                                constant: None,
                            },
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

fn validate_apply(
    apply: &Apply,
    type_env: &Env,
    symbols: &SymbolRegistry,
) -> Result<Option<TypeRef>, CompileError> {
    if let Some((sig, variadic)) = resolve_target_signature(&apply.of, type_env, symbols) {
        let remaining = match_arguments_to_signature(
            &apply.args,
            type_env,
            symbols,
            &sig,
            &variadic,
            &apply.of,
            apply.span,
            true,
        )?;
        Ok(Some(TypeRef::Type(remaining)))
    } else {
        Ok(None)
    }
}

fn validate_exec(
    exec: &Exec,
    type_env: &Env,
    symbols: &SymbolRegistry,
) -> Result<Option<Vec<TypeRef>>, CompileError> {
    if let Some((sig, variadic)) = resolve_target_signature(&exec.of, type_env, symbols) {
        let remaining = match_arguments_to_signature(
            &exec.args,
            type_env,
            symbols,
            &sig,
            &variadic,
            &exec.of,
            exec.span,
            exec.result.is_some(),
        )?;
        return Ok(Some(remaining));
    }
    Ok(None)
}

fn match_arguments_to_signature(
    args: &[Arg],
    type_env: &Env,
    symbols: &SymbolRegistry,
    sig: &[TypeRef],
    variadic: &[bool],
    caller: &str,
    span: Span,
    allow_partial: bool,
) -> Result<Vec<TypeRef>, CompileError> {
    if allow_partial {
        return match_arguments_partial(args, type_env, symbols, sig, variadic, caller, span);
    }
    match_arguments_full(args, type_env, symbols, sig, variadic, caller, span)
}

fn match_arguments_partial(
    args: &[Arg],
    type_env: &Env,
    symbols: &SymbolRegistry,
    sig: &[TypeRef],
    variadic: &[bool],
    caller: &str,
    span: Span,
) -> Result<Vec<TypeRef>, CompileError> {
    let mut sig_idx = 0;
    let mut arg_idx = 0;

    while sig_idx < sig.len() {
        if *variadic.get(sig_idx).unwrap_or(&false) {
            let element_ty =
                element_type_for_variadic(&sig[sig_idx], symbols).ok_or_else(|| {
                    ParseError::new(
                        format!(
                            "cannot determine element type for variadic parameter of '{}'",
                            caller
                        ),
                        span,
                    )
                })?;
            while arg_idx < args.len() {
                let arg = &args[arg_idx];
                let arg_ty = lookup_arg_type(arg, type_env, symbols).ok_or_else(|| {
                    ParseError::new(
                        format!("unknown type for argument {} to '{}'", arg_idx + 1, caller),
                        arg.span,
                    )
                })?;
                if !type_satisfies(&element_ty, &arg_ty, symbols) {
                    return Err(type_mismatch_error(
                        caller,
                        arg_idx + 1,
                        &element_ty,
                        &arg_ty,
                        arg.span,
                        symbols,
                    ));
                }
                ensure_compile_time_requirement(
                    &element_ty,
                    arg,
                    &arg_ty,
                    type_env,
                    symbols,
                    caller,
                    arg_idx,
                )?;
                arg_idx += 1;
            }
            sig_idx += 1;
            continue;
        }

        if arg_idx >= args.len() {
            break;
        }

        let arg = &args[arg_idx];
        let arg_ty = lookup_arg_type(arg, type_env, symbols).ok_or_else(|| {
            ParseError::new(
                format!("unknown type for argument {} to '{}'", arg_idx + 1, caller),
                arg.span,
            )
        })?;
        if !type_satisfies(&sig[sig_idx], &arg_ty, symbols) {
            return Err(type_mismatch_error(
                caller,
                arg_idx + 1,
                &sig[sig_idx],
                &arg_ty,
                arg.span,
                symbols,
            ));
        }
        ensure_compile_time_requirement(
            &sig[sig_idx],
            arg,
            &arg_ty,
            type_env,
            symbols,
            caller,
            arg_idx,
        )?;
        sig_idx += 1;
        arg_idx += 1;
    }

    if arg_idx < args.len() {
        return Err(ParseError::new(
            format!(
                "function '{}' expects {} arguments but got {}",
                caller,
                sig.len(),
                args.len()
            ),
            span,
        )
        .into());
    }

    Ok(sig[sig_idx..].to_vec())
}

fn match_arguments_full(
    args: &[Arg],
    type_env: &Env,
    symbols: &SymbolRegistry,
    sig: &[TypeRef],
    variadic: &[bool],
    caller: &str,
    span: Span,
) -> Result<Vec<TypeRef>, CompileError> {
    let mut variadic_index = None;
    for (idx, flag) in variadic.iter().enumerate() {
        if *flag {
            if variadic_index.is_some() {
                return Err(ParseError::new(
                    format!(
                        "multiple variadic parameters are not supported for '{}'",
                        caller
                    ),
                    span,
                )
                .into());
            }
            variadic_index = Some(idx);
        }
    }

    if let Some(variadic_idx) = variadic_index {
        return match_arguments_full_variadic(
            args,
            type_env,
            symbols,
            sig,
            variadic_idx,
            caller,
            span,
        );
    }

    if args.len() != sig.len() {
        return Err(ParseError::new(
            format!(
                "function '{}' expects {} arguments but got {}",
                caller,
                sig.len(),
                args.len()
            ),
            span,
        )
        .into());
    }

    for (idx, expected) in sig.iter().enumerate() {
        let arg = &args[idx];
        let arg_ty = lookup_arg_type(arg, type_env, symbols).ok_or_else(|| {
            ParseError::new(
                format!("unknown type for argument {} to '{}'", idx + 1, caller),
                arg.span,
            )
        })?;
        if !type_satisfies(expected, &arg_ty, symbols) {
            return Err(type_mismatch_error(
                caller,
                idx + 1,
                expected,
                &arg_ty,
                arg.span,
                symbols,
            ));
        }
        ensure_compile_time_requirement(expected, arg, &arg_ty, type_env, symbols, caller, idx)?;
    }

    Ok(Vec::new())
}

fn match_arguments_full_variadic(
    args: &[Arg],
    type_env: &Env,
    symbols: &SymbolRegistry,
    sig: &[TypeRef],
    variadic_idx: usize,
    caller: &str,
    span: Span,
) -> Result<Vec<TypeRef>, CompileError> {
    let prefix_required = variadic_idx;
    let suffix_required = sig.len().saturating_sub(variadic_idx + 1);
    let required = prefix_required + suffix_required;
    if args.len() < required {
        return Err(ParseError::new(
            format!(
                "function '{}' expected at least {} arguments but got {}",
                caller,
                required,
                args.len()
            ),
            span,
        )
        .into());
    }

    for idx in 0..prefix_required {
        let arg = &args[idx];
        let arg_ty = lookup_arg_type(arg, type_env, symbols).ok_or_else(|| {
            ParseError::new(
                format!("unknown type for argument {} to '{}'", idx + 1, caller),
                arg.span,
            )
        })?;
        if !type_satisfies(&sig[idx], &arg_ty, symbols) {
            return Err(type_mismatch_error(
                caller,
                idx + 1,
                &sig[idx],
                &arg_ty,
                arg.span,
                symbols,
            ));
        }
        ensure_compile_time_requirement(&sig[idx], arg, &arg_ty, type_env, symbols, caller, idx)?;
    }

    let element_ty = element_type_for_variadic(&sig[variadic_idx], symbols).ok_or_else(|| {
        ParseError::new(
            format!(
                "cannot determine element type for variadic parameter of '{}'",
                caller
            ),
            span,
        )
    })?;

    let suffix_arg_start = args.len() - suffix_required;
    let mut arg_idx = prefix_required;
    while arg_idx < suffix_arg_start {
        let arg = &args[arg_idx];
        let arg_ty = lookup_arg_type(arg, type_env, symbols).ok_or_else(|| {
            ParseError::new(
                format!("unknown type for argument {} to '{}'", arg_idx + 1, caller),
                arg.span,
            )
        })?;
        if !type_satisfies(&element_ty, &arg_ty, symbols) {
            return Err(type_mismatch_error(
                caller,
                arg_idx + 1,
                &element_ty,
                &arg_ty,
                arg.span,
                symbols,
            ));
        }
        ensure_compile_time_requirement(
            &element_ty,
            arg,
            &arg_ty,
            type_env,
            symbols,
            caller,
            arg_idx,
        )?;
        arg_idx += 1;
    }

    for (offset, expected) in sig[variadic_idx + 1..].iter().enumerate() {
        let arg_pos = suffix_arg_start + offset;
        let arg = &args[arg_pos];
        let arg_ty = lookup_arg_type(arg, type_env, symbols).ok_or_else(|| {
            ParseError::new(
                format!("unknown type for argument {} to '{}'", arg_pos + 1, caller),
                arg.span,
            )
        })?;
        if !type_satisfies(expected, &arg_ty, symbols) {
            return Err(type_mismatch_error(
                caller,
                arg_pos + 1,
                expected,
                &arg_ty,
                arg.span,
                symbols,
            ));
        }
        ensure_compile_time_requirement(
            expected, arg, &arg_ty, type_env, symbols, caller, arg_pos,
        )?;
    }

    Ok(Vec::new())
}

const TYPE_MISMATCH_DEPTH_LIMIT: usize = 32;

fn type_mismatch_error(
    caller: &str,
    arg_index: usize,
    expected: &TypeRef,
    actual: &TypeRef,
    span: Span,
    symbols: &SymbolRegistry,
) -> CompileError {
    let (expected_mismatch, actual_mismatch) = find_type_mismatch_pair(expected, actual, symbols);
    let base_message = format!(
        "argument {} to '{}' has type {} but expected {}",
        arg_index,
        caller,
        format_source_type_ref(&actual_mismatch, symbols),
        format_source_type_ref(&expected_mismatch, symbols),
    );
    let hint = format_source_type_ref(actual, symbols);
    ParseError::new(
        format!(
            "{}; try changing the '{}' signature to {}",
            base_message, caller, hint
        ),
        span,
    )
    .into()
}

fn find_type_mismatch_pair(
    expected: &TypeRef,
    actual: &TypeRef,
    symbols: &SymbolRegistry,
) -> (TypeRef, TypeRef) {
    let mut expected_visited = HashSet::new();
    let expanded_expected = expand_alias_chain(expected, symbols, &mut expected_visited);
    let mut actual_visited = HashSet::new();
    let expanded_actual = expand_alias_chain(actual, symbols, &mut actual_visited);
    find_type_mismatch_pair_inner(&expanded_expected, &expanded_actual, symbols, 0)
}

fn find_type_mismatch_pair_inner(
    expected: &TypeRef,
    actual: &TypeRef,
    symbols: &SymbolRegistry,
    depth: usize,
) -> (TypeRef, TypeRef) {
    if depth >= TYPE_MISMATCH_DEPTH_LIMIT || type_satisfies(expected, actual, symbols) {
        return (expected.clone(), actual.clone());
    }
    if let (TypeRef::Type(expected_params), TypeRef::Type(actual_params)) = (expected, actual) {
        let expected_trimmed = trim_unit_continuation(expected_params, symbols);
        let actual_trimmed = trim_unit_continuation(actual_params, symbols);
        let len = expected_trimmed.len().min(actual_trimmed.len());
        for idx in 0..len {
            let expected_param = &expected_trimmed[idx];
            let actual_param = &actual_trimmed[idx];
            if type_satisfies(expected_param, actual_param, symbols) {
                continue;
            }
            return find_type_mismatch_pair_inner(expected_param, actual_param, symbols, depth + 1);
        }
    }
    (expected.clone(), actual.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::symbol::SymbolRegistry;

    #[test]
    fn nested_tuple_mismatch_reports_inner_types() {
        let symbols = SymbolRegistry::new();
        let expected = TypeRef::Type(vec![
            TypeRef::Int,
            TypeRef::Type(vec![
                TypeRef::Int,
                TypeRef::Type(vec![]),
                TypeRef::Type(vec![TypeRef::Int]),
            ]),
        ]);
        let actual = TypeRef::Type(vec![
            TypeRef::Int,
            TypeRef::Type(vec![
                TypeRef::Int,
                TypeRef::Type(vec![]),
                TypeRef::Type(vec![TypeRef::Str]),
            ]),
        ]);
        let (expected_mismatch, actual_mismatch) =
            find_type_mismatch_pair(&expected, &actual, &symbols);
        assert_eq!(expected_mismatch, TypeRef::Int);
        assert_eq!(actual_mismatch, TypeRef::Str);
    }
}

fn element_type_for_variadic(ty: &TypeRef, symbols: &SymbolRegistry) -> Option<TypeRef> {
    let mut visited = HashSet::new();
    let expanded = expand_alias_chain(ty, symbols, &mut visited);
    let mut current_params = if let TypeRef::Type(params) = expanded {
        params
    } else {
        return None;
    };
    loop {
        if current_params.len() == 1 {
            if let TypeRef::Type(inner) = &current_params[0] {
                current_params = inner.clone();
                continue;
            }
        }
        break;
    }
    if current_params.len() > 1 {
        let nth_type = current_params[1].clone();
        let mut visited = HashSet::new();
        let nth_expanded = expand_alias_chain(&nth_type, symbols, &mut visited);
        if let TypeRef::Type(nth_params) = nth_expanded {
            if nth_params.len() > 2 {
                let mut visited = HashSet::new();
                let one_expanded = expand_alias_chain(&nth_params[2], symbols, &mut visited);
                if let TypeRef::Type(one_params) = one_expanded {
                    return one_params.get(0).cloned();
                }
            }
        }
    }
    None
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
) -> Option<(Vec<TypeRef>, Vec<bool>)> {
    if let Some(sig) = symbols.get_function(target) {
        return Some((sig.params.clone(), sig.is_variadic.clone()));
    }
    if let Some(entry) = type_env.get(target) {
        if let Some(params) = resolve_type_signature(&entry.ty, symbols) {
            let variadic_flags = symbols
                .get_type_variadic(&entry.ty)
                .cloned()
                .unwrap_or_else(|| vec![false; params.len()]);
            return Some((params, variadic_flags));
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
        TypeRef::AliasInstance { .. } => {
            let mut expand_visited = HashSet::new();
            let expanded = expand_alias_chain(ty, symbols, &mut expand_visited);
            resolve_alias_signature(&expanded, symbols, visited)
        }
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

fn format_source_type_ref(ty: &TypeRef, symbols: &SymbolRegistry) -> String {
    fn helper(ty: &TypeRef, symbols: &SymbolRegistry, visited: &mut HashSet<String>) -> String {
        match ty {
            TypeRef::Int => "int".to_string(),
            TypeRef::Str => "str".to_string(),
            TypeRef::CompileTimeInt => "int!".to_string(),
            TypeRef::CompileTimeStr => "str!".to_string(),
            TypeRef::Alias(name) => {
                if let Some(info) = symbols.get_type_info(name) {
                    if name.starts_with("__type_") && !visited.contains(name) {
                        visited.insert(name.clone());
                        let rendered = helper(&info.target, symbols, visited);
                        visited.remove(name);
                        return rendered;
                    }
                }
                name.clone()
            }
            TypeRef::AliasInstance { name, args } => format!(
                "{}<{}>",
                name,
                args.iter()
                    .map(|ty| helper(ty, symbols, visited))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            TypeRef::Type(inner) => format!(
                "({})",
                inner
                    .iter()
                    .map(|ty| helper(ty, symbols, visited))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            TypeRef::Generic(name) => name.clone(),
        }
    }

    helper(ty, symbols, &mut HashSet::new())
}

fn type_satisfies(expected: &TypeRef, actual: &TypeRef, symbols: &SymbolRegistry) -> bool {
    let mut expand_visited = HashSet::new();
    let expanded_expected = expand_alias_chain(expected, symbols, &mut expand_visited);
    expand_visited.clear();
    let expanded_actual = expand_alias_chain(actual, symbols, &mut expand_visited);
    let mut visited = HashSet::new();
    type_satisfies_inner(&expanded_expected, &expanded_actual, symbols, &mut visited)
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
        TypeRef::AliasInstance { .. } => {
            let mut expand_visited = HashSet::new();
            let expanded = expand_alias_chain(ty, symbols, &mut expand_visited);
            compile_time_requirement_inner(&expanded, symbols, visited)
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
                    format_source_type_ref(param_ty, symbols),
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
        TypeRef::AliasInstance { .. } => {
            let mut expand_visited = HashSet::new();
            let expanded = expand_alias_chain(ty, symbols, &mut expand_visited);
            is_unit_type(&expanded, symbols, visited)
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
            BlockItem::Exec(exec) => {
                rewrite_args(&mut exec.args, &mut prefix, symbols, exec.span);
                rewrite_function_target(&mut exec.of, &mut prefix, symbols, exec.span);
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
