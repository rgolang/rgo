use std::collections::{BTreeSet, HashMap, HashSet};

pub use crate::compiler::air_ast::*;
pub use crate::compiler::ast;
use crate::compiler::builtins;
use crate::compiler::error::{Code, Error};
use crate::compiler::hir;
use crate::compiler::span::Span;
use crate::compiler::symbol::{self, SymbolRegistry};

const NUM_REMAINING_METADATA_WORD_OFFSET: usize = 5;

pub const ENTRY_FUNCTION_NAME: &str = "_start";

fn closure_unwrapper_label(name: &str) -> String {
    format!("{}_unwrapper", name)
}

fn closure_deep_release_label(name: &str) -> String {
    format!("{}_deep_release", name)
}

fn closure_deepcopy_label(name: &str) -> String {
    format!("{}_deepcopy", name)
}

impl AirNewClosure {
    pub fn unwrapper_label(&self) -> String {
        closure_unwrapper_label(&self.target.name)
    }

    pub fn deep_release_label(&self) -> String {
        closure_deep_release_label(&self.target.name)
    }

    pub fn deepcopy_label(&self) -> String {
        closure_deepcopy_label(&self.target.name)
    }
}

fn collect_unused_param_refs(params: &[SigItem]) -> HashSet<String> {
    params
        .iter()
        .filter(|param| matches!(param.kind, SigKind::Sig(_)))
        .map(|param| param.name.clone())
        .collect()
}

fn mark_target(unused: &mut HashSet<String>, target: &AirExecTarget) {
    if let AirExecTarget::Closure { name } = target {
        unused.remove(name);
    }
}

fn mark_args(unused: &mut HashSet<String>, args: &[AirArg]) {
    for arg in args {
        unused.remove(&arg.name);
    }
}

fn take_release_statements(unused: &mut HashSet<String>) -> Vec<AirStmt> {
    let names: BTreeSet<_> = unused.drain().collect();
    if names.is_empty() {
        return Vec::new();
    }
    names
        .into_iter()
        .map(|name| {
            AirStmt::Op(AirOp::ReleaseHeap(AirReleaseHeap {
                name,
                span: Span::unknown(),
            }))
        })
        .collect()
}

fn prepare_args(
    symbols: &mut SymbolRegistry,
    locals: &mut HashSet<String>,
    args: &[ast::Arg],
    statements: &mut Vec<AirStmt>,
    generated_functions: &mut Vec<AirFunction>,
    closure_remaining: &mut HashMap<String, Vec<SigKind>>,
) -> Result<(), Error> {
    for arg in args {
        // checks whether this argument is already a local binding before generating a closure for it.
        // This prevents re-wrapping a name that's already been bound/converted earlier
        // (e.g., because it was already turned into a captured closure or defined locally) and keeps the argument list stable.
        if locals.contains(arg.name.as_str()) {
            continue;
        }
        if let Some(closure) =
            create_closure(symbols, &arg.name, arg.span, generated_functions, None)?
        {
            let name = closure.name.clone();
            let remaining = closure.target.param_kinds();
            statements.push(AirStmt::Op(AirOp::NewClosure(closure)));
            locals.insert(name.clone());
            if !remaining.is_empty() {
                closure_remaining.insert(name, remaining);
            }
        }
    }
    Ok(())
}

fn create_closure(
    symbols: &mut SymbolRegistry,
    target: &str,
    span: Span,
    _generated_functions: &mut Vec<AirFunction>,
    sig_override: Option<&mut FunctionSig>,
) -> Result<Option<AirNewClosure>, Error> {
    if sig_override.is_some() {
        return Ok(None);
    }

    if let Some(orig_sig) = symbols.get_function(target) {
        let cloned_sig = orig_sig.clone();
        return Ok(Some(AirNewClosure {
            target: cloned_sig,
            args: Vec::new(),
            name: target.to_string(),
            span,
        }));
    }

    if let Some(builtin_name) = symbols.builtin_name_for_alias(target) {
        let builtin_sig = symbol::builtin_function_sig(builtin_name, span)?;
        return Ok(Some(AirNewClosure {
            target: builtin_sig,
            args: Vec::new(),
            name: target.to_string(),
            span,
        }));
    }

    Ok(None)
}

pub fn entry_function(
    entry_items: Vec<hir::BlockItem>,
    symbols: &mut SymbolRegistry,
) -> Result<Vec<AirFunction>, Error> {
    let mut items: Vec<AirStmt> = Vec::new();
    let mut locals = HashSet::new();
    let mut generated_functions = Vec::new();
    let mut unused_params = HashSet::new();
    let mut literals = HashMap::new();
    let mut closure_remaining: HashMap<String, Vec<SigKind>> = HashMap::new();
    let mut remaining_uses = count_block_uses(&entry_items);
    for item in entry_items.iter() {
        match item {
            hir::BlockItem::Import { .. }
            | hir::BlockItem::FunctionDef { .. }
            | hir::BlockItem::SigDef { .. } => {} // already handled
            _ => {
                let stmt = lower_block_item(
                    item.clone(),
                    &mut locals,
                    symbols,
                    &mut generated_functions,
                    &mut unused_params,
                    &mut literals,
                    &mut closure_remaining,
                    &mut remaining_uses,
                )?;
                items.extend(stmt);
            }
        }
    }
    generated_functions.push(AirFunction {
        sig: FunctionSig {
            name: ENTRY_FUNCTION_NAME.into(),
            params: Vec::new(),
            span: Span::unknown(),
            builtin: None,
        },
        items,
    });
    Ok(generated_functions)
}

