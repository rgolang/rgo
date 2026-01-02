use std::collections::{BTreeSet, HashSet};

use crate::compiler::ast;
use crate::compiler::builtins::MirInstKind;
use crate::compiler::codegen::ENV_METADATA_NUM_CURRIED_OFFSET;
use crate::compiler::error::{Code, Error};
use crate::compiler::hir;
pub use crate::compiler::mir_ast::*;
use crate::compiler::span::Span;
use crate::compiler::symbol::SymbolRegistry;

pub const ENTRY_FUNCTION_NAME: &str = "_start";

pub fn closure_unwrapper_label(name: &str) -> String {
    format!("{}_unwrapper", name)
}

pub fn closure_release_label(name: &str) -> String {
    format!("{}_release", name)
}

pub fn closure_deepcopy_label(name: &str) -> String {
    format!("{}_deepcopy", name)
}

fn collect_unused_param_refs(params: &[SigItem]) -> HashSet<String> {
    params
        .iter()
        .filter(|param| matches!(param.ty, SigKind::Sig(_)))
        .map(|param| param.name.clone())
        .collect()
}

fn mark_target(unused: &mut HashSet<String>, target: &MirExecTarget) {
    if let MirExecTarget::Closure { name } = target {
        unused.remove(name);
    }
}

fn mark_args(unused: &mut HashSet<String>, args: &[MirArg]) {
    for arg in args {
        unused.remove(&arg.name);
    }
}

fn take_release_statements(unused: &mut HashSet<String>) -> Vec<MirStmt> {
    let names: BTreeSet<_> = unused.drain().collect();
    names
        .into_iter()
        .map(|name| MirStmt::ReleaseClosure(ReleaseClosure { name }))
        .collect()
}

fn prepare_args(
    symbols: &mut SymbolRegistry,
    locals: &mut HashSet<String>,
    args: &[ast::Arg],
    statements: &mut Vec<MirStmt>,
    generated_functions: &mut Vec<MirFunction>,
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
            statements.push(MirStmt::Closure(closure));
            locals.insert(name.clone());
        }
    }
    Ok(())
}

fn create_closure(
    symbols: &mut SymbolRegistry,
    target: &str,
    span: Span,
    generated_functions: &mut Vec<MirFunction>,
    sig_override: Option<&mut FunctionSig>,
) -> Result<Option<MirClosure>, Error> {
    if let Some(sig) = sig_override {
        handle_builtin_for_sig(symbols, sig, generated_functions)?;
        return Ok(None);
    }

    if let Some(orig_sig) = symbols.get_function(target) {
        let mut cloned_sig = orig_sig.clone();
        handle_builtin_for_sig(symbols, &mut cloned_sig, generated_functions)?;
        return Ok(Some(MirClosure {
            target: MirExecTarget::Function(cloned_sig),
            args: Vec::new(),
            env_layout: Vec::new(),
            name: target.to_string(),
            span,
        }));
    }

    Ok(None)
}

fn handle_builtin_for_sig(
    symbols: &mut SymbolRegistry,
    sig: &mut FunctionSig,
    generated_functions: &mut Vec<MirFunction>,
) -> Result<(), Error> {
    if let Some(builtin_kind) = sig.builtin {
        match builtin_kind {
            builtin @ (MirBuiltin::Instruction(_) | MirBuiltin::SysCall(_)) => {
                let mut generated = ensure_builtin_bridge_generated_once(symbols, builtin, sig);
                generated_functions.append(&mut generated);
            }
            MirBuiltin::Call(_) => {
                // MirCall builtin functions are executed directly without additional bridging.
            }
        }
    }

    Ok(())
}

