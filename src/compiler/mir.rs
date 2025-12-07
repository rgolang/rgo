use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::compiler::ast::TypeRef;
use crate::compiler::error::CodegenError;
use crate::compiler::hir::{
    Apply, Arg, Block, BlockItem, Exec, Function, IntLiteral, Param, StrLiteral,
};
use crate::compiler::runtime::env_metadata_size;
use crate::compiler::span::Span;
use crate::compiler::symbol::SymbolRegistry;
use crate::compiler::type_utils::expand_alias_chain;

#[derive(Clone, Debug, Default)]
pub struct MirModule {
    pub functions: Vec<MirFunction>,
}

impl MirModule {
    pub fn new(functions: Vec<MirFunction>) -> Self {
        Self { functions }
    }

    pub fn push(&mut self, function: MirFunction) {
        self.functions.push(function);
    }
}

#[derive(Clone, Debug)]
pub struct MirFunction {
    pub name: String,
    pub params: Vec<Param>,
    pub block: MirBlock,
    pub owns_self: bool,
    pub span: Span,
}

impl MirFunction {
    pub fn lower(func: &Function, symbols: &SymbolRegistry) -> Result<Self, CodegenError> {
        let block = MirBlock::lower(&func.body, symbols, &func.params)?;
        let owns_self = detects_self_capture(&func.name, &block);
        Ok(Self {
            name: func.name.clone(),
            params: func.params.clone(),
            block,
            owns_self,
            span: func.span,
        })
    }

    pub fn builtin_internal_array_str_nth() -> Self {
        Self {
            name: "internal_array_str_nth".to_string(),
            params: Vec::new(),
            block: MirBlock {
                items: Vec::new(),
                span: Span::unknown(),
            },
            owns_self: false,
            span: Span::unknown(),
        }
    }

    pub fn builtin_internal_array_str() -> Self {
        Self {
            name: "internal_array_str".to_string(),
            params: Vec::new(),
            block: MirBlock {
                items: Vec::new(),
                span: Span::unknown(),
            },
            owns_self: false,
            span: Span::unknown(),
        }
    }
}

impl fmt::Display for MirFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params = if self.params.is_empty() {
            String::new()
        } else {
            let rendered = self
                .params
                .iter()
                .map(|param| format!("{}: {}", param.name, format_type(&param.ty)))
                .collect::<Vec<_>>()
                .join(", ");
            rendered
        };
        let root_suffix = if self.owns_self { " [root]" } else { "" };
        writeln!(f, "{}{}({}) {{", self.name, root_suffix, params)?;
        for stmt in &self.block.items {
            writeln!(f, "    {stmt}")?;
        }
        write!(f, "}}")
    }
}

fn detects_self_capture(name: &str, block: &MirBlock) -> bool {
    block.items.iter().any(|stmt| {
        matches!(
            stmt,
            MirStatement::FunctionDef(binding) if binding.function_name == name
        )
    })
}

#[derive(Clone, Debug)]
pub struct MirBlock {
    pub items: Vec<MirStatement>,
    pub span: Span,
}

impl MirBlock {
    fn lower(
        block: &Block,
        symbols: &SymbolRegistry,
        params: &[Param],
    ) -> Result<Self, CodegenError> {
        let release_plan = compute_release_plan(block, symbols, params);
        // TODO: deep_copy_plan is computed but not used yet - needs proper integration
        let _deep_copy_plan = compute_deep_copy_plan(block, symbols, params);
        let mut items = Vec::new();
        let mut ctx = BlockLoweringContext::new(params, symbols);

        for (idx, item) in block.items.iter().enumerate() {
            // Insert releases before the item if needed
            if let Some(releases) = release_plan.get(&idx) {
                for release in releases {
                    items.push(MirStatement::ReleaseEnv(release.clone()));
                }
            }

            let lowered = lower_block_item(item, symbols, &mut ctx)?;
            items.extend(lowered);
        }

        Ok(Self {
            items,
            span: block.span,
        })
    }
}
struct BlockLoweringContext<'a> {
    symbols: &'a SymbolRegistry,
    bound_names: HashSet<String>,
    temp_counter: usize,
}