pub fn lower_function(
    func: &hir::Function,
    symbols: &mut SymbolRegistry,
) -> Result<Vec<AirFunction>, Error> {
    let params = func.sig.items.clone();
    let sig = FunctionSig {
        name: func.name.clone(),
        params: params.clone(),
        span: func.span,
        builtin: None,
    };
    symbols.declare_function(sig.clone())?;

    let mut locals = HashSet::new();
    let mut closure_remaining: HashMap<String, Vec<SigKind>> = HashMap::new();
    for param in func.sig.items.iter() {
        locals.insert(param.name.clone());
        if let SigKind::Sig(signature) = &param.kind {
            closure_remaining.insert(param.name.clone(), signature.kinds());
        }
    }
    let mut unused_param_refs = collect_unused_param_refs(&params);
    let mut remaining_uses = count_block_uses(&func.body.items);

    let mut lowered_items: Vec<AirStmt> = Vec::new();
    let mut generated_functions: Vec<AirFunction> = Vec::new();
    let mut literals = HashMap::new();
    for item in func.body.items.iter() {
        let lowered = lower_block_item(
            item.clone(),
            &mut locals,
            symbols,
            &mut generated_functions,
            &mut unused_param_refs,
            &mut literals,
            &mut closure_remaining,
            &mut remaining_uses,
        )?;
        lowered_items.extend(lowered);
    }

    let function = AirFunction {
        sig: sig,
        items: lowered_items,
    };

    let mut functions: Vec<AirFunction> = vec![function.clone()];
    // TODO: Only generate these helpers if needed.
    if let Some(f) = build_closure_unwrapper(&function) {
        functions.push(f);
    }
    if let Some(f) = build_deep_release_helper(&function) {
        functions.push(f);
    }
    if let Some(f) = build_deep_copy_helper(&function) {
        functions.push(f);
    }
    functions.extend(generated_functions.into_iter());
    Ok(functions)
}

fn lower_block_item(
    item: hir::BlockItem,
    locals: &mut HashSet<String>,
    symbols: &mut SymbolRegistry,
    generated_functions: &mut Vec<AirFunction>,
    unused_params: &mut HashSet<String>,
    literals: &mut HashMap<String, ast::Lit>,
    closure_remaining: &mut HashMap<String, Vec<SigKind>>,
    remaining_uses: &mut HashMap<String, usize>,
) -> Result<Vec<AirStmt>, Error> {
    let lowered = match item {
        hir::BlockItem::FunctionDef(..) => {
            // TODO: This should be unreachable?!
            vec![]
        }
        hir::BlockItem::LitDef { name, literal } => {
            locals.insert(name.clone());
            literals.insert(name.clone(), literal.value);
            vec![]
        }
        hir::BlockItem::ClosureDef(closure) => {
            if locals.contains(&closure.of) && closure_remaining.contains_key(&closure.of) {
                lower_closure_curry(
                    &closure,
                    symbols,
                    locals,
                    generated_functions,
                    unused_params,
                    literals,
                    closure_remaining,
                    remaining_uses,
                )?
            } else {
                lower_new_closure(
                    &closure,
                    symbols,
                    locals,
                    generated_functions,
                    unused_params,
                    literals,
                    closure_remaining,
                    remaining_uses,
                )?
            }
        }
        hir::BlockItem::Exec(exec) => lower_exec(
            &exec,
            symbols,
            locals,
            generated_functions,
            unused_params,
            literals,
            closure_remaining,
            remaining_uses,
        )?,
        _ => unreachable!("unexpected block item: {:#?}", item),
    };
    Ok(lowered)
}

fn count_block_uses(items: &[hir::BlockItem]) -> HashMap<String, usize> {
    let mut uses: HashMap<String, usize> = HashMap::new();
    for item in items {
        match item {
            hir::BlockItem::ClosureDef(closure) => {
                *uses.entry(closure.of.clone()).or_insert(0) += 1;
                for arg in &closure.args {
                    *uses.entry(arg.name.clone()).or_insert(0) += 1;
                }
            }
            hir::BlockItem::Exec(exec) => {
                *uses.entry(exec.of.clone()).or_insert(0) += 1;
                for arg in &exec.args {
                    *uses.entry(arg.name.clone()).or_insert(0) += 1;
                }
            }
            _ => {}
        }
    }
    uses
}

