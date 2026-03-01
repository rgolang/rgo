use std::collections::{BTreeSet, HashMap, HashSet};

pub use crate::compiler::air_ast::*;
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

fn conditional_builtin_branch_label(
    sig: &FunctionSig,
    continuation: &AirArg,
    branch: &str,
) -> String {
    let span = Span::unknown();
    format!(
        "{}_{}_{}_{}_{}",
        crate::sanitize_function_name(&sig.name),
        crate::sanitize_function_name(&continuation.name),
        branch,
        span.line,
        span.column
    )
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

pub struct FunctionLowerer {
    pending: HashMap<String, hir::Function>,
    lowered: HashSet<String>,
    in_progress: HashSet<String>,
    generated: Vec<AirFunction>,
}

impl FunctionLowerer {
    pub fn new(functions: HashMap<String, hir::Function>) -> Self {
        Self {
            pending: functions,
            lowered: HashSet::new(),
            in_progress: HashSet::new(),
            generated: Vec::new(),
        }
    }

    pub fn ensure(&mut self, name: &str, symbols: &mut SymbolRegistry) -> Result<(), Error> {
        if self.lowered.contains(name) || self.in_progress.contains(name) {
            return Ok(());
        }
        let function = match self.pending.remove(name) {
            Some(func) => func,
            None => {
                return Ok(());
            }
        };
        self.in_progress.insert(name.to_string());
        let lowered = lower_function(&function, symbols, self);
        self.in_progress.remove(name);
        match lowered {
            Ok(funcs) => {
                self.lowered.insert(name.to_string());
                self.generated.extend(funcs);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    pub fn take_generated_functions(self) -> Vec<AirFunction> {
        self.generated
    }
}

pub struct AirLowerContext<'a> {
    symbols: &'a mut SymbolRegistry,
    function_lowerer: &'a mut FunctionLowerer,
    locals: HashSet<String>,
    generated_functions: Vec<AirFunction>,
    unused_params: HashSet<String>,
    literals: HashMap<String, Lit>,
    closure_remaining: HashMap<String, Vec<SigKind>>, // TODO: Why is this needed?
    remaining_uses: HashMap<String, usize>,
}

impl<'a> AirLowerContext<'a> {
    pub fn new(
        symbols: &'a mut SymbolRegistry,
        function_lowerer: &'a mut FunctionLowerer,
        remaining_uses: HashMap<String, usize>,
    ) -> Self {
        Self {
            symbols,
            function_lowerer,
            locals: HashSet::new(),
            generated_functions: Vec::new(),
            unused_params: HashSet::new(),
            literals: HashMap::new(),
            closure_remaining: HashMap::new(),
            remaining_uses,
        }
    }

    pub fn push_generated_function(&mut self, function: AirFunction) {
        self.generated_functions.push(function);
    }

    pub fn into_generated_functions(self) -> Vec<AirFunction> {
        self.generated_functions
    }

    pub fn count_remaining_use(&mut self, name: &str) -> usize {
        if let Some(count) = self.remaining_uses.get_mut(name) {
            let previous = *count;
            *count = count.saturating_sub(1);
            previous
        } else {
            0
        }
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
        .map(|name| AirStmt::Op(AirOp::ReleaseHeap(AirReleaseHeap { name })))
        .collect()
}

fn prepare_args(
    ctx: &mut AirLowerContext,
    args: &[String],
    statements: &mut Vec<AirStmt>,
) -> Result<(), Error> {
    for arg in args {
        // checks whether this argument is already a local binding before generating a closure for it.
        // This prevents re-wrapping a name that's already been bound/converted earlier
        // (e.g., because it was already turned into a captured closure or defined locally) and keeps the argument list stable.
        if ctx.locals.contains(arg) {
            continue;
        }
        if let Some(closure) = create_closure(ctx, arg, None)? {
            let name = closure.name.clone();
            let remaining = closure.target.param_kinds();
            statements.push(AirStmt::Op(AirOp::NewClosure(closure)));
            ctx.locals.insert(name.clone());
            if !remaining.is_empty() {
                ctx.closure_remaining.insert(name, remaining);
            }
        }
    }
    Ok(())
}

fn create_closure(
    ctx: &mut AirLowerContext,
    target: &str,
    sig_override: Option<&mut FunctionSig>,
) -> Result<Option<AirNewClosure>, Error> {
    if sig_override.is_some() {
        return Ok(None);
    }

    if let Some(orig_sig) = ctx.symbols.get_function(target).cloned() {
        ctx.function_lowerer.ensure(target, ctx.symbols)?;
        return Ok(Some(AirNewClosure {
            target: orig_sig,
            args: Vec::new(),
            name: target.to_string(),
        }));
    }

    if let Some(builtin_name) = ctx.symbols.builtin_name_for_alias(target) {
        let builtin_sig = symbol::builtin_function_sig(builtin_name)?;
        return Ok(Some(AirNewClosure {
            target: builtin_sig,
            args: Vec::new(),
            name: target.to_string(),
        }));
    }

    Ok(None)
}

pub fn entry_function(
    entry_items: Vec<hir::BlockItem>,
    symbols: &mut SymbolRegistry,
    function_lowerer: &mut FunctionLowerer,
) -> Result<Vec<AirFunction>, Error> {
    let mut ctx = AirLowerContext::new(symbols, function_lowerer, count_block_uses(&entry_items));
    let mut items: Vec<AirStmt> = Vec::new();
    for item in entry_items.into_iter() {
        match item {
            hir::BlockItem::Import { .. }
            | hir::BlockItem::FunctionDef(..)
            | hir::BlockItem::SigDef { .. } => {} // already handled
            other => {
                items.extend(lower_block_item(&mut ctx, other)?);
            }
        }
    }
    ctx.push_generated_function(AirFunction {
        sig: FunctionSig {
            name: ENTRY_FUNCTION_NAME.into(),
            params: Vec::new(),
            generics: BTreeSet::new(),
            builtin: None,
        },
        items,
    });
    Ok(ctx.into_generated_functions())
}

pub fn lower_function(
    func: &hir::Function,
    symbols: &mut SymbolRegistry,
    function_lowerer: &mut FunctionLowerer,
) -> Result<Vec<AirFunction>, Error> {
    let sig = function_sig_from_hir(func);
    let params = sig.params.clone();
    symbols.declare_function(sig.clone())?;

    let mut ctx = AirLowerContext::new(
        symbols,
        function_lowerer,
        count_block_uses(&func.body.items),
    );
    for param in func.sig.items.iter() {
        ctx.locals.insert(param.name.clone());
        if let SigKind::Sig(signature) = &param.kind {
            ctx.closure_remaining
                .insert(param.name.clone(), signature.kinds());
        }
    }
    ctx.unused_params = collect_unused_param_refs(&params);

    let mut lowered_items: Vec<AirStmt> = Vec::new();
    for item in func.body.items.iter() {
        let lowered = lower_block_item(&mut ctx, item.clone())?;
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
    functions.extend(ctx.into_generated_functions());
    Ok(functions)
}

pub fn function_sig_from_hir(function: &hir::Function) -> FunctionSig {
    FunctionSig {
        name: function.name.clone(),
        params: air_sig_items_from_hir(&function.sig.items, &function.sig.generics),
        generics: function.sig.generics.clone(),
        builtin: None,
    }
}

fn air_sig_items_from_hir(items: &[SigItem], generics: &BTreeSet<String>) -> Vec<SigItem> {
    items
        .iter()
        .map(|item| SigItem {
            name: item.name.clone(),
            kind: air_sig_kind_from_hir(&item.kind, generics),
            has_bang: item.has_bang,
        })
        .collect()
}

fn air_sig_kind_from_hir(kind: &SigKind, generics: &BTreeSet<String>) -> SigKind {
    match kind {
        SigKind::Generic(_) => generic_pointer_kind(),
        SigKind::Ident(ident) if generics.contains(&ident.name) => generic_pointer_kind(),
        SigKind::Sig(signature) => SigKind::Sig(hir::Signature {
            items: air_sig_items_from_hir(&signature.items, generics),
            generics: signature.generics.clone(),
        }),
        SigKind::GenericInst { name, args } => SigKind::GenericInst {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| air_sig_kind_from_hir(arg, generics))
                .collect(),
        },
        other => other.clone(),
    }
}

fn generic_pointer_kind() -> SigKind {
    SigKind::Sig(hir::Signature {
        items: Vec::new(),
        generics: BTreeSet::new(),
    })
}

fn lower_block_item(
    ctx: &mut AirLowerContext,
    item: hir::BlockItem,
) -> Result<Vec<AirStmt>, Error> {
    let lowered = match item {
        hir::BlockItem::FunctionDef(..) => {
            // TODO: This should be unreachable?!
            vec![]
        }
        hir::BlockItem::LitDef { name, literal } => {
            ctx.locals.insert(name.clone());
            ctx.literals.insert(name.clone(), literal);
            vec![]
        }
        hir::BlockItem::ClosureDef(closure) => {
            if ctx.locals.contains(&closure.of) && ctx.closure_remaining.contains_key(&closure.of) {
                lower_closure_curry(&closure, ctx)?
            } else {
                lower_new_closure(&closure, ctx)?
            }
        }
        hir::BlockItem::Exec(exec) => lower_exec(&exec, ctx)?,
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
                    *uses.entry(arg.clone()).or_insert(0) += 1;
                }
            }
            hir::BlockItem::Exec(exec) => {
                *uses.entry(exec.of.clone()).or_insert(0) += 1;
                for arg in &exec.args {
                    *uses.entry(arg.clone()).or_insert(0) += 1;
                }
            }
            _ => {}
        }
    }
    uses
}

fn ensure_target(
    ctx: &mut AirLowerContext,
    args: &[String],
    target_name: &str,
) -> Result<(Vec<AirStmt>, AirExecTarget, Vec<AirArg>), Error> {
    let mut block_items = Vec::new();
    prepare_args(ctx, args, &mut block_items)?;
    let mut target = if ctx.locals.contains(target_name) {
        AirExecTarget::Closure {
            name: target_name.to_string(),
        }
    } else {
        resolve_target(target_name, ctx.symbols)?
    };
    let args = extract_closure_sig_info(&mut target, args, &ctx.literals);
    if let AirExecTarget::Function(sig) = &mut target {
        ctx.function_lowerer.ensure(&sig.name, ctx.symbols)?;
        create_closure(ctx, target_name, Some(sig))?;
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
    ctx: &mut AirLowerContext,
) -> Result<Vec<AirStmt>, Error> {
    ctx.count_remaining_use(&closure.of);
    for arg in &closure.args {
        ctx.count_remaining_use(arg);
    }
    let (mut block_items, target, args) = ensure_target(ctx, &closure.args, &closure.of)?;
    ctx.locals.insert(closure.name.clone());
    mark_target(&mut ctx.unused_params, &target);
    mark_args(&mut ctx.unused_params, &args);

    let new_remaining =
        closure_remaining_after_applying(&ctx.closure_remaining, &target, args.len());
    let target_sig = match target {
        AirExecTarget::Function(sig) => sig,
        _ => {
            return Err(Error::new(
                Code::Internal,
                "expected function target when creating new closure".to_string(),
                Span::unknown(),
            ))
        }
    };
    block_items.push(AirStmt::Op(AirOp::NewClosure(AirNewClosure {
        name: closure.name.clone(),
        target: target_sig.clone(),
        args,
    })));
    if let Some(remaining) = new_remaining {
        ctx.closure_remaining
            .insert(closure.name.clone(), remaining);
    }
    Ok(block_items)
}

fn lower_closure_curry(
    closure: &hir::Closure,
    ctx: &mut AirLowerContext,
) -> Result<Vec<AirStmt>, Error> {
    let existing_remaining = ctx
        .closure_remaining
        .get(&closure.of)
        .cloned()
        .ok_or_else(|| {
            Error::new(
                Code::Internal,
                format!("missing closure signature for '{}'", closure.of),
                Span::unknown(),
            )
        })?;

    let (mut block_items, _, _) = ensure_target(ctx, &closure.args, &closure.of)?;

    let applied = closure.args.len().min(existing_remaining.len());
    let mut args = Vec::with_capacity(closure.args.len());
    for (idx, arg) in closure.args.iter().enumerate() {
        let kind = existing_remaining.get(idx).cloned().unwrap_or(SigKind::Int);
        args.push(AirArg {
            name: arg.clone(),
            kind,
            literal: literal_for_arg(arg, &ctx.literals),
        });
    }
    mark_target(
        &mut ctx.unused_params,
        &AirExecTarget::Closure {
            name: closure.of.clone(),
        },
    );
    mark_args(&mut ctx.unused_params, &args);

    ctx.locals.insert(closure.name.clone());
    block_items.push(AirStmt::Op(AirOp::CloneClosure(AirCloneClosure {
        src: closure.of.clone(),
        dst: closure.name.clone(),
        remaining: existing_remaining.clone(),
    })));

    let mut stored_args = Vec::with_capacity(args.len());
    for (idx, arg) in args.iter().enumerate() {
        let arg_use_count = ctx.count_remaining_use(&arg.name);
        let should_clone_arg = matches!(arg.kind, SigKind::Sig(_)) && arg_use_count > 1;
        if should_clone_arg {
            let arg_remaining = ctx
                .closure_remaining
                .get(&arg.name)
                .cloned()
                .ok_or_else(|| {
                    Error::new(
                        Code::Internal,
                        format!("missing closure signature for '{}'", arg.name),
                        Span::unknown(),
                    )
                })?;
            let clone_name = format!("__{}_arg_clone_{}", closure.name, idx);
            block_items.push(AirStmt::Op(AirOp::CloneClosure(AirCloneClosure {
                src: arg.name.clone(),
                dst: clone_name.clone(),
                remaining: arg_remaining,
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
    })));

    let suffix_word_counts = suffix_word_counts(&existing_remaining);
    for (idx, arg) in stored_args.iter().take(applied).enumerate() {
        let offset_words = suffix_word_counts[idx] as isize;
        block_items.push(AirStmt::Op(AirOp::SetField(AirSetField {
            env_end: env_end_binding.clone(),
            offset: -offset_words,
            value: arg.clone(),
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
            literal: Some(Lit::Int(remaining_words)),
        },
    })));

    ctx.closure_remaining
        .insert(closure.name.clone(), remaining.clone());

    Ok(block_items)
}

fn lower_exec(exec: &hir::Exec, ctx: &mut AirLowerContext) -> Result<Vec<AirStmt>, Error> {
    let exec = exec.clone();
    ctx.count_remaining_use(&exec.of);
    for arg in &exec.args {
        ctx.count_remaining_use(arg);
    }
    let (mut block_items, target, args) = ensure_target(ctx, &exec.args, &exec.of)?;
    mark_target(&mut ctx.unused_params, &target);
    mark_args(&mut ctx.unused_params, &args);

    if let AirExecTarget::Function(sig) = &target {
        if let Some(builtin) = sig.builtin {
            if builtin.is_call() {
                let builtin_items = lower_builtin_call(sig, builtin, args, &mut ctx.unused_params)?;
                block_items.extend(builtin_items);
                return Ok(block_items);
            }
            if is_inline_builtin(builtin) {
                for continuation in &sig.params {
                    if matches!(continuation.kind, SigKind::Sig(_)) {
                        mark_target(
                            &mut ctx.unused_params,
                            &AirExecTarget::Closure {
                                name: continuation.name.clone(),
                            },
                        );
                    }
                }
                block_items.extend(take_release_statements(&mut ctx.unused_params));
                block_items.extend(build_builtin_statements(sig, builtin, args));
                return Ok(block_items);
            }
        }
    }

    block_items.extend(take_release_statements(&mut ctx.unused_params));
    match target {
        AirExecTarget::Function(sig) => {
            block_items.push(AirStmt::Op(AirOp::JumpArgs(AirJumpArgs {
                target: sig,
                args,
            })));
        }
        AirExecTarget::Closure { name } => {
            block_items.push(AirStmt::Op(AirOp::JumpClosure(AirJumpClosure {
                env_end: name,
                args,
            })));
        }
    }
    Ok(block_items)
}

fn lower_builtin_call(
    sig: &FunctionSig,
    builtin: builtins::Builtin,
    args: Vec<AirArg>,
    unused_params: &mut HashSet<String>,
) -> Result<Vec<AirStmt>, Error> {
    let mut stmts = Vec::new();

    // Continuation is always the last param + last arg
    if let (Some(param), Some(arg)) = (sig.params.last(), args.last()) {
        if matches!(param.kind, SigKind::Sig(_)) {
            mark_target(
                unused_params,
                &AirExecTarget::Closure {
                    name: arg.name.clone(),
                },
            );
        }
    }

    stmts.extend(take_release_statements(unused_params));
    stmts.push(AirStmt::Op(call_op(builtin, args)));

    Ok(stmts)
}

// TODO: Simplify this.
fn resolve_target(name: &str, symbols: &SymbolRegistry) -> Result<AirExecTarget, Error> {
    if let Some(sig) = symbols.get_function(name) {
        return Ok(AirExecTarget::Function(sig.clone()));
    }
    if let Some(builtin_name) = symbols.builtin_name_for_alias(name) {
        let sig = symbol::builtin_function_sig(builtin_name)?;
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
    };

    Some(build_unwrapper_function(
        closure_unwrapper_label(&function.sig.name),
        function.sig.clone(),
        env_param,
        function.sig.params.clone(),
    ))
}

fn extract_closure_sig_info(
    target: &AirExecTarget,
    args: &[String],
    literals: &HashMap<String, Lit>,
) -> Vec<AirArg> {
    if let Some(params) = target_signature(target) {
        return consume_signature_for_args(params, args, literals);
    }
    let fallback_args = args
        .iter()
        .map(|arg| AirArg {
            name: arg.clone(),
            kind: SigKind::Int,
            literal: literal_for_arg(arg, literals),
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
    args: &[String],
    literals: &HashMap<String, Lit>,
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
                        name: args[consumed].clone(),
                        kind: SigKind::Int,
                        literal: literal_for_arg(&args[consumed], literals),
                    });
                    consumed += 1;
                }
                sig_index += 1;
            }
            ty => {
                air_args.push(AirArg {
                    name: args[consumed].clone(),
                    kind: ty.clone(),
                    literal: literal_for_arg(&args[consumed], literals),
                });
                consumed += 1;
                sig_index += 1;
            }
        }
    }

    while consumed < args.len() {
        air_args.push(AirArg {
            name: args[consumed].clone(),
            kind: SigKind::Int,
            literal: literal_for_arg(&args[consumed], literals),
        });
        consumed += 1;
    }

    air_args
}