impl<'a> BlockLoweringContext<'a> {
    fn new(params: &[Param], symbols: &'a SymbolRegistry) -> Self {
        let mut bound_names = HashSet::new();
        for param in params {
            bound_names.insert(param.name.clone());
        }
        Self {
            symbols,
            bound_names,
            temp_counter: 0,
        }
    }

    fn bind(&mut self, name: &str) {
        self.bound_names.insert(name.to_string());
    }

    fn ensure_function_arg(
        &mut self,
        arg: &mut Arg,
        statements: &mut Vec<MirStatement>,
    ) -> Result<(), CodegenError> {
        if self.bound_names.contains(arg.name.as_str()) {
            return Ok(());
        }
        if let Some(binding) = self.create_function_binding(&arg.name, arg.span)? {
            let temp_name = binding.name.clone();
            statements.push(MirStatement::FunctionDef(binding));
            self.bind(&temp_name);
            arg.name = temp_name;
        }
        Ok(())
    }

    fn create_function_binding(
        &mut self,
        target: &str,
        span: Span,
    ) -> Result<Option<MirFunctionBinding>, CodegenError> {
        let allocation = match function_env_allocation(target, self.symbols) {
            Some(plan) => plan,
            None => return Ok(None),
        };
        let temp_name = format!("__mir_fn_{}", self.temp_counter);
        self.temp_counter += 1;
        Ok(Some(MirFunctionBinding {
            name: temp_name,
            function_name: target.to_string(),
            span,
            env_allocation: Some(allocation),
        }))
    }
}

#[derive(Clone, Debug)]
pub enum MirStatement {
    FunctionDef(MirFunctionBinding),
    StrDef(StrLiteral),
    IntDef(IntLiteral),
    StructDef(MirStruct),
    Exec(MirExec),
    ReleaseEnv(ReleaseEnv),
    DeepCopy(DeepCopy),
}