fn take_remaining_use(remaining_uses: &mut HashMap<String, usize>, name: &str) -> usize {
    if let Some(count) = remaining_uses.get_mut(name) {
        let previous = *count;
        *count = count.saturating_sub(1);
        previous
    } else {
        0
    }
}

fn ensure_target(
    symbols: &mut SymbolRegistry,
    locals: &mut HashSet<String>,
    args: &[ast::Arg],
    target_name: &str,
    span: Span,
    generated_functions: &mut Vec<AirFunction>,
    literals: &HashMap<String, ast::Lit>,
    closure_remaining: &mut HashMap<String, Vec<SigKind>>,
) -> Result<(Vec<AirStmt>, AirExecTarget, Vec<AirArg>), Error> {
    let mut block_items = Vec::new();
    prepare_args(
        symbols,
        locals,
        args,
        &mut block_items,
        generated_functions,
        closure_remaining,
    )?;
    let mut target = if locals.contains(target_name) {
        AirExecTarget::Closure {
            name: target_name.to_string(),
        }
    } else {
        resolve_target(target_name, span, symbols)?
    };
    let args = extract_closure_sig_info(&mut target, args, literals);
    if let AirExecTarget::Function(sig) = &mut target {
        create_closure(symbols, target_name, span, generated_functions, Some(sig))?;
    }
    Ok((block_items, target, args))
}

fn closure_remaining_after_applying(
    closure_remaining: &HashMap<String, Vec<SigKind>>,
    target: &AirExecTarget,
    applied: usize,
) -> Option<Vec<SigKind>> {
    let remaining = match target {
        AirExecTarget::Function(sig) => sig.param_kinds(),
        AirExecTarget::Closure { name } => closure_remaining.get(name).cloned()?,
    };
    if remaining.is_empty() {
        return None;
    }
    let applied = applied.min(remaining.len());
    Some(remaining[applied..].to_vec())
}

fn lower_new_closure(
    closure: &hir::Closure,
    symbols: &mut SymbolRegistry,
    locals: &mut HashSet<String>,
    generated_functions: &mut Vec<AirFunction>,
    unused_params: &mut HashSet<String>,
    literals: &HashMap<String, ast::Lit>,
    closure_remaining: &mut HashMap<String, Vec<SigKind>>,
    remaining_uses: &mut HashMap<String, usize>,
) -> Result<Vec<AirStmt>, Error> {
    take_remaining_use(remaining_uses, &closure.of);
    for arg in &closure.args {
        take_remaining_use(remaining_uses, &arg.name);
    }
    let (mut block_items, target, args) = ensure_target(
        symbols,
        locals,
        &closure.args,
        &closure.of,
        closure.span,
        generated_functions,
        literals,
        closure_remaining,
    )?;
    locals.insert(closure.name.clone());
    mark_target(unused_params, &target);
    mark_args(unused_params, &args);

    let new_remaining = closure_remaining_after_applying(closure_remaining, &target, args.len());
    let target_sig = match target {
        AirExecTarget::Function(sig) => sig,
        _ => {
            return Err(Error::new(
                Code::Internal,
                "expected function target when creating new closure".to_string(),
                closure.span,
            ))
        }
    };
    block_items.push(AirStmt::Op(AirOp::NewClosure(AirNewClosure {
        name: closure.name.clone(),
        target: target_sig.clone(),
        args,
        span: closure.span,
    })));
    if let Some(remaining) = new_remaining {
        closure_remaining.insert(closure.name.clone(), remaining);
    }
    Ok(block_items)
}