fn literal_for_arg(name: &str, literals: &HashMap<String, Lit>) -> Option<Lit> {
    literals.get(name).cloned()
}

fn build_unwrapper_function(
    name: String,
    target_sig: FunctionSig,
    env_param: SigItem,
    field_sig_items: Vec<SigItem>,
) -> AirFunction {
    let env_end_reg = "__env_end".to_string();
    let env_word_count = env_word_count_from_params(&field_sig_items);
    let env_word_count_isize = env_word_count as isize;
    let offsets = env_word_offsets_from_params(&field_sig_items);
    let mut items = Vec::with_capacity(field_sig_items.len() + 1);

    items.push(AirStmt::Op(AirOp::Pin(AirPin {
        result: env_end_reg.clone(),
        value: AirValue::Binding(env_param.name.clone()),
    })));

    for (idx, sig_item) in field_sig_items.iter().enumerate() {
        let offset = offsets[idx] as isize - env_word_count_isize;
        items.push(AirStmt::Op(AirOp::Field(AirField {
            result: sig_item.name.clone(),
            ptr: env_end_reg.clone(),
            offset,
            kind: sig_item.kind.clone(),
        })));
    }

    items.push(AirStmt::Op(AirOp::ReleaseHeap(AirReleaseHeap {
        name: env_end_reg.clone(),
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
            items.extend(build_builtin_statements(&target_sig, builtin, builtin_args));
        } else {
            items.push(AirStmt::Op(AirOp::JumpArgs(AirJumpArgs {
                target: target_sig.clone(),
                args: builtin_args,
            })));
        }
    } else {
        items.push(AirStmt::Op(AirOp::JumpArgs(AirJumpArgs {
            target: target_sig.clone(),
            args: builtin_args,
        })));
    }

    AirFunction {
        sig: FunctionSig {
            name,
            params: vec![env_param],
            generics: BTreeSet::new(),
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
        })));
        for (idx, offset, offset_from_end, kind) in &reference_fields {
            let skip_label = format!("{}_release_skip_{}", function.sig.name, idx);
            let threshold = offset_from_end.saturating_sub(1);
            items.push(AirStmt::Op(AirOp::JumpGt(AirJumpGt {
                left: AirValue::Binding(num_remaining_binding.clone()),
                right: AirValue::Literal(threshold as i64),
                target: skip_label.clone(),
            })));
            let location = format!("{}_release_field_{}", function.sig.name, idx);
            items.push(AirStmt::Op(AirOp::Field(AirField {
                result: location.clone(),
                ptr: env_end_reg.clone(),
                offset: *offset,
                kind: kind.clone(),
            })));
            items.push(AirStmt::Op(AirOp::CallPtr(AirCallPtr {
                target: AirCallPtrTarget::Binding(location),
            })));
            items.push(AirStmt::Label(AirLabel { name: skip_label }));
        }
    }

    items.push(AirStmt::Op(AirOp::ReleaseHeap(AirReleaseHeap {
        name: env_end_reg.clone(),
    })));

    items.push(AirStmt::Op(AirOp::Return(AirReturn { value: None })));

    Some(AirFunction {
        sig: FunctionSig {
            name: closure_deep_release_label(&function.sig.name),
            params: vec![env_param],
            generics: BTreeSet::new(),
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
    };

    let offsets = env_word_offsets_from_params(&function.sig.params);
    let env_word_count = env_word_count_from_params(&function.sig.params);
    let mut items = Vec::new();
    let num_remaining_binding = "num_remaining".to_string();
    let env_end_reg = "__env_end".to_string();

    items.push(AirStmt::Op(AirOp::Pin(AirPin {
        result: env_end_reg.clone(),
        value: AirValue::Binding(env_param.name.clone()),
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
        })));
        for (idx, env_offset_from_start, kind) in &reference_fields {
            let skip_label = format!("{}_deepcopy_skip_{}", function.sig.name, idx);
            let offset_from_end = env_word_count.saturating_sub(*env_offset_from_start);
            let threshold = offset_from_end.saturating_sub(1);
            items.push(AirStmt::Op(AirOp::JumpGt(AirJumpGt {
                left: AirValue::Binding(num_remaining_binding.clone()),
                right: AirValue::Literal(threshold as i64),
                target: skip_label.clone(),
            })));
            items.push(AirStmt::Op(AirOp::CopyField(AirField {
                result: format!("{}_deepcopy_field_{}", function.sig.name, idx),
                ptr: env_end_reg.clone(),
                offset: -(offset_from_end as isize),
                kind: kind.clone(),
            })));
            items.push(AirStmt::Label(AirLabel { name: skip_label }));
        }
    }

    items.push(AirStmt::Op(AirOp::Return(AirReturn { value: None })));

    Some(AirFunction {
        sig: FunctionSig {
            name: closure_deepcopy_label(&function.sig.name),
            params: vec![env_param],
            generics: BTreeSet::new(),
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

fn instruction_op(builtin: builtins::Builtin, args: Vec<AirArg>) -> AirOp {
    let arg_len = args.len();
    let continuation_target = args
        .last()
        .expect("builtin invocation requires a continuation target")
        .name
        .clone();
    let inputs = args[..arg_len - 1].to_vec();

    match builtin {
        builtins::Builtin::Add => {
            let (input_a, input_b) = binary_input_args(builtin.name(), inputs); // TODO: Maybe use a dedicated instruction_op alternative for this?
            AirOp::Add(AirAdd {
                input_a,
                input_b,
                target: continuation_target,
            })
        }
        builtins::Builtin::AddF64 => {
            let (input_a, input_b) = binary_input_args(builtin.name(), inputs);
            AirOp::AddF64(AirAddF64 {
                input_a,
                input_b,
                target: continuation_target,
            })
        }
        builtins::Builtin::Sub => {
            let (input_a, input_b) = binary_input_args(builtin.name(), inputs);
            AirOp::Sub(AirSub {
                input_a,
                input_b,
                target: continuation_target,
            })
        }
        builtins::Builtin::Mul => {
            let (input_a, input_b) = binary_input_args(builtin.name(), inputs);
            AirOp::Mul(AirMul {
                input_a,
                input_b,
                target: continuation_target,
            })
        }
        builtins::Builtin::MulF64 => {
            let (input_a, input_b) = binary_input_args(builtin.name(), inputs);
            AirOp::MulF64(AirMulF64 {
                input_a,
                input_b,
                target: continuation_target,
            })
        }
        builtins::Builtin::Div => {
            let err_target = args
                .get(arg_len - 2)
                .expect("div requires an error continuation")
                .name
                .clone();
            let (input_a, input_b) =
                binary_input_args(builtin.name(), args[..arg_len - 2].to_vec());
            AirOp::DivInt(AirDivInt {
                input_a,
                input_b,
                err_target,
                ok_target: continuation_target,
            })
        }
        builtins::Builtin::DivF64 => {
            let (input_a, input_b) = binary_input_args(builtin.name(), inputs);
            AirOp::DivF64(AirDivF64 {
                input_a,
                input_b,
                target: continuation_target,
            })
        }
        builtins::Builtin::Eq | builtins::Builtin::Eqi => AirOp::JumpEqInt(AirJumpEq {
            args: inputs,
            target: continuation_target,
        }),
        builtins::Builtin::Eqs => AirOp::JumpEqStr(AirJumpEq {
            args: inputs,
            target: continuation_target,
        }),
        builtins::Builtin::Lt => {
            let (left, right) = binary_operands(builtin.name(), inputs);
            AirOp::JumpLt(AirJumpLt {
                left,
                right,
                target: continuation_target,
            })
        }
        builtins::Builtin::Gt => {
            let (left, right) = binary_operands(builtin.name(), inputs);
            AirOp::JumpGt(AirJumpGt {
                left,
                right,
                target: continuation_target,
            })
        }
        _ => unreachable!("unexpected instruction op: {}", builtin.name()),
    }
}

fn binary_operands(name: &str, inputs: Vec<AirArg>) -> (AirValue, AirValue) {
    let (input_a, input_b) = binary_input_args(name, inputs);
    (arg_to_operand(input_a), arg_to_operand(input_b))
}

fn binary_input_args(name: &str, inputs: Vec<AirArg>) -> (AirArg, AirArg) {
    let mut iter = inputs.into_iter();
    let input_a = iter
        .next()
        .unwrap_or_else(|| panic!("{} requires two operands", name));
    let input_b = iter
        .next()
        .unwrap_or_else(|| panic!("{} requires two operands", name));
    (input_a, input_b)
}

fn arg_to_operand(arg: AirArg) -> AirValue {
    if let Some(literal) = arg.literal {
        match literal {
            Lit::Int(value) => AirValue::Literal(value as i64),
            Lit::Str(_) => panic!("unexpected string literal in numeric operation"),
            Lit::F64(_) => panic!("unexpected float literal in integer numeric operation"),
        }
    } else {
        AirValue::Binding(arg.name)
    }
}

fn call_op(builtin: builtins::Builtin, args: Vec<AirArg>) -> AirOp {
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
        }),
        builtins::Builtin::Sprintf => AirOp::Sprintf(AirSprintf {
            args: call_args,
            arg_kinds,
            target: continuation_target,
        }),
        builtins::Builtin::Write => AirOp::Write(AirWrite {
            args: call_args,
            arg_kinds,
            target: continuation_target,
        }),
        builtins::Builtin::Puts => AirOp::Puts(AirPuts {
            args: call_args,
            arg_kinds,
            target: continuation_target,
        }),
        builtins::Builtin::Exit => AirOp::SysExit(AirSysExit { args }),
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
    let true_label = conditional_builtin_branch_label(sig, true_cont, "true");
    let false_label = conditional_builtin_branch_label(sig, false_cont, "false");

    let eq_jump = if matches!(builtin, builtins::Builtin::Eqs) {
        AirOp::JumpEqStr(AirJumpEq {
            args: inputs.clone(),
            target: true_label.clone(),
        })
    } else {
        AirOp::JumpEqInt(AirJumpEq {
            args: inputs.clone(),
            target: true_label.clone(),
        })
    };

    vec![
        AirStmt::Op(eq_jump),
        AirStmt::Label(AirLabel {
            name: false_label.clone(),
        }),
        AirStmt::Op(AirOp::ReleaseHeap(AirReleaseHeap {
            name: true_cont.name.clone(),
        })),
        AirStmt::Op(AirOp::JumpClosure(AirJumpClosure {
            env_end: false_cont.name.clone(),
            args: Vec::new(),
        })),
        AirStmt::Label(AirLabel {
            name: true_label.clone(),
        }),
        AirStmt::Op(AirOp::ReleaseHeap(AirReleaseHeap {
            name: false_cont.name.clone(),
        })),
        AirStmt::Op(AirOp::JumpClosure(AirJumpClosure {
            env_end: true_cont.name.clone(),
            args: Vec::new(),
        })),
    ]
}

fn build_builtin_statements(
    sig: &FunctionSig,
    builtin: builtins::Builtin,
    args: Vec<AirArg>,
) -> Vec<AirStmt> {
    if builtin.is_conditional() {
        return build_conditional_builtin_bridge(sig, builtin, args);
    }

    if builtin.is_instruction() {
        return vec![AirStmt::Op(instruction_op(builtin, args))];
    }

    if builtin.is_libc_call() {
        return vec![AirStmt::Op(call_op(builtin, args))];
    }

    vec![AirStmt::Op(AirOp::SysExit(AirSysExit { args }))]
}

fn is_inline_builtin(builtin: builtins::Builtin) -> bool {
    return matches!(
        builtin,
        builtins::Builtin::Add
            | builtins::Builtin::Sub
            | builtins::Builtin::Mul
            | builtins::Builtin::Div
            | builtins::Builtin::AddF64
            | builtins::Builtin::MulF64
            | builtins::Builtin::DivF64
            | builtins::Builtin::Eq
            | builtins::Builtin::Eqi
            | builtins::Builtin::Eqs
            | builtins::Builtin::Lt
            | builtins::Builtin::Gt
    );
}