pub fn entry_function(
    entry_items: Vec<hir::BlockItem>,
    symbols: &mut SymbolRegistry,
) -> Result<Vec<MirFunction>, Error> {
    let mut items: Vec<MirStmt> = Vec::new();
    let mut locals = HashSet::new();
    let mut generated_functions = Vec::new();
    let mut unused_params = HashSet::new();
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
                )?;
                items.extend(stmt);
            }
        }
    }
    generated_functions.push(MirFunction {
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
) -> Result<Vec<MirFunction>, Error> {
    let params = func.sig.items.clone();
    let sig = FunctionSig {
        name: func.name.clone(),
        params: params.clone(),
        span: func.span,
        builtin: None,
    };
    symbols.declare_function(sig.clone())?;

    let mut locals = HashSet::new();
    for param in func.sig.items.iter() {
        locals.insert(param.name.clone());
    }
    let mut unused_param_refs = collect_unused_param_refs(&params);

    let mut lowered_items: Vec<MirStmt> = Vec::new();
    let mut generated_functions: Vec<MirFunction> = Vec::new();
    for item in func.body.items.iter() {
        let lowered = lower_block_item(
            item.clone(),
            &mut locals,
            symbols,
            &mut generated_functions,
            &mut unused_param_refs,
        )?;
        lowered_items.extend(lowered);
    }

    let function = MirFunction {
        sig: sig,
        items: lowered_items,
    };

    let mut functions: Vec<MirFunction> = vec![function.clone()];
    // TODO: Only generate these helpers if needed.
    if let Some(f) = build_closure_unwrapper(&function) {
        functions.push(f);
    }
    if let Some(f) = build_release_helper(&function) {
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
    generated_functions: &mut Vec<MirFunction>,
    unused_params: &mut HashSet<String>,
) -> Result<Vec<MirStmt>, Error> {
    let lowered = match item {
        hir::BlockItem::FunctionDef(..) => {
            // TODO: This should be unreachable?!
            vec![]
        }
        hir::BlockItem::StrDef { name, literal } => {
            locals.insert(name.clone());
            vec![MirStmt::StrDef { name, literal }]
        }
        hir::BlockItem::IntDef { name, literal } => {
            locals.insert(name.clone());
            vec![MirStmt::IntDef { name, literal }]
        }
        hir::BlockItem::ClosureDef(closure) => lower_closure(
            &closure,
            symbols,
            locals,
            generated_functions,
            unused_params,
        )?,
        hir::BlockItem::Exec(exec) => {
            lower_exec(&exec, symbols, locals, generated_functions, unused_params)?
        }
        _ => unreachable!("unexpected block item: {:#?}", item),
    };
    Ok(lowered)
}

fn ensure_target(
    symbols: &mut SymbolRegistry,
    locals: &mut HashSet<String>,
    args: &[ast::Arg],
    target_name: &str,
    span: Span,
    generated_functions: &mut Vec<MirFunction>,
) -> Result<(Vec<MirStmt>, MirExecTarget, Vec<MirArg>), Error> {
    let mut block_items = Vec::new();
    prepare_args(symbols, locals, args, &mut block_items, generated_functions)?;
    let mut target = if locals.contains(target_name) {
        MirExecTarget::Closure {
            name: target_name.to_string(),
        }
    } else {
        resolve_target(target_name, symbols)
    };
    let args = extract_closure_sig_info(&mut target, args);
    if let MirExecTarget::Function(sig) = &mut target {
        create_closure(symbols, target_name, span, generated_functions, Some(sig))?;
    }
    Ok((block_items, target, args))
}

// TODO: ABC: This is the target
fn lower_closure(
    closure: &hir::Closure,
    symbols: &mut SymbolRegistry,
    locals: &mut HashSet<String>,
    generated_functions: &mut Vec<MirFunction>,
    unused_params: &mut HashSet<String>,
) -> Result<Vec<MirStmt>, Error> {
    let (mut block_items, target, args) = ensure_target(
        symbols,
        locals,
        &closure.args,
        &closure.of,
        closure.span,
        generated_functions,
    )?;
    locals.insert(closure.name.clone());
    mark_target(unused_params, &target);
    mark_args(unused_params, &args);

    let env_layout = args.iter().map(|arg| arg.kind.clone()).collect();
    block_items.push(MirStmt::Closure(MirClosure {
        name: closure.name.clone(),
        target,
        args,
        env_layout,
        span: closure.span,
    }));
    Ok(block_items)
}

fn lower_exec(
    exec: &hir::Exec,
    symbols: &mut SymbolRegistry,
    locals: &mut HashSet<String>,
    generated_functions: &mut Vec<MirFunction>,
    unused_params: &mut HashSet<String>,
) -> Result<Vec<MirStmt>, Error> {
    let exec = exec.clone();
    let (mut block_items, target, args) = ensure_target(
        symbols,
        locals,
        &exec.args,
        &exec.of,
        exec.span,
        generated_functions,
    )?;
    mark_target(unused_params, &target);
    mark_args(unused_params, &args);

    if let MirExecTarget::Function(sig) = &target {
        if let Some(MirBuiltin::Call(kind)) = sig.builtin {
            let builtin_items = lower_builtin_call(sig, kind, args, exec.span, unused_params)?;
            block_items.extend(builtin_items);
            return Ok(block_items);
        }
    }
    block_items.extend(take_release_statements(unused_params));
    block_items.push(MirStmt::Exec(MirExec {
        target,
        args,
        span: exec.span,
    }));
    Ok(block_items)
}

fn lower_builtin_call(
    sig: &FunctionSig,
    kind: MirCallKind,
    mut args: Vec<MirArg>,
    span: Span,
    unused_params: &mut HashSet<String>,
) -> Result<Vec<MirStmt>, Error> {
    if args.is_empty() {
        return Err(Error::new(
            Code::Internal,
            format!("{} requires a continuation argument", kind.name()),
            span,
        ));
    }

    let continuation_arg = args.pop().unwrap();
    if args.is_empty() {
        return Err(Error::new(
            Code::Internal,
            format!(
                "{} requires at least one argument before the continuation",
                kind.name()
            ),
            span,
        ));
    }

    let call_args = args;
    let call_arg_kinds = call_args
        .iter()
        .map(|arg| arg.kind.clone())
        .collect::<Vec<_>>();
    let (_, continuation_params) = split_inputs_and_continuations(&sig.params);
    let continuation_signature = extract_continuation_signature(&continuation_params);
    let outputs = build_continuation_outputs(continuation_signature.clone(), sig.span);
    let result_name = outputs
        .first()
        .map(|arg| arg.name.clone())
        .unwrap_or_else(String::new);

    let mut stmts = Vec::new();
    let call_stmt = MirStmt::Call(MirCall {
        result: result_name.clone(),
        target: MirCallTarget::Builtin(kind),
        args: call_args,
        arg_kinds: call_arg_kinds,
        span,
    });
    stmts.push(call_stmt);

    if let Some((_, _)) = continuation_signature {
        stmts.extend(take_release_statements(unused_params));
        stmts.push(MirStmt::Exec(MirExec {
            target: MirExecTarget::Closure {
                name: continuation_arg.name,
            },
            args: outputs,
            span,
        }));
    }

    Ok(stmts)
}

// TODO: Simplify this.
fn resolve_target(name: &str, symbols: &SymbolRegistry) -> MirExecTarget {
    if let Some(sig) = symbols.get_function(name) {
        return MirExecTarget::Function(sig.clone());
    }
    MirExecTarget::Closure {
        name: name.to_string(),
    }
}

fn build_closure_unwrapper(function: &MirFunction) -> Option<MirFunction> {
    let env_param = SigItem {
        name: "env_end".to_string(),
        ty: SigKind::Int,
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

fn extract_closure_sig_info(target: &MirExecTarget, args: &[Arg]) -> Vec<MirArg> {
    if let Some(params) = target_signature(target) {
        return consume_signature_for_args(params, args);
    }
    let fallback_args = args
        .iter()
        .map(|arg| MirArg {
            name: arg.name.clone(),
            kind: SigKind::Int,
        })
        .collect();
    fallback_args
}

fn target_signature<'a>(target: &'a MirExecTarget) -> Option<&'a [SigItem]> {
    match target {
        MirExecTarget::Function(sig) => Some(&sig.params),
        MirExecTarget::Closure { .. } => None,
    }
}

fn consume_signature_for_args(params: &[SigItem], args: &[Arg]) -> Vec<MirArg> {
    let mut consumed = 0;
    let mut sig_index = 0;
    let total = params.len();
    let mut mir_args = Vec::with_capacity(args.len());
    while consumed < args.len() && sig_index < total {
        match &params[sig_index].ty {
            SigKind::Variadic => {
                let remaining_args = args.len() - consumed;
                let final_items = total.saturating_sub(sig_index + 1);
                let variadic_count = remaining_args.saturating_sub(final_items);
                for _ in 0..variadic_count {
                    mir_args.push(MirArg {
                        name: args[consumed].name.clone(),
                        kind: SigKind::Int,
                    });
                    consumed += 1;
                }
                sig_index += 1;
            }
            ty => {
                mir_args.push(MirArg {
                    name: args[consumed].name.clone(),
                    kind: ty.clone(),
                });
                consumed += 1;
                sig_index += 1;
            }
        }
    }

    while consumed < args.len() {
        mir_args.push(MirArg {
            name: args[consumed].name.clone(),
            kind: SigKind::Int,
        });
        consumed += 1;
    }

    mir_args
}

fn build_unwrapper_function(
    name: String,
    target_sig: FunctionSig,
    env_param: SigItem,
    field_sig_items: Vec<SigItem>,
    span: Span,
) -> MirFunction {
    let env_word_count = env_word_count_from_params(&field_sig_items);
    let offsets = env_word_offsets_from_params(&field_sig_items);
    let mut items = Vec::with_capacity(field_sig_items.len() + 1);

    for (idx, sig_item) in field_sig_items.iter().enumerate() {
        let offset_from_end = env_word_count as isize - offsets[idx] as isize;
        items.push(MirStmt::EnvField(MirEnvField {
            result: sig_item.name.clone(),
            env_end: env_param.name.clone(),
            field_name: sig_item.name.clone(),
            offset_from_end,
            ty: sig_item.ty.clone(),
            continuation_params: continuation_params_for_type(&sig_item.ty),
            span: sig_item.span,
        }));
    }

    items.push(MirStmt::ReleaseClosure(ReleaseClosure {
        name: env_param.name.clone(),
    }));

    let exec_args = field_sig_items
        .iter()
        .map(|item| MirArg {
            name: item.name.clone(),
            kind: item.ty.clone(),
        })
        .collect::<Vec<_>>();

    items.push(MirStmt::Exec(MirExec {
        target: MirExecTarget::Function(target_sig),
        args: exec_args,
        span,
    }));

    MirFunction {
        sig: FunctionSig {
            name,
            params: vec![env_param],
            span,
            builtin: None,
        },
        items,
    }
}

fn build_release_helper(function: &MirFunction) -> Option<MirFunction> {
    let env_param = SigItem {
        name: "env_end".to_string(),
        ty: SigKind::Int,
        has_bang: false,
        span: function.sig.span,
    };

    let offsets = env_word_offsets_from_params(&function.sig.params);
    let env_word_count = env_word_count_from_params(&function.sig.params);
    let mut items = Vec::new();
    let num_curried_binding = "__num_curried".to_string();

    let reference_fields = function
        .sig
        .params
        .iter()
        .enumerate()
        .filter_map(|(idx, param)| {
            if !is_reference_type(&param.ty) {
                return None;
            }
            Some(env_word_count - offsets[idx])
        })
        .collect::<Vec<_>>();

    if !reference_fields.is_empty() {
        items.push(MirStmt::EnvField(MirEnvField {
            result: num_curried_binding.clone(),
            env_end: env_param.name.clone(),
            field_name: "num_curried".to_string(),
            offset_from_end: -(NUM_CURRIED_METADATA_WORD_OFFSET as isize),
            ty: SigKind::Int,
            continuation_params: Vec::new(),
            span: function.sig.span,
        }));
        let done_label = format!("{}_done", closure_release_label(&function.sig.name));

        for count in (1..=reference_fields.len()).rev() {
            let skip_label = format!("{}_release_if_num_curried_lt_{}", function.sig.name, count);
            items.push(MirStmt::CondJump(MirCondJump {
                left: MirOperand::Binding(num_curried_binding.clone()),
                right: MirOperand::Literal(count as i64),
                kind: MirComparisonKind::Less,
                target: skip_label.clone(),
            }));
            for offset in reference_fields.iter().take(count) {
                items.push(MirStmt::Call(MirCall {
                    result: String::new(),
                    target: MirCallTarget::Field(MirField {
                        env_end: env_param.name.clone(),
                        offset_from_end: *offset,
                        span: function.sig.span,
                    }),
                    args: Vec::new(),
                    arg_kinds: Vec::new(),
                    span: function.sig.span,
                }));
            }
            items.push(MirStmt::Jump(MirJump {
                target: done_label.clone(),
            }));
            items.push(MirStmt::Label(MirLabel { name: skip_label }));
        }
        items.push(MirStmt::Label(MirLabel { name: done_label }));
    }

    items.push(MirStmt::Return(MirReturn { value: None }));

    Some(MirFunction {
        sig: FunctionSig {
            name: closure_release_label(&function.sig.name),
            params: vec![env_param],
            span: function.sig.span,
            builtin: None,
        },
        items,
    })
}

fn build_deep_copy_helper(function: &MirFunction) -> Option<MirFunction> {
    let env_param = SigItem {
        name: "env_end".to_string(),
        ty: SigKind::Int,
        has_bang: false,
        span: function.sig.span,
    };

    let offsets = env_word_offsets_from_params(&function.sig.params);
    let env_word_count = env_word_count_from_params(&function.sig.params);
    let mut items = Vec::new();
    let num_curried_binding = "num_curried".to_string();

    let reference_fields = function
        .sig
        .params
        .iter()
        .enumerate()
        .filter_map(|(idx, param)| {
            if !is_reference_type(&param.ty) {
                return None;
            }
            let env_offset_from_start = offsets[idx];
            Some((idx, env_offset_from_start))
        })
        .collect::<Vec<_>>();

    if !reference_fields.is_empty() {
        items.push(MirStmt::EnvField(MirEnvField {
            result: num_curried_binding.clone(),
            env_end: env_param.name.clone(),
            field_name: "num_curried".to_string(),
            offset_from_end: -(NUM_CURRIED_METADATA_WORD_OFFSET as isize),
            ty: SigKind::Int,
            continuation_params: Vec::new(),
            span: function.sig.span,
        }));
        let done_label = format!("{}_done", closure_deepcopy_label(&function.sig.name));

        for count in (1..=reference_fields.len()).rev() {
            let skip_label = format!("{}_deepcopy_if_num_curried_lt_{}", function.sig.name, count);
            items.push(MirStmt::CondJump(MirCondJump {
                left: MirOperand::Binding(num_curried_binding.clone()),
                right: MirOperand::Literal(count as i64),
                kind: MirComparisonKind::Less,
                target: skip_label.clone(),
            }));
            for &(idx, env_offset_from_start) in reference_fields.iter().take(count) {
                items.push(MirStmt::Copy(MirCopyField {
                    env_end: env_param.name.clone(),
                    offset: env_offset_from_start as isize,
                    env_word_count,
                    result: format!("{}_deepcopy_field_{}_{}", function.sig.name, count, idx),
                    span: function.sig.span,
                }));
            }
            items.push(MirStmt::Jump(MirJump {
                target: done_label.clone(),
            }));
            items.push(MirStmt::Label(MirLabel { name: skip_label }));
        }
        items.push(MirStmt::Label(MirLabel {
            name: done_label.clone(),
        }));
    }

    items.push(MirStmt::Return(MirReturn { value: None }));

    Some(MirFunction {
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
    params.iter().map(|param| words_for_type(&param.ty)).sum()
}

fn env_word_offsets_from_params(params: &[SigItem]) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(params.len());
    let mut current = 0usize;
    for param in params {
        offsets.push(current);
        current += words_for_type(&param.ty);
    }
    offsets
}

fn words_for_type(ty: &SigKind) -> usize {
    match resolved_type_kind(ty) {
        ValueKind::Word => 1,
        ValueKind::Closure => 1,
        ValueKind::Variadic => 0,
    }
}

fn is_reference_type(ty: &SigKind) -> bool {
    return matches!(ty, SigKind::Sig(_));
}

const WORD_SIZE: usize = 8;
const NUM_CURRIED_METADATA_WORD_OFFSET: usize = ENV_METADATA_NUM_CURRIED_OFFSET / WORD_SIZE;

fn resolved_type_kind(ty: &SigKind) -> ValueKind {
    match ty {
        SigKind::Int | SigKind::Str | SigKind::CompileTimeInt | SigKind::CompileTimeStr => {
            ValueKind::Word
        }
        SigKind::Variadic => ValueKind::Variadic,
        SigKind::Sig(_) => ValueKind::Closure,
        SigKind::Ident(_) => ValueKind::Word,
        _ => unreachable!("unexpected type kind in env: {:#?}", ty),
    }
}

// TODO: supicious, we should not need this to be public.
pub fn continuation_params_for_type(ty: &SigKind) -> Vec<SigKind> {
    match ty {
        SigKind::Sig(signature) => signature.kinds(),
        SigKind::Ident(_) => Vec::new(),
        _ => Vec::new(),
    }
}

fn build_builtin_bridge_function(builtin: MirBuiltin, sig: &FunctionSig) -> MirFunction {
    let (input_params, continuation_params) = split_inputs_and_continuations(&sig.params);
    let continuation_signature = extract_continuation_signature(&continuation_params);
    let outputs = build_continuation_outputs(continuation_signature.clone(), sig.span);

    let mir_inputs = input_params
        .iter()
        .map(|param| MirArg {
            name: param.name.clone(),
            kind: param.ty.clone(),
        })
        .collect::<Vec<_>>();
    let output_names = outputs
        .iter()
        .map(|arg| arg.name.clone())
        .collect::<Vec<_>>();

    let stmt = match builtin {
        MirBuiltin::Instruction(kind) => MirStmt::Op(MirInstruction {
            kind,
            opcode: kind.name(),
            operand_comments: builtin_comments(kind),
            inputs: mir_inputs.clone(),
            outputs: output_names.clone(),
            span: sig.span,
        }),
        MirBuiltin::SysCall(kind) => MirStmt::SysCall(MirSysCall {
            kind,
            operand_comments: syscall_comments(kind),
            args: mir_inputs.clone(),
            outputs: output_names.clone(),
            span: sig.span,
        }),
        MirBuiltin::Call(kind) => {
            let arg_kinds = input_params
                .iter()
                .map(|param| param.ty.clone())
                .collect::<Vec<_>>();
            let result_name = "__result".to_string();
            MirStmt::Call(MirCall {
                result: result_name,
                target: MirCallTarget::Builtin(kind),
                args: mir_inputs.clone(),
                arg_kinds,
                span: sig.span,
            })
        }
    };

    let mut items = vec![stmt];
    if let Some((continuation, _)) = continuation_signature {
        items.push(MirStmt::Exec(MirExec {
            target: MirExecTarget::Closure {
                name: continuation.name.clone(),
            },
            args: outputs.clone(),
            span: sig.span,
        }));
    }

    MirFunction {
        sig: sig.clone(),
        items,
    }
}

fn ensure_generated_once<F>(
    symbols: &mut SymbolRegistry,
    trigger_name: &str,
    generate: F,
) -> Vec<MirFunction>
where
    F: FnOnce(&mut SymbolRegistry) -> Vec<MirFunction>,
{
    if symbols.builtin_function_generated(trigger_name) {
        return Vec::new();
    }

    let functions = generate(symbols);
    for function in &functions {
        symbols.mark_builtin_function_generated(&function.sig.name);
    }
    functions
}

fn ensure_builtin_bridge_generated_once(
    symbols: &mut SymbolRegistry,
    builtin: MirBuiltin,
    sig: &FunctionSig,
) -> Vec<MirFunction> {
    ensure_generated_once(symbols, sig.name.as_str(), move |_symbols| {
        let builtin_fn = build_builtin_bridge_function(builtin, sig);
        let mut functions = vec![builtin_fn.clone()];
        if let Some(unwrapper) = build_closure_unwrapper(&builtin_fn) {
            functions.push(unwrapper);
        }
        functions
    })
}

fn extract_continuation_signature(
    continuation_params: &[SigItem],
) -> Option<(SigItem, ast::Signature)> {
    continuation_params.first().and_then(|continuation| {
        if let SigKind::Sig(signature) = &continuation.ty {
            Some((continuation.clone(), signature.clone()))
        } else {
            None
        }
    })
}

fn build_continuation_outputs(
    continuation_signature: Option<(SigItem, ast::Signature)>,
    _sig_span: Span,
) -> Vec<MirArg> {
    if let Some((continuation, signature)) = continuation_signature {
        let mut outputs = signature
            .items
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let name = if !item.name.is_empty() {
                    item.name.clone()
                } else if continuation.name.is_empty() {
                    format!("result_{}", idx)
                } else {
                    format!("{}_{}", continuation.name, idx)
                };
                MirArg {
                    name,
                    kind: item.ty.clone(),
                }
            })
            .collect::<Vec<_>>();
        if outputs.is_empty() {
            outputs.push(MirArg {
                name: "__result".to_string(),
                kind: SigKind::Int,
            });
        }
        outputs
    } else {
        Vec::new()
    }
}

pub fn args_to_kinds(args: &[MirArg]) -> Vec<SigKind> {
    args.iter().map(|arg| arg.kind.clone()).collect()
}

pub fn builtin_functions(symbols: &SymbolRegistry) -> Vec<MirFunction> {
    let mut functions = Vec::new();
    if symbols.builtin_imports().contains("itoa") {
        if let Some(sig) = symbols.get_function("itoa") {
            functions.push(MirFunction {
                sig: sig.clone(),
                items: Vec::new(),
            });
        }
    }
    functions
}

fn split_inputs_and_continuations(params: &[SigItem]) -> (Vec<SigItem>, Vec<SigItem>) {
    let mut inputs = Vec::new();
    let mut continuations = Vec::new();
    let mut seen_continuation = false;
    for param in params {
        if matches!(param.ty, SigKind::Sig(_)) {
            seen_continuation = true;
            continuations.push(param.clone());
        } else if seen_continuation {
            continuations.push(param.clone());
        } else {
            inputs.push(param.clone());
        }
    }
    (inputs, continuations)
}

fn builtin_comments(kind: MirInstKind) -> (&'static str, &'static str, &'static str) {
    match kind {
        MirInstKind::Add => ("load first integer", "add second integer", "store sum"),
        MirInstKind::Sub => ("load minuend", "subtract subtrahend", "store difference"),
        MirInstKind::Mul => (
            "load multiplicand",
            "multiply by multiplier",
            "store product",
        ),
        MirInstKind::Div => ("load dividend", "divide by divisor", "store quotient"),
        MirInstKind::EqInt => (
            "load first integer",
            "compare to second integer",
            "jump to selected continuation",
        ),
        MirInstKind::EqStr => (
            "load first string pointer",
            "compare bytes with second string",
            "jump to selected continuation",
        ),
        MirInstKind::Lt => (
            "load first integer",
            "compare to second integer",
            "branch to lesser continuation",
        ),
        MirInstKind::Gt => (
            "load first integer",
            "compare to second integer",
            "branch to greater continuation",
        ),
    }
}

fn syscall_comments(kind: MirSysCallKind) -> (&'static str, &'static str, &'static str) {
    match kind {
        MirSysCallKind::Exit => ("load exit code", "", "terminate program"),
    }
}