fn lower_closure_curry(
    closure: &hir::Closure,
    symbols: &mut SymbolRegistry,
    locals: &mut HashSet<String>,
    generated_functions: &mut Vec<AirFunction>,
    unused_params: &mut HashSet<String>,
    literals: &HashMap<String, ast::Lit>,
    closure_remaining: &mut HashMap<String, Vec<SigKind>>,
    remaining_uses: &mut HashMap<String, usize>,
) -> Result<Vec<AirStmt>, Error> {
    let existing_remaining = closure_remaining.get(&closure.of).cloned().ok_or_else(|| {
        Error::new(
            Code::Internal,
            format!("missing closure signature for '{}'", closure.of),
            closure.span,
        )
    })?;

    let (mut block_items, _, _) = ensure_target(
        symbols,
        locals,
        &closure.args,
        &closure.of,
        closure.span,
        generated_functions,
        literals,
        closure_remaining,
    )?;

    let applied = closure.args.len().min(existing_remaining.len());
    let mut args = Vec::with_capacity(closure.args.len());
    for (idx, arg) in closure.args.iter().enumerate() {
        let kind = existing_remaining.get(idx).cloned().unwrap_or(SigKind::Int);
        args.push(AirArg {
            name: arg.name.clone(),
            kind,
            literal: literal_for_arg(&arg.name, literals),
        });
    }
    mark_target(
        unused_params,
        &AirExecTarget::Closure {
            name: closure.of.clone(),
        },
    );
    mark_args(unused_params, &args);

    locals.insert(closure.name.clone());
    block_items.push(AirStmt::Op(AirOp::CloneClosure(AirCloneClosure {
        src: closure.of.clone(),
        dst: closure.name.clone(),
        remaining: existing_remaining.clone(),
        span: closure.span,
    })));

    let mut stored_args = Vec::with_capacity(args.len());
    for (idx, arg) in args.iter().enumerate() {
        let arg_use_count = take_remaining_use(remaining_uses, &arg.name);
        let should_clone_arg = matches!(arg.kind, SigKind::Sig(_)) && arg_use_count > 1;
        if should_clone_arg {
            let arg_remaining = closure_remaining.get(&arg.name).cloned().ok_or_else(|| {
                Error::new(
                    Code::Internal,
                    format!("missing closure signature for '{}'", arg.name),
                    closure.span,
                )
            })?;
            let clone_name = format!("__{}_arg_clone_{}", closure.name, idx);
            block_items.push(AirStmt::Op(AirOp::CloneClosure(AirCloneClosure {
                src: arg.name.clone(),
                dst: clone_name.clone(),
                remaining: arg_remaining,
                span: closure.span,
            })));
            stored_args.push(AirArg {
                name: clone_name,
                kind: arg.kind.clone(),
                literal: None,
            });
        } else {
            stored_args.push(arg.clone());
        }
    }

    let env_end_binding = format!("__{}_env_end", closure.name);
    block_items.push(AirStmt::Op(AirOp::Pin(AirPin {
        result: env_end_binding.clone(),
        value: AirValue::Binding(closure.name.clone()),
        span: closure.span,
    })));

    let suffix_word_counts = suffix_word_counts(&existing_remaining);
    for (idx, arg) in stored_args.iter().take(applied).enumerate() {
        let offset_words = suffix_word_counts[idx] as isize;
        block_items.push(AirStmt::Op(AirOp::SetField(AirSetField {
            env_end: env_end_binding.clone(),
            offset: -offset_words,
            value: arg.clone(),
            span: closure.span,
        })));
    }

    let remaining = existing_remaining[applied..].to_vec();
    let remaining_words = word_count_from_kinds(&remaining) as isize;
    block_items.push(AirStmt::Op(AirOp::SetField(AirSetField {
        env_end: env_end_binding,
        offset: NUM_REMAINING_METADATA_WORD_OFFSET as isize,
        value: AirArg {
            name: format!("__{}_num_remaining_value", closure.name),
            kind: SigKind::Int,
            literal: Some(ast::Lit::Int(remaining_words)),
        },
        span: closure.span,
    })));

    closure_remaining.insert(closure.name.clone(), remaining.clone());

    Ok(block_items)
}

fn lower_exec(
    exec: &hir::Exec,
    symbols: &mut SymbolRegistry,
    locals: &mut HashSet<String>,
    generated_functions: &mut Vec<AirFunction>,
    unused_params: &mut HashSet<String>,
    literals: &HashMap<String, ast::Lit>,
    closure_remaining: &mut HashMap<String, Vec<SigKind>>,
    remaining_uses: &mut HashMap<String, usize>,
) -> Result<Vec<AirStmt>, Error> {
    let exec = exec.clone();
    take_remaining_use(remaining_uses, &exec.of);
    for arg in &exec.args {
        take_remaining_use(remaining_uses, &arg.name);
    }
    let (mut block_items, target, args) = ensure_target(
        symbols,
        locals,
        &exec.args,
        &exec.of,
        exec.span,
        generated_functions,
        literals,
        closure_remaining,
    )?;
    mark_target(unused_params, &target);
    mark_args(unused_params, &args);

    if let AirExecTarget::Function(sig) = &target {
        if let Some(builtin) = sig.builtin {
            if builtin.is_call() {
                let builtin_items =
                    lower_builtin_call(sig, builtin, args, exec.span, unused_params)?;
                block_items.extend(builtin_items);
                return Ok(block_items);
            }
            if is_inline_builtin(builtin) {
                for continuation in &sig.params {
                    if matches!(continuation.kind, SigKind::Sig(_)) {
                        mark_target(
                            unused_params,
                            &AirExecTarget::Closure {
                                name: continuation.name.clone(),
                            },
                        );
                    }
                }
                block_items.extend(take_release_statements(unused_params));
                block_items.extend(build_builtin_statements(sig, builtin, args, exec.span));
                return Ok(block_items);
            }
        }
    }

    block_items.extend(take_release_statements(unused_params));
    match target {
        AirExecTarget::Function(sig) => {
            block_items.push(AirStmt::Op(AirOp::JumpArgs(AirJumpArgs {
                target: sig,
                args,
                span: exec.span,
            })));
        }
        AirExecTarget::Closure { name } => {
            block_items.push(AirStmt::Op(AirOp::JumpClosure(AirJumpClosure {
                env_end: name,
                args,
                span: exec.span,
            })));
        }
    }
    Ok(block_items)
}