impl fmt::Display for MirStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MirStatement::FunctionDef(binding) => {
                let label = if binding.name == binding.function_name {
                    "<function>".to_string()
                } else {
                    format!("<function {}>", binding.function_name)
                };
                if let Some(allocation) = &binding.env_allocation {
                    write!(
                        f,
                        "{} = {}; mmap env_size={} heap_size={}",
                        binding.name, label, allocation.env_size, allocation.heap_size
                    )
                } else {
                    write!(f, "{} = {}", binding.name, label)
                }
            }
            MirStatement::StrDef(StrLiteral { name, value, .. }) => {
                write!(f, "{} = \"{}\"", name, escape_literal(value))
            }
            MirStatement::IntDef(IntLiteral { name, value, .. }) => {
                write!(f, "{} = {}", name, value)
            }
            MirStatement::StructDef(apply) => {
                let args = render_args(&apply.value.args);
                write!(f, "{} = {}({})", apply.value.name, apply.value.of, args)
            }
            MirStatement::Exec(exec) => {
                let args = render_args(&exec.exec.args);
                if let Some(result) = &exec.exec.result {
                    write!(f, "{} = {}({})", result, exec.exec.of, args)
                } else {
                    write!(f, "{}({})", exec.exec.of, args)
                }
            }
            MirStatement::ReleaseEnv(ReleaseEnv { name, .. }) => {
                write!(f, "munmap {}", name)
            }
            MirStatement::DeepCopy(DeepCopy { original, copy, .. }) => {
                write!(f, "{} = deepcopy {}", copy, original)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct MirFunctionBinding {
    pub name: String,
    pub function_name: String,
    pub span: Span,
    pub env_allocation: Option<MirEnvAllocation>,
}

#[derive(Clone, Debug)]
pub struct MirEnvAllocation {
    pub env_size: usize,
    pub heap_size: usize,
}

#[derive(Clone, Debug)]
pub struct MirStruct {
    pub value: Apply,
    pub variadic: Option<MirVariadicExecInfo>,
}

#[derive(Clone, Debug)]
pub struct MirExec {
    pub exec: Exec,
    pub variadic: Option<MirVariadicExecInfo>,
}

#[derive(Clone, Debug)]
pub struct MirVariadicExecInfo {
    pub variadic_param_index: usize,
    pub prefix_len: usize,
    pub suffix_len: usize,
}

impl MirVariadicExecInfo {
    pub fn required_arguments(&self) -> usize {
        self.prefix_len + self.suffix_len
    }
}

#[derive(Clone, Debug)]
pub struct ReleaseEnv {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct DeepCopy {
    pub original: String,
    pub copy: String,
    pub span: Span,
}

fn lower_block_item(
    item: &BlockItem,
    symbols: &SymbolRegistry,
    ctx: &mut BlockLoweringContext,
) -> Result<Vec<MirStatement>, CodegenError> {
    match item {
        BlockItem::FunctionDef(function) => {
            let binding = lower_function_binding(function, symbols)?;
            ctx.bind(&binding.name);
            Ok(vec![MirStatement::FunctionDef(binding)])
        }
        BlockItem::StrDef(literal) => {
            ctx.bind(&literal.name);
            Ok(vec![MirStatement::StrDef(literal.clone())])
        }
        BlockItem::IntDef(literal) => {
            ctx.bind(&literal.name);
            Ok(vec![MirStatement::IntDef(literal.clone())])
        }
        BlockItem::ApplyDef(apply) => lower_apply_statements(apply, symbols, ctx),
        BlockItem::Exec(exec) => lower_exec_statements(exec, symbols, ctx),
    }
}

fn lower_apply_statements(
    apply: &Apply,
    symbols: &SymbolRegistry,
    ctx: &mut BlockLoweringContext,
) -> Result<Vec<MirStatement>, CodegenError> {
    let mut statements = Vec::new();
    let mut apply = apply.clone();
    for arg in &mut apply.args {
        ctx.ensure_function_arg(arg, &mut statements)?;
    }
    let lowered = lower_apply(&apply, symbols)?;
    ctx.bind(&apply.name);
    statements.push(MirStatement::StructDef(lowered));
    Ok(statements)
}

fn lower_exec_statements(
    exec: &Exec,
    symbols: &SymbolRegistry,
    ctx: &mut BlockLoweringContext,
) -> Result<Vec<MirStatement>, CodegenError> {
    // Validate builtin function arguments before transforming them
    validate_builtin_exec(exec, symbols)?;

    let mut statements = Vec::new();
    let mut exec = exec.clone();
    for arg in &mut exec.args {
        ctx.ensure_function_arg(arg, &mut statements)?;
    }
    let lowered = lower_exec(&exec, symbols)?;
    if let Some(result) = &exec.result {
        ctx.bind(result);
    }
    statements.push(MirStatement::Exec(lowered));
    Ok(statements)
}

fn lower_function_binding(
    function: &Function,
    symbols: &SymbolRegistry,
) -> Result<MirFunctionBinding, CodegenError> {
    create_binding_for_function(
        function.name.clone(),
        &function.name,
        function.span,
        symbols,
    )
}

fn create_binding_for_function(
    binding_name: String,
    function_name: &str,
    span: Span,
    symbols: &SymbolRegistry,
) -> Result<MirFunctionBinding, CodegenError> {
    let allocation = function_env_allocation(function_name, symbols).ok_or_else(|| {
        CodegenError::new(
            format!("compiler bug: unknown function '{}'", function_name),
            span,
        )
    })?;
    Ok(MirFunctionBinding {
        name: binding_name,
        function_name: function_name.to_string(),
        span,
        env_allocation: Some(allocation),
    })
}

fn function_env_allocation(
    function_name: &str,
    symbols: &SymbolRegistry,
) -> Option<MirEnvAllocation> {
    symbols.get_function(function_name).map(|sig| {
        let env_size = env_size_bytes(&sig.params, symbols);
        let pointer_count = env_pointer_count(&sig.params, symbols);
        MirEnvAllocation {
            env_size,
            heap_size: env_size + env_metadata_size(pointer_count),
        }
    })
}

fn lower_apply(apply: &Apply, symbols: &SymbolRegistry) -> Result<MirStruct, CodegenError> {
    let variadic = compute_variadic_call_info(&apply.of, apply.args.len(), apply.span, symbols)?;
    Ok(MirStruct {
        value: apply.clone(),
        variadic,
    })
}

fn validate_builtin_exec(exec: &Exec, symbols: &SymbolRegistry) -> Result<(), CodegenError> {
    let name = exec.of.as_str();

    // Validate printf continuation
    if name == "printf" {
        if exec.args.len() < 2 {
            return Err(CodegenError::new(
                "printf requires a format string and a continuation",
                exec.span,
            ));
        }

        let continuation_arg = &exec.args[exec.args.len() - 1];
        if let Some(sig) = symbols.get_function(&continuation_arg.name) {
            if !sig.params.is_empty() {
                return Err(CodegenError::new(
                    "printf continuation must accept no arguments",
                    continuation_arg.span,
                ));
            }
        }
    }

    // Validate sprintf continuation
    if name == "sprintf" {
        if exec.args.len() < 2 {
            return Err(CodegenError::new(
                "sprintf requires a format string and a continuation",
                exec.span,
            ));
        }

        let continuation_arg = &exec.args[exec.args.len() - 1];
        if let Some(sig) = symbols.get_function(&continuation_arg.name) {
            if sig.params.is_empty() {
                return Err(CodegenError::new(
                    "sprintf continuation must accept at least one string argument",
                    continuation_arg.span,
                ));
            }
            if sig.params[0] != TypeRef::Str {
                return Err(CodegenError::new(
                    "sprintf continuation must accept a string argument",
                    continuation_arg.span,
                ));
            }
        }
    }

    Ok(())
}

fn lower_exec(exec: &Exec, symbols: &SymbolRegistry) -> Result<MirExec, CodegenError> {
    validate_builtin_exec(exec, symbols)?;
    let variadic = compute_variadic_call_info(&exec.of, exec.args.len(), exec.span, symbols)?;
    Ok(MirExec {
        exec: exec.clone(),
        variadic,
    })
}

fn compute_variadic_call_info(
    of: &str,
    args_len: usize,
    span: Span,
    symbols: &SymbolRegistry,
) -> Result<Option<MirVariadicExecInfo>, CodegenError> {
    let sig = match symbols.get_function(of) {
        Some(sig) => sig,
        None => return Ok(None),
    };
    let mut variadic_index = None;
    for (idx, flag) in sig.is_variadic.iter().enumerate() {
        if *flag {
            if variadic_index.is_some() {
                return Err(CodegenError::new(
                    format!("multiple variadic parameters are not supported for '{of}'"),
                    span,
                ));
            }
            variadic_index = Some(idx);
        }
    }
    let variadic_idx = match variadic_index {
        Some(idx) => idx,
        None => return Ok(None),
    };
    let prefix_required = variadic_idx;
    let suffix_required = sig.params.len().saturating_sub(variadic_idx + 1);
    let required = prefix_required + suffix_required;
    if args_len < required {
        return Err(CodegenError::new(
            format!("function '{of}' expected at least {required} arguments but got {args_len}"),
            span,
        ));
    }
    let suffix_arg_start = args_len - suffix_required;
    if suffix_arg_start < prefix_required {
        return Err(CodegenError::new(
            format!("function '{of}' expected at least {required} arguments but got {args_len}"),
            span,
        ));
    }
    Ok(Some(MirVariadicExecInfo {
        variadic_param_index: variadic_idx,
        prefix_len: prefix_required,
        suffix_len: suffix_required,
    }))
}

fn env_size_bytes(params: &[TypeRef], symbols: &SymbolRegistry) -> usize {
    params.iter().map(|ty| bytes_for_type(ty, symbols)).sum()
}

fn env_pointer_count(params: &[TypeRef], symbols: &SymbolRegistry) -> usize {
    params
        .iter()
        .map(|ty| pointer_slots_for_type(ty, symbols))
        .sum()
}

fn bytes_for_type(ty: &TypeRef, symbols: &SymbolRegistry) -> usize {
    let mut visited = HashSet::new();
    if is_closure_type(ty, symbols, &mut visited) {
        16
    } else {
        8
    }
}

fn pointer_slots_for_type(ty: &TypeRef, symbols: &SymbolRegistry) -> usize {
    let mut visited = HashSet::new();
    if is_closure_type(ty, symbols, &mut visited) {
        1
    } else {
        0
    }
}

fn compute_release_plan(
    block: &Block,
    symbols: &SymbolRegistry,
    params: &[Param],
) -> HashMap<usize, Vec<ReleaseEnv>> {
    let mut plan = HashMap::new();
    let closure_params = closure_param_names(params, symbols);
    if closure_params.is_empty() {
        return plan;
    }
    let last_tail_call_idx = block
        .items
        .iter()
        .rposition(|item| matches!(item, BlockItem::Exec(inv) if inv.result.is_none()));
    if let Some(idx) = last_tail_call_idx {
        if let BlockItem::Exec(exec) = &block.items[idx] {
            let captured = captured_closure_params(&block.items, &closure_params);
            let mut releases = Vec::new();
            for param in &closure_params {
                if captured.contains(param) {
                    continue;
                }
                if should_release_closure_param(param, exec) {
                    releases.push(ReleaseEnv {
                        name: param.clone(),
                        span: exec.span,
                    });
                }
            }
            if !releases.is_empty() {
                plan.insert(idx, releases);
            }
        }
    }
    plan
}

/// Analyzes closure usage patterns to determine where deep copies are needed
/// Returns a map of item index -> (closure name -> copy name for subsequent uses)
fn compute_deep_copy_plan(
    block: &Block,
    symbols: &SymbolRegistry,
    params: &[Param],
) -> HashMap<usize, HashMap<String, String>> {
    let mut plan = HashMap::new();
    let closure_params = closure_param_names(params, symbols);
    if closure_params.is_empty() {
        return plan;
    }

    // Track usage count for each closure
    let mut usage_counts: HashMap<String, usize> = HashMap::new();
    let mut usage_indices: HashMap<String, Vec<usize>> = HashMap::new();

    for (item_idx, item) in block.items.iter().enumerate() {
        match item {
            BlockItem::ApplyDef(Apply { of, args, .. }) => {
                if closure_params.contains(of) {
                    *usage_counts.entry(of.clone()).or_insert(0) += 1;
                    usage_indices
                        .entry(of.clone())
                        .or_insert_with(Vec::new)
                        .push(item_idx);
                }
                for arg in args {
                    if closure_params.contains(&arg.name) {
                        *usage_counts.entry(arg.name.clone()).or_insert(0) += 1;
                        usage_indices
                            .entry(arg.name.clone())
                            .or_insert_with(Vec::new)
                            .push(item_idx);
                    }
                }
            }
            BlockItem::Exec(exec) => {
                if closure_params.contains(&exec.of) {
                    *usage_counts.entry(exec.of.clone()).or_insert(0) += 1;
                    usage_indices
                        .entry(exec.of.clone())
                        .or_insert_with(Vec::new)
                        .push(item_idx);
                }
                for arg in &exec.args {
                    if closure_params.contains(&arg.name) {
                        *usage_counts.entry(arg.name.clone()).or_insert(0) += 1;
                        usage_indices
                            .entry(arg.name.clone())
                            .or_insert_with(Vec::new)
                            .push(item_idx);
                    }
                }
            }
            BlockItem::FunctionDef(_) | BlockItem::StrDef(_) | BlockItem::IntDef(_) => {}
        }
    }

    // For closures used multiple times, insert deep copies before subsequent uses
    let mut copy_counter = 0;
    for closure_name in &closure_params {
        if let Some(count) = usage_counts.get(closure_name) {
            if *count > 1 {
                // Need to create copies for uses after the first
                if let Some(indices) = usage_indices.get(closure_name) {
                    // Skip the first index, create copies for subsequent ones
                    for &idx in indices.iter().skip(1) {
                        let copy_name = format!("__closure_copy_{}_{}", closure_name, copy_counter);
                        copy_counter += 1;
                        plan.entry(idx)
                            .or_insert_with(HashMap::new)
                            .insert(closure_name.clone(), copy_name);
                    }
                }
            }
        }
    }

    plan
}

fn closure_param_names(params: &[Param], symbols: &SymbolRegistry) -> Vec<String> {
    params
        .iter()
        .filter(|param| is_closure_param(&param.ty, symbols))
        .map(|param| param.name.clone())
        .collect()
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
            BlockItem::Exec(exec) => {
                if closure_names.contains(exec.of.as_str()) {
                    captured.insert(exec.of.clone());
                }
                for arg in &exec.args {
                    if closure_names.contains(arg.name.as_str()) {
                        captured.insert(arg.name.clone());
                    }
                }
                if let Some(result) = &exec.result {
                    if closure_names.contains(result.as_str()) {
                        captured.insert(result.clone());
                    }
                }
            }
            BlockItem::FunctionDef(_) | BlockItem::StrDef(_) | BlockItem::IntDef(_) => {}
        }
    }

    captured
}