fn lower_builtin_call(
    sig: &FunctionSig,
    builtin: builtins::Builtin,
    args: Vec<AirArg>,
    span: Span,
    unused_params: &mut HashSet<String>,
) -> Result<Vec<AirStmt>, Error> {
    let continuation_start = continuation_start_index(&sig.params);
    let continuation_params = &sig.params[continuation_start..];
    let continuation_count = sig.params.len() - continuation_start;
    let args_len = args.len();
    let continuation_args = if continuation_count > 0 {
        args[args_len - continuation_count..].to_vec()
    } else {
        Vec::new()
    };
    let mut stmts = Vec::new();
    for (param, continuation_arg) in continuation_params.iter().zip(continuation_args.iter()) {
        if matches!(param.kind, SigKind::Sig(_)) {
            mark_target(
                unused_params,
                &AirExecTarget::Closure {
                    name: continuation_arg.name.clone(),
                },
            );
        }
    }

    stmts.extend(take_release_statements(unused_params));
    let call_stmt = AirStmt::Op(call_op(builtin, args, span));
    stmts.push(call_stmt);

    Ok(stmts)
}

// TODO: Simplify this.
fn resolve_target(
    name: &str,
    span: Span,
    symbols: &SymbolRegistry,
) -> Result<AirExecTarget, Error> {
    if let Some(sig) = symbols.get_function(name) {
        return Ok(AirExecTarget::Function(sig.clone()));
    }
    if let Some(builtin_name) = symbols.builtin_name_for_alias(name) {
        let sig = symbol::builtin_function_sig(builtin_name, span)?;
        return Ok(AirExecTarget::Function(sig));
    }
    Ok(AirExecTarget::Closure {
        name: name.to_string(),
    })
}

fn build_closure_unwrapper(function: &AirFunction) -> Option<AirFunction> {
    let env_param = SigItem {
        name: "env_end".to_string(),
        kind: SigKind::Int,
        has_bang: false,
        span: function.sig.span,
    };

    Some(build_unwrapper_function(
        closure_unwrapper_label(&function.sig.name),
        function.sig.clone(),
        env_param,
        function.sig.params.clone(),
        function.sig.span,
    ))
}

fn extract_closure_sig_info(
    target: &AirExecTarget,
    args: &[Arg],
    literals: &HashMap<String, ast::Lit>,
) -> Vec<AirArg> {
    if let Some(params) = target_signature(target) {
        return consume_signature_for_args(params, args, literals);
    }
    let fallback_args = args
        .iter()
        .map(|arg| AirArg {
            name: arg.name.clone(),
            kind: SigKind::Int,
            literal: literal_for_arg(&arg.name, literals),
        })
        .collect();
    fallback_args
}

fn target_signature<'a>(target: &'a AirExecTarget) -> Option<&'a [SigItem]> {
    match target {
        AirExecTarget::Function(sig) => Some(&sig.params),
        AirExecTarget::Closure { .. } => None,
    }
}

fn consume_signature_for_args(
    params: &[SigItem],
    args: &[Arg],
    literals: &HashMap<String, ast::Lit>,
) -> Vec<AirArg> {
    let mut consumed = 0;
    let mut sig_index = 0;
    let total = params.len();
    let mut air_args = Vec::with_capacity(args.len());
    while consumed < args.len() && sig_index < total {
        match &params[sig_index].kind {
            SigKind::Variadic => {
                let remaining_args = args.len() - consumed;
                let final_items = total.saturating_sub(sig_index + 1);
                let variadic_count = remaining_args.saturating_sub(final_items);
                for _ in 0..variadic_count {
                    air_args.push(AirArg {
                        name: args[consumed].name.clone(),
                        kind: SigKind::Int,
                        literal: literal_for_arg(&args[consumed].name, literals),
                    });
                    consumed += 1;
                }
                sig_index += 1;
            }
            ty => {
                air_args.push(AirArg {
                    name: args[consumed].name.clone(),
                    kind: ty.clone(),
                    literal: literal_for_arg(&args[consumed].name, literals),
                });
                consumed += 1;
                sig_index += 1;
            }
        }
    }

    while consumed < args.len() {
        air_args.push(AirArg {
            name: args[consumed].name.clone(),
            kind: SigKind::Int,
            literal: literal_for_arg(&args[consumed].name, literals),
        });
        consumed += 1;
    }

    air_args
}

fn literal_for_arg(name: &str, literals: &HashMap<String, ast::Lit>) -> Option<ast::Lit> {
    literals.get(name).cloned()
}

fn build_unwrapper_function(
    name: String,
    target_sig: FunctionSig,
    env_param: SigItem,
    field_sig_items: Vec<SigItem>,
    span: Span,
) -> AirFunction {
    let env_end_reg = "__env_end".to_string();
    let env_word_count = env_word_count_from_params(&field_sig_items);
    let env_word_count_isize = env_word_count as isize;
    let offsets = env_word_offsets_from_params(&field_sig_items);
    let mut items = Vec::with_capacity(field_sig_items.len() + 1);

    items.push(AirStmt::Op(AirOp::Pin(AirPin {
        result: env_end_reg.clone(),
        value: AirValue::Binding(env_param.name.clone()),
        span,
    })));

    for (idx, sig_item) in field_sig_items.iter().enumerate() {
        let offset = offsets[idx] as isize - env_word_count_isize;
        items.push(AirStmt::Op(AirOp::Field(AirField {
            result: sig_item.name.clone(),
            ptr: env_end_reg.clone(),
            offset,
            kind: sig_item.kind.clone(),
            span: sig_item.span,
        })));
    }

    items.push(AirStmt::Op(AirOp::ReleaseHeap(AirReleaseHeap {
        name: env_end_reg.clone(),
        span: Span::unknown(),
    })));

    let builtin_args = field_sig_items
        .iter()
        .map(|item| AirArg {
            name: item.name.clone(),
            kind: item.kind.clone(),
            literal: None,
        })
        .collect::<Vec<_>>();

    if let Some(builtin) = target_sig.builtin {
        if builtin.is_call() || is_inline_builtin(builtin) {
            items.extend(build_builtin_statements(
                &target_sig,
                builtin,
                builtin_args,
                span,
            ));
        } else {
            items.push(AirStmt::Op(AirOp::JumpArgs(AirJumpArgs {
                target: target_sig.clone(),
                args: builtin_args,
                span,
            })));
        }
    } else {
        items.push(AirStmt::Op(AirOp::JumpArgs(AirJumpArgs {
            target: target_sig.clone(),
            args: builtin_args,
            span,
        })));
    }

    AirFunction {
        sig: FunctionSig {
            name,
            params: vec![env_param],
            span,
            builtin: None,
        },
        items,
    }
}

fn build_deep_release_helper(function: &AirFunction) -> Option<AirFunction> {
    let env_param = SigItem {
        name: "env_end".to_string(),
        kind: SigKind::Int,
        has_bang: false,
        span: function.sig.span,
    };

    let offsets = env_word_offsets_from_params(&function.sig.params);
    let env_word_count = env_word_count_from_params(&function.sig.params);
    let env_word_count_isize = env_word_count as isize;
    let mut items = Vec::new();
    let num_remaining_binding = "__num_remaining".to_string();
    let env_end_reg = "__env_end".to_string();

    items.push(AirStmt::Op(AirOp::Pin(AirPin {
        result: env_end_reg.clone(),
        value: AirValue::Binding(env_param.name.clone()),
        span: function.sig.span,
    })));

    let reference_fields = function
        .sig
        .params
        .iter()
        .enumerate()
        .filter_map(|(idx, param)| {
            if !is_reference_type(&param.kind) {
                return None;
            }
            let offset_from_end = env_word_count.saturating_sub(offsets[idx]);
            Some((
                idx,
                offsets[idx] as isize - env_word_count_isize,
                offset_from_end,
                param.kind.clone(),
            ))
        })
        .collect::<Vec<_>>();

    if !reference_fields.is_empty() {
        items.push(AirStmt::Op(AirOp::Field(AirField {
            result: num_remaining_binding.clone(),
            ptr: env_end_reg.clone(),
            offset: NUM_REMAINING_METADATA_WORD_OFFSET as isize,
            kind: SigKind::Int,
            span: function.sig.span,
        })));
        for (idx, offset, offset_from_end, kind) in &reference_fields {
            let skip_label = format!("{}_release_skip_{}", function.sig.name, idx);
            let threshold = offset_from_end.saturating_sub(1);
            items.push(AirStmt::Op(AirOp::JumpGt(AirJumpGt {
                left: AirValue::Binding(num_remaining_binding.clone()),
                right: AirValue::Literal(threshold as i64),
                target: skip_label.clone(),
                span: function.sig.span,
            })));
            let location = format!("{}_release_field_{}", function.sig.name, idx);
            items.push(AirStmt::Op(AirOp::Field(AirField {
                result: location.clone(),
                ptr: env_end_reg.clone(),
                offset: *offset,
                kind: kind.clone(),
                span: function.sig.span,
            })));
            items.push(AirStmt::Op(AirOp::CallPtr(AirCallPtr {
                target: AirCallPtrTarget::Binding(location),
                span: function.sig.span,
            })));
            items.push(AirStmt::Label(AirLabel { name: skip_label }));
        }
    }

    items.push(AirStmt::Op(AirOp::ReleaseHeap(AirReleaseHeap {
        name: env_end_reg.clone(),
        span: function.sig.span,
    })));

    items.push(AirStmt::Op(AirOp::Return(AirReturn { value: None })));

    Some(AirFunction {
        sig: FunctionSig {
            name: closure_deep_release_label(&function.sig.name),
            params: vec![env_param],
            span: function.sig.span,
            builtin: None,
        },
        items,
    })
}