fn should_release_closure_param(param_name: &str, exec: &Exec) -> bool {
    if exec.of == param_name {
        return false;
    }
    !exec.args.iter().any(|arg| arg.name == param_name)
}

fn is_closure_param(ty: &crate::compiler::ast::TypeRef, symbols: &SymbolRegistry) -> bool {
    let mut visited = HashSet::new();
    is_closure_type(ty, symbols, &mut visited)
}

fn is_closure_type(
    ty: &crate::compiler::ast::TypeRef,
    symbols: &SymbolRegistry,
    visited: &mut HashSet<String>,
) -> bool {
    use crate::compiler::ast::TypeRef;
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
        TypeRef::AliasInstance { .. } => {
            let mut expand_visited = HashSet::new();
            let expanded = expand_alias_chain(ty, symbols, &mut expand_visited);
            is_closure_type(&expanded, symbols, visited)
        }
        TypeRef::Generic(_) => false,
    }
}

fn render_args(args: &[crate::compiler::hir::Arg]) -> String {
    args.iter()
        .map(|arg| arg.name.clone())
        .collect::<Vec<_>>()
        .join(", ")
}

fn escape_literal(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '\\' => "\\\\".to_string(),
            '\"' => "\\\"".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join("")
}

pub fn format_type(ty: &crate::compiler::ast::TypeRef) -> String {
    use crate::compiler::ast::TypeRef;
    match ty {
        TypeRef::Int => "int".to_string(),
        TypeRef::Str => "str".to_string(),
        TypeRef::CompileTimeInt => "int!".to_string(),
        TypeRef::CompileTimeStr => "str!".to_string(),
        TypeRef::Type(params) => {
            let inner = params.iter().map(format_type).collect::<Vec<_>>();
            format!("({})", inner.join(", "))
        }
        TypeRef::Alias(name) => name.clone(),
        TypeRef::AliasInstance { name, args } => format!(
            "{}<{}>",
            name,
            args.iter().map(format_type).collect::<Vec<_>>().join(", ")
        ),
        TypeRef::Generic(name) => name.clone(),
    }
}

impl fmt::Display for MirModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, function) in self.functions.iter().enumerate() {
            writeln!(f, "{function}")?;
            if idx + 1 < self.functions.len() {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}