fn build_deep_copy_helper(function: &AirFunction) -> Option<AirFunction> {
    let env_param = SigItem {
        name: "env_end".to_string(),
        kind: SigKind::Int,
        has_bang: false,
        span: function.sig.span,
    };

    let offsets = env_word_offsets_from_params(&function.sig.params);
    let env_word_count = env_word_count_from_params(&function.sig.params);
    let mut items = Vec::new();
    let num_remaining_binding = "num_remaining".to_string();
    let env_end_reg = "__env_end".to_string();

    items.push(AirStmt::Op(AirOp::Pin(AirPin {
        result: env_end_reg.clone(),
        value: AirValue::Binding(env_param.name.clone()),
        span: function.sig.span,
    })));

    let reference_fields = function
        .sig
        .params
        .iter()
        .enumerate()
        .filter_map(|(idx, param)| {
            if !is_reference_type(&param.kind) {
                return None;
            }
            let env_offset_from_start = offsets[idx];
            Some((idx, env_offset_from_start, param.kind.clone()))
        })
        .collect::<Vec<_>>();

    if !reference_fields.is_empty() {
        items.push(AirStmt::Op(AirOp::Field(AirField {
            result: num_remaining_binding.clone(),
            ptr: env_end_reg.clone(),
            offset: NUM_REMAINING_METADATA_WORD_OFFSET as isize,
            kind: SigKind::Int,
            span: function.sig.span,
        })));
        for (idx, env_offset_from_start, kind) in &reference_fields {
            let skip_label = format!("{}_deepcopy_skip_{}", function.sig.name, idx);
            let offset_from_end = env_word_count.saturating_sub(*env_offset_from_start);
            let threshold = offset_from_end.saturating_sub(1);
            items.push(AirStmt::Op(AirOp::JumpGt(AirJumpGt {
                left: AirValue::Binding(num_remaining_binding.clone()),
                right: AirValue::Literal(threshold as i64),
                target: skip_label.clone(),
                span: function.sig.span,
            })));
            items.push(AirStmt::Op(AirOp::CopyField(AirField {
                result: format!("{}_deepcopy_field_{}", function.sig.name, idx),
                ptr: env_end_reg.clone(),
                offset: -(offset_from_end as isize),
                kind: kind.clone(),
                span: function.sig.span,
            })));
            items.push(AirStmt::Label(AirLabel { name: skip_label }));
        }
    }

    items.push(AirStmt::Op(AirOp::Return(AirReturn { value: None })));

    Some(AirFunction {
        sig: FunctionSig {
            name: closure_deepcopy_label(&function.sig.name),
            params: vec![env_param],
            span: function.sig.span,
            builtin: None,
        },
        items,
    })
}

fn env_word_count_from_params(params: &[SigItem]) -> usize {
    params.len()
}

fn env_word_offsets_from_params(params: &[SigItem]) -> Vec<usize> {
    (0..params.len()).collect()
}

fn word_count_from_kinds(kinds: &[SigKind]) -> usize {
    kinds.len()
}

fn suffix_word_counts(kinds: &[SigKind]) -> Vec<usize> {
    let len = kinds.len();
    (0..len).map(|idx| len - idx).collect()
}

fn is_reference_type(ty: &SigKind) -> bool {
    return matches!(ty, SigKind::Sig(_));
}

fn instruction_op(builtin: builtins::Builtin, args: Vec<AirArg>, span: Span) -> AirOp {
    let arg_len = args.len();
    let continuation_target = args
        .last()
        .expect("builtin invocation requires a continuation target")
        .name
        .clone();
    let inputs = args[..arg_len - 1].to_vec();

    match builtin {
        builtins::Builtin::Add => AirOp::Add(AirAdd {
            inputs,
            target: continuation_target,
            span,
        }),
        builtins::Builtin::Sub => AirOp::Sub(AirSub {
            inputs,
            target: continuation_target,
            span,
        }),
        builtins::Builtin::Mul => AirOp::Mul(AirMul {
            inputs,
            target: continuation_target,
            span,
        }),
        builtins::Builtin::Div => AirOp::Div(AirDiv {
            inputs,
            target: continuation_target,
            span,
        }),
        builtins::Builtin::Eq | builtins::Builtin::Eqi => AirOp::JumpEqInt(AirJumpEq {
            args: inputs,
            target: continuation_target,
            span,
        }),
        builtins::Builtin::Eqs => AirOp::JumpEqStr(AirJumpEq {
            args: inputs,
            target: continuation_target,
            span,
        }),
        builtins::Builtin::Lt => {
            let (left, right) = binary_operands(builtin.name(), inputs);
            AirOp::JumpLt(AirJumpLt {
                left,
                right,
                target: continuation_target,
                span,
            })
        }
        builtins::Builtin::Gt => {
            let (left, right) = binary_operands(builtin.name(), inputs);
            AirOp::JumpGt(AirJumpGt {
                left,
                right,
                target: continuation_target,
                span,
            })
        }
        _ => unreachable!("unexpected instruction op: {}", builtin.name()),
    }
}

fn binary_operands(name: &str, inputs: Vec<AirArg>) -> (AirValue, AirValue) {
    let mut iter = inputs.into_iter();
    let left = iter
        .next()
        .unwrap_or_else(|| panic!("{} requires two operands", name));
    let right = iter
        .next()
        .unwrap_or_else(|| panic!("{} requires two operands", name));
    (arg_to_operand(left), arg_to_operand(right))
}

fn arg_to_operand(arg: AirArg) -> AirValue {
    if let Some(literal) = arg.literal {
        match literal {
            ast::Lit::Int(value) => AirValue::Literal(value as i64),
            ast::Lit::Str(_) => panic!("unexpected string literal in numeric operation"),
        }
    } else {
        AirValue::Binding(arg.name)
    }
}

fn call_op(builtin: builtins::Builtin, args: Vec<AirArg>, span: Span) -> AirOp {
    let arg_len = args.len();
    let continuation_target = args
        .last()
        .expect("builtin invocation requires a continuation target")
        .name
        .clone();
    let call_args = args[..arg_len - 1].to_vec();
    let arg_kinds = call_args
        .iter()
        .map(|arg| arg.kind.clone())
        .collect::<Vec<_>>();

    match builtin {
        builtins::Builtin::Printf => AirOp::Printf(AirPrintf {
            args: call_args,
            arg_kinds,
            target: continuation_target,
            span,
        }),
        builtins::Builtin::Sprintf => AirOp::Sprintf(AirSprintf {
            args: call_args,
            arg_kinds,
            target: continuation_target,
            span,
        }),
        builtins::Builtin::Write => AirOp::Write(AirWrite {
            args: call_args,
            arg_kinds,
            target: continuation_target,
            span,
        }),
        builtins::Builtin::Puts => AirOp::Puts(AirPuts {
            args: call_args,
            arg_kinds,
            target: continuation_target,
            span,
        }),
        builtins::Builtin::Exit => AirOp::SysExit(AirSysExit { args, span }),
        _ => unreachable!("unexpected call op: {}", builtin.name()),
    }
}

fn build_conditional_builtin_bridge(
    sig: &FunctionSig,
    builtin: builtins::Builtin,
    args: Vec<AirArg>,
) -> Vec<AirStmt> {
    let arg_len = args.len();
    let true_cont = &args[arg_len - 2];
    let false_cont = &args[arg_len - 1];
    let inputs = args[..arg_len - 2].to_vec();
    let true_label = format!("{}_true", sig.name);
    let false_label = format!("{}_false", sig.name);

    let eq_jump = if matches!(builtin, builtins::Builtin::Eqs) {
        AirOp::JumpEqStr(AirJumpEq {
            args: inputs.clone(),
            target: true_label.clone(),
            span: sig.span,
        })
    } else {
        AirOp::JumpEqInt(AirJumpEq {
            args: inputs.clone(),
            target: true_label.clone(),
            span: sig.span,
        })
    };

    vec![
        AirStmt::Op(eq_jump),
        AirStmt::Label(AirLabel {
            name: false_label.clone(),
        }),
        AirStmt::Op(AirOp::ReleaseHeap(AirReleaseHeap {
            name: true_cont.name.clone(),
            span: Span::unknown(),
        })),
        AirStmt::Op(AirOp::JumpClosure(AirJumpClosure {
            env_end: false_cont.name.clone(),
            args: Vec::new(),
            span: sig.span,
        })),
        AirStmt::Label(AirLabel {
            name: true_label.clone(),
        }),
        AirStmt::Op(AirOp::ReleaseHeap(AirReleaseHeap {
            name: false_cont.name.clone(),
            span: Span::unknown(),
        })),
        AirStmt::Op(AirOp::JumpClosure(AirJumpClosure {
            env_end: true_cont.name.clone(),
            args: Vec::new(),
            span: sig.span,
        })),
    ]
}

fn build_builtin_statements(
    sig: &FunctionSig,
    builtin: builtins::Builtin,
    args: Vec<AirArg>,
    span: Span,
) -> Vec<AirStmt> {
    if builtin.is_conditional() {
        return build_conditional_builtin_bridge(sig, builtin, args);
    }

    if builtin.is_instruction() {
        return vec![AirStmt::Op(instruction_op(builtin, args, span))];
    }

    if builtin.is_libc_call() {
        return vec![AirStmt::Op(call_op(builtin, args, span))];
    }

    vec![AirStmt::Op(AirOp::SysExit(AirSysExit { args, span }))]
}

fn is_inline_builtin(builtin: builtins::Builtin) -> bool {
    return matches!(
        builtin,
        builtins::Builtin::Add
            | builtins::Builtin::Sub
            | builtins::Builtin::Mul
            | builtins::Builtin::Div
            | builtins::Builtin::Eq
            | builtins::Builtin::Eqi
            | builtins::Builtin::Eqs
            | builtins::Builtin::Lt
            | builtins::Builtin::Gt
    );
}

fn continuation_start_index(params: &[SigItem]) -> usize {
    params
        .iter()
        .position(|param| matches!(param.kind, SigKind::Sig(_)))
        .unwrap_or(params.len())
}
