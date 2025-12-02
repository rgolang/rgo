use std::collections::{HashMap, HashSet};
use std::io::Write;

use crate::compiler::ast::TypeRef;
use crate::compiler::codegen_model::{AsmFunction, AsmLiteral, AsmModule, AsmStatement};
use crate::compiler::error::CodegenError;
use crate::compiler::hir::{
    Arg, Block, BlockItem, Function, Invocation, Param, ReleaseEnv, ENTRY_FUNCTION_NAME,
};
use crate::compiler::span::Span;
use crate::compiler::symbol::{FunctionSig, SymbolRegistry};

#[allow(dead_code)]
#[derive(Clone, Debug)]
enum Expr {
    Int {
        value: i64,
        span: Span,
    },
    Ident {
        name: String,
        span: Span,
    },
    String {
        value: String,
        span: Span,
    },
    Apply {
        of: String,
        args: Vec<Expr>,
        span: Span,
    },
    Lambda {
        params: Vec<Param>,
        body: Block,
        span: Span,
    },
}

const ARG_REGS: [&str; 8] = ["rdi", "rsi", "rdx", "rcx", "r8", "r9", "r10", "r11"];
const SYSCALL_MMAP: i32 = 9;
const SYSCALL_MUNMAP: i32 = 11;
const SYSCALL_EXIT: i32 = 60;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 2;
const MAP_ANONYMOUS: i32 = 32;
const ENV_METADATA_FIELD_SIZE: usize = 8;
const ENV_METADATA_SIZE: usize = ENV_METADATA_FIELD_SIZE * 2;
const FMT_BUFFER_SIZE: usize = 1024;

pub fn write_preamble<W: Write>(out: &mut W) -> Result<(), CodegenError> {
    writeln!(out, "bits 64")?;
    writeln!(out, "default rel")?;
    writeln!(out, "section .text")?;
    Ok(())
}

#[derive(Clone, Debug)]
enum GlobalValue {
    Str { name: String, literal_label: String },
    Int { name: String, value: i64 },
}

pub fn emit_builtin_definitions<W: Write>(
    symbols: &SymbolRegistry,
    ctx: &mut CodegenContext,
    out: &mut W,
) -> Result<(), CodegenError> {
    let mut builtins: Vec<&String> = symbols.builtin_imports().iter().collect();
    builtins.sort();
    for name in builtins {
        match name.as_str() {
            "add" => emit_builtin_add(out)?,
            "div" => emit_builtin_div(out)?,
            "eqi" => emit_builtin_eqi(out)?,
            "eqs" => emit_builtin_eqs(out)?,
            "gt" => emit_builtin_gt(out)?,
            "itoa" => emit_builtin_itoa(ctx, out)?,
            "lt" => emit_builtin_lt(out)?,
            "mul" => emit_builtin_mul(out)?,
            "rgo_write" => emit_builtin_write(ctx, out)?,
            "sub" => emit_builtin_sub(out)?,
            _ => {}
        }
    }
    Ok(())
}

pub fn function<W: Write>(
    func: Function,
    symbols: &SymbolRegistry,
    ctx: &mut CodegenContext,
    out: &mut W,
) -> Result<(), CodegenError> {
    let frame = FrameLayout::build(&func, symbols)?;
    let mut emitter = FunctionEmitter::new(func.clone(), out, frame, symbols, ctx);
    emitter.emit_function()?;
    if func.name != ENTRY_FUNCTION_NAME {
        emit_closure_wrapper(&func, symbols, out)?;
    }
    Ok(())
}

pub struct CodegenContext {
    string_literals: Vec<String>,
    string_map: HashMap<String, usize>,
    externs: HashSet<String>,
    globals: Vec<GlobalValue>,
    global_names: HashSet<String>,
    asm_functions: Vec<AsmFunction>,
}

impl CodegenContext {
    pub fn new() -> Self {
        Self {
            string_literals: Vec::new(),
            string_map: HashMap::new(),
            externs: HashSet::new(),
            globals: Vec::new(),
            global_names: HashSet::new(),
            asm_functions: Vec::new(),
        }
    }

    pub fn string_literal_label(&mut self, value: &str) -> String {
        if let Some(&idx) = self.string_map.get(value) {
            return Self::literal_label(idx);
        }
        let value = value.to_string();
        let idx = self.string_literals.len();
        self.string_literals.push(value.clone());
        self.string_map.insert(value, idx);
        Self::literal_label(idx)
    }

    pub fn emit_data<W: Write>(&self, out: &mut W) -> Result<(), CodegenError> {
        if self.string_literals.is_empty() && self.globals.is_empty() {
            return Ok(());
        }
        writeln!(out, "section .rodata")?;
        for (idx, literal) in self.string_literals.iter().enumerate() {
            let label = Self::literal_label(idx);
            writeln!(out, "{}:", label)?;
            let escaped = Self::escape_literal(literal);
            writeln!(out, "    db {}, 0", escaped)?;
        }
        for global in &self.globals {
            match global {
                GlobalValue::Str {
                    name,
                    literal_label,
                } => {
                    writeln!(out, "{}:", name)?;
                    writeln!(out, "    dq {}", literal_label)?;
                }
                GlobalValue::Int { name, value } => {
                    writeln!(out, "{}:", name)?;
                    writeln!(out, "    dq {}", value)?;
                }
            }
        }
        Ok(())
    }

    pub fn add_extern(&mut self, name: &str) {
        self.externs.insert(name.to_string());
    }

    pub fn register_global_str(&mut self, name: &str, value: &str) {
        if self.global_names.contains(name) {
            return;
        }
        let label = self.string_literal_label(value);
        let name_owned = name.to_string();
        self.global_names.insert(name_owned.clone());
        self.globals.push(GlobalValue::Str {
            name: name_owned,
            literal_label: label,
        });
    }

    pub fn register_global_int(&mut self, name: &str, value: i64) {
        if self.global_names.contains(name) {
            return;
        }
        let name_owned = name.to_string();
        self.global_names.insert(name_owned.clone());
        self.globals.push(GlobalValue::Int {
            name: name_owned,
            value,
        });
    }

    pub fn is_global_value(&self, name: &str) -> bool {
        self.global_names.contains(name)
    }

    pub fn emit_externs<W: Write>(&self, out: &mut W) -> Result<(), CodegenError> {
        if self.externs.is_empty() {
            return Ok(());
        }
        let mut externs: Vec<&String> = self.externs.iter().collect();
        externs.sort();
        for name in externs {
            writeln!(out, "extern {}", name)?;
        }
        Ok(())
    }

    pub fn push_asm_function(&mut self, function: AsmFunction) {
        self.asm_functions.push(function);
    }

    pub fn take_asm_module(&mut self) -> AsmModule {
        let functions = std::mem::take(&mut self.asm_functions);
        AsmModule::new(functions)
    }

    fn literal_label(idx: usize) -> String {
        format!("str_literal_{}", idx)
    }

    fn escape_literal(literal: &str) -> String {
        fn append_part(output: &mut String, part: &str) {
            if !output.is_empty() {
                output.push_str(", ");
            }
            output.push_str(part);
        }

        fn flush_chunk(output: &mut String, chunk: &mut Vec<u8>) {
            if chunk.is_empty() {
                return;
            }
            let mut literal = String::from("\"");
            for &byte in chunk.iter() {
                match byte {
                    b'"' => literal.push_str("\\\""),
                    other => literal.push(other as char),
                }
            }
            literal.push('"');
            append_part(output, &literal);
            chunk.clear();
        }

        let mut output = String::new();
        let mut chunk = Vec::new();
        for &byte in literal.as_bytes() {
            match byte {
                b'\n' => {
                    flush_chunk(&mut output, &mut chunk);
                    append_part(&mut output, "10");
                }
                b'\r' => {
                    flush_chunk(&mut output, &mut chunk);
                    append_part(&mut output, "13");
                }
                b'\t' => {
                    flush_chunk(&mut output, &mut chunk);
                    append_part(&mut output, "9");
                }
                b if b == b'\\' || b == b'"' || b == b' ' || (0x21..=0x7e).contains(&b) => {
                    chunk.push(byte);
                }
                other => {
                    flush_chunk(&mut output, &mut chunk);
                    append_part(&mut output, &format!("0x{other:02x}"));
                }
            }
        }

        flush_chunk(&mut output, &mut chunk);

        if output.is_empty() {
            return "\"\"".to_string();
        }
        output
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ValueKind {
    Word,
    Closure,
}

#[derive(Clone, Debug)]
struct Binding {
    offset: i32,
    kind: ValueKind,
    continuation_params: Vec<TypeRef>,
}

impl Binding {
    fn slot_addr(&self, slot: usize) -> i32 {
        self.offset - (slot as i32) * 8
    }
}

struct FrameLayout {
    bindings: HashMap<String, Binding>,
    stack_size: i32,
    next_offset: i32,
}

impl FrameLayout {
    fn build(func: &Function, symbols: &SymbolRegistry) -> Result<Self, CodegenError> {
        let mut layout = Self {
            bindings: HashMap::new(),
            stack_size: 0,
            next_offset: 0,
        };
        for param in &func.params {
            layout.allocate_param(param, symbols)?;
        }
        for stmt in &func.body.items {
            if let Some((name, span)) = stmt.binding_info() {
                layout.allocate_word(name, span)?;
            }
        }
        layout.stack_size = align_to(layout.next_offset as usize, 16) as i32;
        Ok(layout)
    }

    fn allocate_param(
        &mut self,
        param: &Param,
        symbols: &SymbolRegistry,
    ) -> Result<(), CodegenError> {
        let name = &param.name;
        let ty = &param.ty;
        let kind = resolved_type_kind(ty, symbols);
        let continuation_params = match kind {
            ValueKind::Closure => continuation_params_for_type(ty, symbols),
            ValueKind::Word => Vec::new(),
        };
        self.allocate_binding(name, param.span, kind, continuation_params)
    }

    fn allocate_word(&mut self, name: &str, span: Span) -> Result<(), CodegenError> {
        self.allocate_binding(name, span, ValueKind::Word, Vec::new())
    }

    fn allocate_binding(
        &mut self,
        name: &str,
        span: Span,
        kind: ValueKind,
        continuation_params: Vec<TypeRef>,
    ) -> Result<(), CodegenError> {
        if self.bindings.contains_key(name) {
            return Err(CodegenError::new(
                format!("duplicate binding '{}'", name),
                span,
            ));
        }
        self.next_offset += 16;
        self.bindings.insert(
            name.to_string(),
            Binding {
                offset: self.next_offset,
                kind,
                continuation_params,
            },
        );
        Ok(())
    }

    fn binding(&self, name: &str) -> Option<&Binding> {
        self.bindings.get(name)
    }

    fn binding_mut(&mut self, name: &str) -> Option<&mut Binding> {
        self.bindings.get_mut(name)
    }
}

#[derive(Clone, Debug)]
struct ClosureState {
    remaining: Vec<TypeRef>,
}

impl ClosureState {
    fn new(params: Vec<TypeRef>) -> Self {
        Self { remaining: params }
    }

    fn remaining(&self) -> &[TypeRef] {
        &self.remaining
    }

    fn after_applying(&self, count: usize) -> Self {
        Self {
            remaining: self.remaining[count..].to_vec(),
        }
    }
}

#[derive(Clone, Debug)]
enum ExprValue {
    Word,
    Closure(ClosureState),
}

struct FunctionEmitter<'a, W: Write> {
    func: Function,
    out: &'a mut W,
    frame: FrameLayout,
    symbols: &'a SymbolRegistry,
    ctx: &'a mut CodegenContext,
    literal_strings: HashMap<String, String>,
    asm_builder: Option<AsmFunctionBuilder>,
    terminated: bool,
    write_loop_counter: usize,
}

impl<'a, W: Write> FunctionEmitter<'a, W> {
    fn new(
        func: Function,
        out: &'a mut W,
        frame: FrameLayout,
        symbols: &'a SymbolRegistry,
        ctx: &'a mut CodegenContext,
    ) -> Self {
        let builder = AsmFunctionBuilder::new(&func);
        Self {
            func,
            out,
            frame,
            symbols,
            ctx,
            literal_strings: HashMap::new(),
            asm_builder: Some(builder),
            terminated: false,
            write_loop_counter: 0,
        }
    }

    fn emit_function(&mut self) -> Result<(), CodegenError> {
        self.write_header()?;
        self.write_prologue()?;
        self.store_params()?;
        let body = self.func.body.clone();
        self.emit_block(&body)?;
        if !self.terminated {
            self.write_epilogue()?;
        }
        if let Some(builder) = self.asm_builder.take() {
            self.ctx.push_asm_function(builder.build());
        }
        Ok(())
    }

    fn builder(&mut self) -> &mut AsmFunctionBuilder {
        self.asm_builder
            .as_mut()
            .expect("asm builder already consumed")
    }

    fn write_header(&mut self) -> Result<(), CodegenError> {
        writeln!(self.out, "global {}", self.func.name)?;
        writeln!(self.out, "{}:", self.func.name)?;
        Ok(())
    }

    fn write_prologue(&mut self) -> Result<(), CodegenError> {
        writeln!(self.out, "    push rbp ; save caller frame pointer")?;
        writeln!(self.out, "    mov rbp, rsp ; establish new frame base")?;
        if self.frame.stack_size > 0 {
            writeln!(
                self.out,
                "    sub rsp, {} ; reserve stack space for locals",
                self.frame.stack_size
            )?;
        }
        Ok(())
    }

    fn write_epilogue(&mut self) -> Result<(), CodegenError> {
        writeln!(self.out, "    leave ; epilogue: restore rbp and rsp")?;
        writeln!(self.out, "    mov rax, {} ; exit syscall", SYSCALL_EXIT)?;
        writeln!(self.out, "    xor rdi, rdi")?;
        writeln!(self.out, "    syscall")?;
        Ok(())
    }

    fn store_params(&mut self) -> Result<(), CodegenError> {
        let mut slot = 0usize;
        for param in &self.func.params {
            let name = &param.name;
            let ty = &param.ty;
            let binding = self
                .frame
                .binding(name)
                .ok_or_else(|| CodegenError::new("missing binding", param.span))?
                .clone();
            let required = slots_for_type(ty, self.symbols);
            if slot + required > ARG_REGS.len() {
                return Err(CodegenError::new(
                    format!(
                        "function '{}' supports at most {} argument slots",
                        self.func.name,
                        ARG_REGS.len()
                    ),
                    self.func.span,
                ));
            }
            match resolved_type_kind(ty, self.symbols) {
                ValueKind::Word => {
                    let reg = ARG_REGS[slot];
                    writeln!(
                        self.out,
                        "    mov [rbp-{}], {} ; store scalar arg in frame",
                        binding.slot_addr(0),
                        reg
                    )?;
                }
                ValueKind::Closure => {
                    let reg_code = ARG_REGS[slot];
                    let reg_env = ARG_REGS[slot + 1];
                    writeln!(
                        self.out,
                        "    mov [rbp-{}], {} ; save closure code pointer",
                        binding.slot_addr(0),
                        reg_code
                    )?;
                    writeln!(
                        self.out,
                        "    mov [rbp-{}], {} ; save closure environment pointer",
                        binding.slot_addr(1),
                        reg_env
                    )?;
                }
            }
            slot += required;
        }
        Ok(())
    }

    fn emit_block(&mut self, block: &Block) -> Result<(), CodegenError> {
        for stmt in &block.items {
            self.emit_statement(stmt)?;
        }
        Ok(())
    }

    fn emit_statement(&mut self, stmt: &BlockItem) -> Result<(), CodegenError> {
        match stmt {
            BlockItem::FunctionDef(function) => {
                let expr = Expr::Ident {
                    name: function.name.clone(),
                    span: function.span,
                };
                let result = self.emit_expr(&expr)?;
                self.store_binding_value(&function.name, result, function.span)?;
                return Ok(());
            }
            BlockItem::StrDef(literal) => {
                self.literal_strings
                    .insert(literal.name.clone(), literal.value.clone());
                let expr = Expr::String {
                    value: literal.value.clone(),
                    span: literal.span,
                };
                let value = self.emit_expr(&expr)?;
                self.builder()
                    .record_literal(&literal.name, AsmLiteral::Str(literal.value.clone()));
                self.store_binding_value(&literal.name, value, literal.span)?;
                return Ok(());
            }
            BlockItem::IntDef(literal) => {
                let expr = Expr::Int {
                    value: literal.value,
                    span: literal.span,
                };
                let value = self.emit_expr(&expr)?;
                self.builder()
                    .record_literal(&literal.name, AsmLiteral::Int(literal.value));
                self.store_binding_value(&literal.name, value, literal.span)?;
                return Ok(());
            }
            BlockItem::ApplyDef(apply) => {
                let invocation = Invocation {
                    of: apply.of.clone(),
                    args: apply.args.clone(),
                    span: apply.span,
                    result: None,
                };
                let was_terminated = self.terminated;
                let value = self.emit_invocation(&invocation)?;
                let tail = self.terminated && !was_terminated;
                let args = invocation
                    .args
                    .iter()
                    .map(|arg| arg.name.clone())
                    .collect::<Vec<_>>();
                let callee = invocation.of.clone();
                self.builder()
                    .record_invocation(Some(apply.name.clone()), &callee, args, tail);
                self.store_binding_value(&apply.name, value, apply.span)?;
                return Ok(());
            }
            BlockItem::Invocation(invocation) if invocation.result.is_some() => {
                let name = invocation.result.as_ref().unwrap().clone();
                let was_terminated = self.terminated;
                let value = self.emit_invocation_value(invocation)?;
                let tail = self.terminated && !was_terminated;
                let args = invocation
                    .args
                    .iter()
                    .map(|arg| arg.name.clone())
                    .collect::<Vec<_>>();
                let callee = invocation.of.clone();
                self.builder()
                    .record_invocation(Some(name.clone()), &callee, args, tail);
                self.store_binding_value(&name, value, invocation.span)?;
                return Ok(());
            }
            BlockItem::Invocation(invocation) => {
                let was_terminated = self.terminated;
                let result = self.emit_invocation(invocation)?;
                let tail = self.terminated && !was_terminated;
                let args = invocation
                    .args
                    .iter()
                    .map(|arg| arg.name.clone())
                    .collect::<Vec<_>>();
                let callee = invocation.of.clone();
                self.builder().record_invocation(None, &callee, args, tail);
                if let ExprValue::Closure(_) = result {
                    // discard unused closures to avoid leaking temporaries
                }
                return Ok(());
            }
            BlockItem::ReleaseEnv(release) => {
                self.emit_release_env(release)?;
                return Ok(());
            }
        }
    }

    fn emit_release_env(&mut self, release: &ReleaseEnv) -> Result<(), CodegenError> {
        let binding = self.frame.binding(&release.name).ok_or_else(|| {
            CodegenError::new(format!("unknown binding '{}'", release.name), release.span)
        })?;
        if binding.kind != ValueKind::Closure {
            return Err(CodegenError::new(
                format!("cannot release non-closure binding '{}'", release.name),
                release.span,
            ));
        }
        let env_offset = binding.slot_addr(1);
        writeln!(
            self.out,
            "    mov rdx, [rbp-{}] ; load closure env_end pointer",
            env_offset
        )?;
        writeln!(self.out, "    mov rcx, [rdx] ; read env size metadata")?;
        writeln!(
            self.out,
            "    mov rsi, [rdx+{}] ; read heap length metadata",
            ENV_METADATA_FIELD_SIZE
        )?;
        writeln!(self.out, "    mov rbx, rdx ; env_end pointer")?;
        writeln!(self.out, "    sub rbx, rcx ; compute env base pointer")?;
        writeln!(self.out, "    mov rdi, rbx ; env base for munmap")?;
        writeln!(self.out, "    mov rax, {} ; munmap syscall", SYSCALL_MUNMAP)?;
        writeln!(self.out, "    syscall ; release closure environment")?;
        Ok(())
    }

    fn emit_invocation_value(
        &mut self,
        invocation: &Invocation,
    ) -> Result<ExprValue, CodegenError> {
        if let Some(sig) = self.symbols.get_function(&invocation.of) {
            self.emit_named_function_closure(&invocation.of, sig)?;
            let state = ClosureState::new(sig.params.clone());
            return self.apply_closure(state, &invocation.args, invocation.span, false);
        }
        self.emit_invocation(invocation)
    }

    fn store_binding_value(
        &mut self,
        name: &str,
        value: ExprValue,
        span: Span,
    ) -> Result<(), CodegenError> {
        let binding = self
            .frame
            .binding_mut(name)
            .ok_or_else(|| CodegenError::new(format!("unknown binding '{}'", name), span))?;
        match value {
            ExprValue::Word => {
                writeln!(
                    self.out,
                    "    mov [rbp-{}], rax ; save evaluated scalar in frame",
                    binding.slot_addr(0)
                )?;
                binding.kind = ValueKind::Word;
                binding.continuation_params.clear();
            }
            ExprValue::Closure(state) => {
                writeln!(
                    self.out,
                    "    mov [rbp-{}], rax ; update closure code pointer",
                    binding.slot_addr(0)
                )?;
                writeln!(
                    self.out,
                    "    mov [rbp-{}], rdx ; update closure environment pointer",
                    binding.slot_addr(1)
                )?;
                binding.kind = ValueKind::Closure;
                binding.continuation_params = state.remaining.clone();
            }
        }
        Ok(())
    }

    fn emit_expr(&mut self, expr: &Expr) -> Result<ExprValue, CodegenError> {
        match expr {
            Expr::Int { value, .. } => {
                writeln!(self.out, "    mov rax, {} ; load literal integer", value)?;
                Ok(ExprValue::Word)
            }
            Expr::String { value, .. } => {
                let label = self.ctx.string_literal_label(value);
                writeln!(
                    self.out,
                    "    lea rax, [rel {}] ; point to string literal",
                    label
                )?;
                Ok(ExprValue::Word)
            }
            Expr::Ident { name, span } => {
                if let Some(binding) = self.frame.binding(name) {
                    match binding.kind {
                        ValueKind::Word => {
                            writeln!(
                                self.out,
                                "    mov rax, [rbp-{}] ; load scalar from frame",
                                binding.slot_addr(0)
                            )?;
                            Ok(ExprValue::Word)
                        }
                        ValueKind::Closure => {
                            writeln!(
                                self.out,
                                "    mov rax, [rbp-{}] ; load closure code pointer",
                                binding.slot_addr(0)
                            )?;
                            writeln!(
                                self.out,
                                "    mov rdx, [rbp-{}] ; load closure env_end pointer",
                                binding.slot_addr(1)
                            )?;
                            Ok(ExprValue::Closure(ClosureState::new(
                                binding.continuation_params.clone(),
                            )))
                        }
                    }
                } else if let Some(sig) = self.symbols.get_function(name) {
                    self.emit_named_function_closure(name, sig)?;
                    Ok(ExprValue::Closure(ClosureState::new(sig.params.clone())))
                } else if self.symbols.get_value(name).is_some() {
                    if !self.ctx.is_global_value(name) {
                        self.ctx.add_extern(name);
                    }
                    writeln!(
                        self.out,
                        "    mov rax, [rel {}] ; load external value pointer",
                        name
                    )?;
                    Ok(ExprValue::Word)
                } else {
                    Err(CodegenError::new(
                        format!("compiler bug: unresolved identifier '{}'", name),
                        *span,
                    ))
                }
            }
            Expr::Apply { span, .. } => Err(CodegenError::new(
                "apply expressions should already be normalized into statements",
                *span,
            )),
            Expr::Lambda { span, .. } => Err(CodegenError::new(
                "lambda expressions are not supported yet",
                *span,
            )),
        }
    }

    fn emit_arg_value(&mut self, arg: &Arg) -> Result<ExprValue, CodegenError> {
        let expr = Expr::Ident {
            name: arg.name.clone(),
            span: arg.span,
        };
        self.emit_expr(&expr)
    }

    fn emit_invocation(&mut self, invocation: &Invocation) -> Result<ExprValue, CodegenError> {
        let name = invocation.of.as_str();
        let span = invocation.span;

        if name == "exit" {
            return self.emit_exit_call(&invocation.args, span);
        }

        if name == "printf" {
            return self.emit_printf_call(&invocation.args, span);
        }
        if name == "sprintf" {
            return self.emit_sprintf_call(&invocation.args, span);
        }
        if name == "write" {
            return self.emit_write_call(&invocation.args, span);
        }

        if let Some(binding) = self.frame.binding(name) {
            if binding.kind != ValueKind::Closure {
                return Err(CodegenError::new(
                    format!("'{}' is not callable", name),
                    span,
                ));
            }
            writeln!(
                self.out,
                "    mov rax, [rbp-{}] ; load closure code for call",
                binding.slot_addr(0)
            )?;
            writeln!(
                self.out,
                "    mov rdx, [rbp-{}] ; load closure env_end for call",
                binding.slot_addr(1)
            )?;
            let state = ClosureState::new(binding.continuation_params.clone());
            return self.apply_closure(state, &invocation.args, span, true);
        }

        if let Some(sig) = self.symbols.get_function(name) {
            return self.emit_named_call(name, sig, &invocation.args, span);
        }

        self.ctx.add_extern(name);
        let placeholder: Vec<TypeRef> = std::iter::repeat(TypeRef::Int)
            .take(invocation.args.len())
            .collect();
        self.prepare_call_args(&invocation.args, &placeholder)?;
        self.move_args_to_registers(&placeholder)?;
        writeln!(self.out, "    leave ; unwind before jumping")?;
        writeln!(self.out, "    jmp {} ; jump to named function", name)?;
        self.terminated = true;

        Ok(ExprValue::Word)
    }

    fn emit_printf_call(&mut self, args: &[Arg], span: Span) -> Result<ExprValue, CodegenError> {
        if args.len() < 2 {
            return Err(CodegenError::new(
                "printf requires a format string and a continuation",
                span,
            ));
        }

        let continuation_arg = &args[args.len() - 1];
        let continuation_value = self.emit_arg_value(continuation_arg)?;
        let closure_state = match continuation_value {
            ExprValue::Closure(state) => state,
            _ => {
                return Err(CodegenError::new(
                    "last argument to printf must be a continuation",
                    continuation_arg.span,
                ))
            }
        };

        if !closure_state.remaining().is_empty() {
            return Err(CodegenError::new(
                "printf continuation must accept no arguments",
                continuation_arg.span,
            ));
        }

        writeln!(
            self.out,
            "    push rax ; preserve continuation code pointer"
        )?;
        writeln!(
            self.out,
            "    push rdx ; preserve continuation env_end pointer"
        )?;

        let call_args = &args[..args.len() - 1];
        if call_args.is_empty() {
            return Err(CodegenError::new(
                "printf requires a format string before the continuation",
                span,
            ));
        }

        let mut params = Vec::with_capacity(call_args.len());
        params.push(TypeRef::Str);
        for _ in 1..call_args.len() {
            params.push(TypeRef::Int);
        }

        self.prepare_call_args(call_args, &params)?;
        self.move_args_to_registers(&params)?;

        self.ctx.add_extern("printf");
        writeln!(self.out, "    sub rsp, 8 ; align stack for variadic call")?;
        writeln!(self.out, "    call printf ; invoke libc printf")?;
        writeln!(self.out, "    add rsp, 8")?;
        self.ctx.add_extern("fflush");
        self.ctx.add_extern("stdout");
        writeln!(self.out, "    mov rdi, [rel stdout] ; flush stdout")?;
        writeln!(self.out, "    sub rsp, 8 ; align stack for fflush")?;
        writeln!(self.out, "    call fflush")?;
        writeln!(self.out, "    add rsp, 8")?;

        writeln!(
            self.out,
            "    pop rdx ; restore continuation env_end pointer"
        )?;
        writeln!(self.out, "    pop rax ; restore continuation code pointer")?;
        writeln!(
            self.out,
            "    mov rdi, rdx ; pass env_end pointer to continuation"
        )?;
        writeln!(self.out, "    leave ; unwind before jumping")?;
        writeln!(self.out, "    jmp rax ; jump into continuation")?;
        self.terminated = true;

        Ok(ExprValue::Word)
    }

    fn emit_sprintf_call(&mut self, args: &[Arg], span: Span) -> Result<ExprValue, CodegenError> {
        if args.len() < 2 {
            return Err(CodegenError::new(
                "sprintf requires a format string, an integer, and a continuation",
                span,
            ));
        }

        let continuation_arg = &args[args.len() - 1];
        let continuation_value = self.emit_arg_value(continuation_arg)?;
        let closure_state = match continuation_value {
            ExprValue::Closure(state) => state,
            _ => {
                return Err(CodegenError::new(
                    "last argument to sprintf must be a continuation",
                    continuation_arg.span,
                ))
            }
        };

        let remaining_params = closure_state.remaining();
        if remaining_params.is_empty() {
            return Err(CodegenError::new(
                "sprintf continuation must accept at least one string argument",
                continuation_arg.span,
            ));
        }
        if remaining_params[0] != TypeRef::Str {
            return Err(CodegenError::new(
                "sprintf continuation must accept a string argument",
                continuation_arg.span,
            ));
        }

        writeln!(
            self.out,
            "    push rax ; preserve continuation code pointer"
        )?;
        writeln!(
            self.out,
            "    push rdx ; preserve continuation env_end pointer"
        )?;
        writeln!(
            self.out,
            "    mov r13, [rsp] ; stash continuation env_end pointer"
        )?;

        let call_args = &args[..args.len() - 1];
        if call_args.len() != 2 {
            return Err(CodegenError::new(
                "sprintf requires a format string and a single integer before the continuation",
                span,
            ));
        }

        let params = vec![TypeRef::Str, TypeRef::Int];
        self.prepare_call_args(call_args, &params)?;

        self.emit_mmap(FMT_BUFFER_SIZE)?;
        writeln!(self.out, "    mov r12, rax ; keep sprintf buffer pointer")?;
        writeln!(
            self.out,
            "    pop rsi ; restore format string pointer for sprintf"
        )?;
        writeln!(
            self.out,
            "    pop rdx ; restore integer argument for sprintf"
        )?;

        writeln!(
            self.out,
            "    mov rdi, r12 ; destination buffer for sprintf"
        )?;
        self.ctx.add_extern("sprintf");
        writeln!(
            self.out,
            "    sub rsp, 8 ; align stack for variadic sprintf call"
        )?;
        writeln!(self.out, "    call sprintf")?;
        writeln!(self.out, "    add rsp, 8")?;

        let suffix_sizes = remaining_suffix_sizes(remaining_params, self.symbols);
        let result_offset = suffix_sizes
            .get(0)
            .copied()
            .unwrap_or_else(|| bytes_for_type(&TypeRef::Str, self.symbols));
        writeln!(self.out, "    mov r10, r13 ; copy env_end pointer")?;
        writeln!(
            self.out,
            "    sub r10, {} ; slot for formatted result",
            result_offset
        )?;
        writeln!(
            self.out,
            "    mov [r10], r12 ; store formatted string pointer"
        )?;

        writeln!(
            self.out,
            "    pop rdx ; restore continuation env_end pointer"
        )?;
        writeln!(self.out, "    pop rax ; restore continuation code pointer")?;
        writeln!(
            self.out,
            "    mov rdi, rdx ; pass env_end pointer to continuation"
        )?;
        writeln!(self.out, "    leave ; unwind before jumping")?;
        writeln!(self.out, "    jmp rax ; jump into continuation")?;
        self.terminated = true;

        Ok(ExprValue::Word)
    }

    fn emit_write_call(&mut self, args: &[Arg], span: Span) -> Result<ExprValue, CodegenError> {
        if args.len() < 2 {
            return Err(CodegenError::new(
                "write requires a string and a continuation",
                span,
            ));
        }

        let continuation_arg = &args[args.len() - 1];
        let continuation_value = self.emit_arg_value(continuation_arg)?;
        let closure_state = match continuation_value {
            ExprValue::Closure(state) => state,
            _ => {
                return Err(CodegenError::new(
                    "last argument to write must be a continuation",
                    continuation_arg.span,
                ))
            }
        };

        if !closure_state.remaining().is_empty() {
            return Err(CodegenError::new(
                "write continuation must accept no arguments",
                continuation_arg.span,
            ));
        }

        writeln!(
            self.out,
            "    push rax ; preserve continuation code pointer"
        )?;
        writeln!(
            self.out,
            "    push rdx ; preserve continuation env_end pointer"
        )?;

        let call_args = &args[..args.len() - 1];
        if call_args.len() != 1 {
            return Err(CodegenError::new(
                "write requires a string before the continuation",
                span,
            ));
        }

        let params = vec![TypeRef::Str];
        self.prepare_call_args(call_args, &params)?;
        self.move_args_to_registers(&params)?;

        let (loop_label, done_label) = self.next_write_loop_labels();
        writeln!(self.out, "    mov r8, rdi ; keep string pointer")?;
        writeln!(self.out, "    xor rcx, rcx ; reset length counter")?;
        writeln!(self.out, "{}:", loop_label)?;
        writeln!(
            self.out,
            "    mov dl, byte [r8+rcx] ; load current character"
        )?;
        writeln!(self.out, "    cmp dl, 0 ; stop at terminator")?;
        writeln!(self.out, "    je {}", done_label)?;
        writeln!(self.out, "    inc rcx ; advance char counter")?;
        writeln!(self.out, "    jmp {}", loop_label)?;
        writeln!(self.out, "{}:", done_label)?;
        writeln!(self.out, "    mov rsi, r8 ; buffer start")?;
        writeln!(self.out, "    mov rdx, rcx ; length to write")?;
        writeln!(self.out, "    mov rdi, 1 ; stdout fd")?;
        self.ctx.add_extern("write");
        writeln!(self.out, "    call write ; invoke libc write")?;

        writeln!(
            self.out,
            "    pop rdx ; restore continuation env_end pointer"
        )?;
        writeln!(self.out, "    pop rax ; restore continuation code pointer")?;
        writeln!(
            self.out,
            "    mov rdi, rdx ; pass env_end pointer to continuation"
        )?;
        writeln!(self.out, "    leave ; unwind before jumping")?;
        writeln!(self.out, "    jmp rax ; jump into continuation")?;
        self.terminated = true;

        Ok(ExprValue::Word)
    }

    fn emit_exit_call(&mut self, args: &[Arg], span: Span) -> Result<ExprValue, CodegenError> {
        if args.len() != 1 {
            return Err(CodegenError::new("exit requires an integer argument", span));
        }

        let arg = &args[0];
        let value = self.emit_arg_value(arg)?;
        self.ensure_value_matches(&value, &TypeRef::Int, arg.span)?;

        match value {
            ExprValue::Word => {
                writeln!(self.out, "    mov rdi, rax ; pass exit code")?;
            }
            ExprValue::Closure(_) => unreachable!(),
        }

        writeln!(self.out, "    leave ; unwind before exit")?;
        writeln!(self.out, "    mov rax, {} ; exit syscall", SYSCALL_EXIT)?;
        writeln!(self.out, "    syscall ; exit program")?;
        self.terminated = true;

        Ok(ExprValue::Word)
    }

    fn emit_named_call(
        &mut self,
        of: &str,
        sig: &FunctionSig,
        args: &[Arg],
        span: Span,
    ) -> Result<ExprValue, CodegenError> {
        if args.len() > sig.params.len() {
            return Err(CodegenError::new(
                format!(
                    "function '{}' expected {} arguments but got {}",
                    of,
                    sig.params.len(),
                    args.len()
                ),
                span,
            ));
        }

        if args.len() < sig.params.len() {
            self.emit_named_function_closure(of, sig)?;
            let state = ClosureState::new(sig.params.clone());
            return self.apply_closure(state, args, span, true);
        }

        self.prepare_call_args(args, &sig.params)?;
        self.move_args_to_registers(&sig.params)?;
        writeln!(self.out, "    leave ; unwind before named jump")?;
        writeln!(self.out, "    jmp {} ; jump to fully applied function", of)?;
        self.terminated = true;
        Ok(ExprValue::Word)
    }

    fn emit_named_function_closure(
        &mut self,
        name: &str,
        sig: &FunctionSig,
    ) -> Result<(), CodegenError> {
        let env_size = env_size_bytes(&sig.params, self.symbols);
        let heap_size = env_size + ENV_METADATA_SIZE;
        self.emit_mmap(heap_size)?;
        writeln!(self.out, "    mov rdx, rax ; store env base pointer")?;
        if env_size > 0 {
            writeln!(
                self.out,
                "    add rdx, {} ; bump pointer past env header",
                env_size
            )?;
        }
        writeln!(
            self.out,
            "    mov qword [rdx], {} ; env size metadata",
            env_size
        )?;
        writeln!(
            self.out,
            "    mov qword [rdx+{}], {} ; heap size metadata",
            ENV_METADATA_FIELD_SIZE, heap_size
        )?;
        let wrapper = closure_wrapper_label(name);
        writeln!(
            self.out,
            "    mov rax, {} ; load wrapper entry point",
            wrapper
        )?;
        Ok(())
    }

    fn emit_mmap(&mut self, size: usize) -> Result<(), CodegenError> {
        writeln!(self.out, "    mov rax, {} ; mmap syscall", SYSCALL_MMAP)?;
        writeln!(self.out, "    xor rdi, rdi ; addr = NULL hint")?;
        writeln!(
            self.out,
            "    mov rsi, {} ; length for allocation",
            size.max(1)
        )?;
        writeln!(
            self.out,
            "    mov rdx, {} ; prot = read/write",
            PROT_READ | PROT_WRITE
        )?;
        writeln!(
            self.out,
            "    mov r10, {} ; flags: private & anonymous",
            MAP_PRIVATE | MAP_ANONYMOUS
        )?;
        writeln!(self.out, "    mov r8, -1 ; fd = -1")?;
        writeln!(self.out, "    xor r9, r9 ; offset = 0")?;
        writeln!(self.out, "    syscall ; allocate env pages")?;
        self.builder().record_heap_alloc(
            size.max(1),
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        );
        Ok(())
    }

    fn prepare_call_args(&mut self, args: &[Arg], params: &[TypeRef]) -> Result<(), CodegenError> {
        let total_slots: usize = params
            .iter()
            .map(|ty| slots_for_type(ty, self.symbols))
            .sum();
        if total_slots > ARG_REGS.len() {
            return Err(CodegenError::new(
                format!(
                    "functions support at most {} argument slots",
                    ARG_REGS.len()
                ),
                self.func.span,
            ));
        }
        for (arg, ty) in args.iter().zip(params).rev() {
            let value = self.emit_arg_value(arg)?;
            self.ensure_value_matches(&value, ty, arg.span)?;
            self.push_value(&value)?;
        }
        Ok(())
    }

    fn push_value(&mut self, value: &ExprValue) -> Result<(), CodegenError> {
        match value {
            ExprValue::Word => {
                writeln!(self.out, "    push rax ; stack arg: scalar")?;
            }
            ExprValue::Closure(_) => {
                writeln!(self.out, "    push rdx ; stack arg: closure env_end")?;
                writeln!(self.out, "    push rax ; stack arg: closure code")?;
            }
        }
        Ok(())
    }

    fn move_args_to_registers(&mut self, params: &[TypeRef]) -> Result<(), CodegenError> {
        let mut slot = 0usize;
        for ty in params {
            match resolved_type_kind(ty, self.symbols) {
                ValueKind::Word => {
                    let reg = ARG_REGS[slot];
                    writeln!(
                        self.out,
                        "    pop {} ; restore scalar arg into register",
                        reg
                    )?;
                    slot += 1;
                }
                ValueKind::Closure => {
                    if slot + 1 >= ARG_REGS.len() {
                        return Err(CodegenError::new(
                            "continuations require two argument slots",
                            self.func.span,
                        ));
                    }
                    let code_reg = ARG_REGS[slot];
                    let env_reg = ARG_REGS[slot + 1];
                    writeln!(
                        self.out,
                        "    pop {} ; restore closure code into register",
                        code_reg
                    )?;
                    writeln!(
                        self.out,
                        "    pop {} ; restore closure env_end into register",
                        env_reg
                    )?;
                    slot += 2;
                }
            }
        }
        Ok(())
    }

    fn apply_closure(
        &mut self,
        state: ClosureState,
        args: &[Arg],
        span: Span,
        invoke_when_ready: bool,
    ) -> Result<ExprValue, CodegenError> {
        if args.len() > state.remaining().len() {
            return Err(CodegenError::new("too many arguments for closure", span));
        }
        writeln!(
            self.out,
            "    sub rsp, 16 ; allocate temporary stack for closure state"
        )?;
        writeln!(
            self.out,
            "    mov [rsp], rax ; save closure code pointer temporarily"
        )?;
        writeln!(
            self.out,
            "    mov [rsp+8], rdx ; save closure env_end pointer temporarily"
        )?;

        let remaining = state.remaining();
        let suffix_sizes = remaining_suffix_sizes(remaining, self.symbols);
        for (idx, (arg, ty)) in args.iter().zip(remaining.iter()).enumerate() {
            let value = self.emit_arg_value(arg)?;
            self.ensure_value_matches(&value, ty, arg.span)?;
            writeln!(self.out, "    mov rbx, [rsp+8] ; env_end pointer")?;
            writeln!(
                self.out,
                "    sub rbx, {} ; compute slot for next argument",
                suffix_sizes[idx]
            )?;
            match resolved_type_kind(ty, self.symbols) {
                ValueKind::Word => {
                    writeln!(self.out, "    mov [rbx], rax ; store scalar arg in env")?;
                }
                ValueKind::Closure => {
                    writeln!(self.out, "    mov [rbx], rax ; store closure code for arg")?;
                    writeln!(
                        self.out,
                        "    mov [rbx+8], rdx ; store closure env_end for arg"
                    )?;
                }
            }
        }

        let remaining = state.after_applying(args.len());
        writeln!(
            self.out,
            "    mov rax, [rsp] ; restore closure code pointer"
        )?;
        writeln!(
            self.out,
            "    mov rdx, [rsp+8] ; restore closure env_end pointer"
        )?;
        writeln!(self.out, "    add rsp, 16 ; pop temporary closure state")?;

        if remaining.remaining().is_empty() {
            if invoke_when_ready {
                writeln!(
                    self.out,
                    "    mov rdi, rdx ; pass env_end pointer as parameter"
                )?;
                writeln!(self.out, "    leave ; unwind before calling closure")?;
                writeln!(self.out, "    jmp rax ; jump into fully applied closure")?;
                self.terminated = true;
                Ok(ExprValue::Word)
            } else {
                Ok(ExprValue::Closure(remaining))
            }
        } else {
            Ok(ExprValue::Closure(remaining))
        }
    }

    fn ensure_value_matches(
        &self,
        value: &ExprValue,
        ty: &TypeRef,
        span: Span,
    ) -> Result<(), CodegenError> {
        match (value, resolved_type_kind(ty, self.symbols)) {
            (ExprValue::Word, ValueKind::Word) => Ok(()),
            (ExprValue::Closure(_), ValueKind::Closure) => Ok(()),
            (ExprValue::Word, ValueKind::Closure) => {
                Err(CodegenError::new("expected a closure value", span))
            }
            (ExprValue::Closure(_), ValueKind::Word) => {
                Err(CodegenError::new("expected a scalar value", span))
            }
        }
    }

    fn next_write_loop_labels(&mut self) -> (String, String) {
        let idx = self.write_loop_counter;
        self.write_loop_counter += 1;
        let prefix = self.sanitized_func_name();
        (
            format!("{}_write_strlen_loop_{}", prefix, idx),
            format!("{}_write_strlen_done_{}", prefix, idx),
        )
    }

    fn sanitized_func_name(&self) -> String {
        self.func
            .name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }
}

struct AsmFunctionBuilder {
    name: String,
    params: Vec<String>,
    statements: Vec<AsmStatement>,
}

impl AsmFunctionBuilder {
    fn new(func: &Function) -> Self {
        let params = func
            .params
            .iter()
            .map(|param| format!("{}: {}", param.name, format_type(&param.ty)))
            .collect();
        Self {
            name: func.name.clone(),
            params,
            statements: Vec::new(),
        }
    }

    fn record_literal(&mut self, target: &str, literal: AsmLiteral) {
        self.statements.push(AsmStatement::Literal {
            target: target.to_string(),
            value: literal,
        });
    }

    fn record_invocation(
        &mut self,
        target: Option<String>,
        callee: &str,
        args: Vec<String>,
        tail: bool,
    ) {
        self.statements.push(AsmStatement::Invocation {
            target,
            callee: callee.to_string(),
            args,
            tail,
        });
    }

    fn record_heap_alloc(&mut self, size: usize, prot: i32, flags: i32, fd: i32, offset: i32) {
        self.statements.push(AsmStatement::HeapAlloc {
            size,
            prot,
            flags,
            fd,
            offset,
        });
    }

    fn build(self) -> AsmFunction {
        AsmFunction::new(self.name, self.params, self.statements)
    }
}

fn format_type(ty: &TypeRef) -> String {
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
    }
}

fn emit_closure_wrapper<W: Write>(
    func: &Function,
    symbols: &SymbolRegistry,
    out: &mut W,
) -> Result<(), CodegenError> {
    let total_slots: usize = func
        .params
        .iter()
        .map(|p| slots_for_type(&p.ty, symbols))
        .sum();
    if total_slots > ARG_REGS.len() {
        return Err(CodegenError::new(
            format!(
                "function '{}' exceeds supported continuation argument slots",
                func.name
            ),
            func.span,
        ));
    }
    let label = closure_wrapper_label(&func.name);
    writeln!(out, "{}:", label)?;
    writeln!(out, "    push rbp ; save caller frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; establish wrapper frame")?;
    writeln!(out, "    push rbx ; preserve base register")?;
    writeln!(out, "    mov rbx, rdi ; rdi points to env_end when invoked")?;
    let param_types: Vec<TypeRef> = func.params.iter().map(|p| p.ty.clone()).collect();
    let env_size = env_size_bytes(&param_types, symbols);
    if env_size > 0 {
        writeln!(out, "    sub rbx, {} ; compute env base", env_size)?;
    }
    let mut reg_slot = 0usize;
    let mut saved_regs: Vec<&str> = Vec::new();
    for (idx, param) in func.params.iter().enumerate() {
        match resolved_type_kind(&param.ty, symbols) {
            ValueKind::Word => {
                let offset = env_offset(&func.params, idx, symbols);
                let reg = ARG_REGS[reg_slot];
                writeln!(
                    out,
                    "    mov {}, [rbx+{}] ; load scalar param from env",
                    reg, offset
                )?;
                writeln!(out, "    push {} ; preserve parameter register", reg)?;
                saved_regs.push(reg);
                reg_slot += 1;
            }
            ValueKind::Closure => {
                let offset = env_offset(&func.params, idx, symbols);
                let reg_code = ARG_REGS[reg_slot];
                let reg_env = ARG_REGS[reg_slot + 1];
                writeln!(
                    out,
                    "    mov {}, [rbx+{}] ; load continuation code pointer",
                    reg_code, offset
                )?;
                writeln!(
                    out,
                    "    push {} ; preserve closure code register",
                    reg_code
                )?;
                saved_regs.push(reg_code);
                writeln!(
                    out,
                    "    mov {}, [rbx+{}] ; load continuation env_end pointer",
                    reg_env,
                    offset + 8
                )?;
                writeln!(
                    out,
                    "    push {} ; preserve closure env_end register",
                    reg_env
                )?;
                saved_regs.push(reg_env);
                reg_slot += 2;
            }
        }
    }
    for reg in saved_regs.iter().rev() {
        writeln!(out, "    pop {} ; restore parameter register", reg)?;
    }
    writeln!(out, "    pop rbx ; restore saved base register")?;
    writeln!(out, "    leave ; epilogue: restore rbp of caller")?;
    writeln!(out, "    jmp {} ; jump into actual function", func.name)?;
    Ok(())
}

fn closure_wrapper_label(name: &str) -> String {
    format!("{}_closure_entry", name)
}

fn env_offset(params: &[Param], idx: usize, symbols: &SymbolRegistry) -> usize {
    params
        .iter()
        .take(idx)
        .map(|p| bytes_for_type(&p.ty, symbols))
        .sum()
}

fn remaining_suffix_sizes(params: &[TypeRef], symbols: &SymbolRegistry) -> Vec<usize> {
    let mut sizes = Vec::with_capacity(params.len());
    let mut acc = 0;
    for param in params.iter().rev() {
        acc += bytes_for_type(param, symbols);
        sizes.push(acc);
    }
    sizes.reverse();
    sizes
}

fn align_to(value: usize, align: usize) -> usize {
    if value == 0 {
        return 0;
    }
    ((value + align - 1) / align) * align
}

fn slots_for_type(ty: &TypeRef, symbols: &SymbolRegistry) -> usize {
    match resolved_type_kind(ty, symbols) {
        ValueKind::Word => 1,
        ValueKind::Closure => 2,
    }
}

fn bytes_for_type(ty: &TypeRef, symbols: &SymbolRegistry) -> usize {
    match resolved_type_kind(ty, symbols) {
        ValueKind::Word => 8,
        ValueKind::Closure => 16,
    }
}

fn env_size_bytes(params: &[TypeRef], symbols: &SymbolRegistry) -> usize {
    params.iter().map(|ty| bytes_for_type(ty, symbols)).sum()
}

fn resolved_type_kind(ty: &TypeRef, symbols: &SymbolRegistry) -> ValueKind {
    let mut visited = HashSet::new();
    resolved_type_kind_inner(ty, symbols, &mut visited)
}

fn resolved_type_kind_inner(
    ty: &TypeRef,
    symbols: &SymbolRegistry,
    visited: &mut HashSet<String>,
) -> ValueKind {
    match ty {
        TypeRef::Int | TypeRef::Str | TypeRef::CompileTimeInt | TypeRef::CompileTimeStr => {
            ValueKind::Word
        }
        TypeRef::Type(_) => ValueKind::Closure,
        TypeRef::Alias(name) => {
            if visited.contains(name) {
                ValueKind::Closure
            } else if let Some(info) = symbols.get_type_info(name) {
                visited.insert(name.clone());
                let kind = resolved_type_kind_inner(&info.target, symbols, visited);
                visited.remove(name);
                kind
            } else {
                ValueKind::Word
            }
        }
    }
}

fn continuation_params_for_type(ty: &TypeRef, symbols: &SymbolRegistry) -> Vec<TypeRef> {
    let mut visited = HashSet::new();
    continuation_params_for_type_inner(ty, symbols, &mut visited)
}

fn continuation_params_for_type_inner(
    ty: &TypeRef,
    symbols: &SymbolRegistry,
    visited: &mut HashSet<String>,
) -> Vec<TypeRef> {
    match ty {
        TypeRef::Type(params) => params.clone(),
        TypeRef::Alias(name) => {
            if visited.contains(name) {
                Vec::new()
            } else if let Some(info) = symbols.get_type_info(name) {
                visited.insert(name.clone());
                let result = continuation_params_for_type_inner(&info.target, symbols, visited);
                visited.remove(name);
                result
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    }
}

fn emit_builtin_write<W: Write>(ctx: &mut CodegenContext, out: &mut W) -> Result<(), CodegenError> {
    writeln!(out, "global rgo_write")?;
    writeln!(out, "rgo_write:")?;
    writeln!(out, "    push rbp ; prologue: save caller frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
    writeln!(out, "    push rsi ; preserve continuation code pointer")?;
    writeln!(out, "    push rdx ; preserve continuation env_end pointer")?;
    writeln!(out, "    mov r8, rdi ; keep string pointer")?;
    writeln!(out, "    xor rcx, rcx ; reset length counter")?;
    writeln!(out, "write_strlen_loop:")?;
    writeln!(out, "    mov dl, byte [r8+rcx] ; load current character")?;
    writeln!(out, "    cmp dl, 0 ; stop at terminator")?;
    writeln!(out, "    je write_strlen_done")?;
    writeln!(out, "    inc rcx ; advance char counter")?;
    writeln!(out, "    jmp write_strlen_loop")?;
    writeln!(out, "write_strlen_done:")?;
    writeln!(out, "    mov rdx, rcx ; length to write")?;
    writeln!(out, "    mov rsi, r8 ; buffer start")?;
    writeln!(out, "    mov rdi, 1 ; stdout fd")?;
    ctx.add_extern("write");
    writeln!(out, "    call write ; invoke libc write")?;
    writeln!(out, "    pop rdx ; restore continuation env_end pointer")?;
    writeln!(out, "    pop rsi ; restore continuation code pointer")?;
    writeln!(out, "    mov rax, rsi ; continuation entry point")?;
    writeln!(
        out,
        "    mov rdi, rdx ; pass env_end pointer to continuation"
    )?;
    writeln!(out, "    leave ; epilogue: unwind before jump")?;
    writeln!(out, "    jmp rax ; jump into continuation")?;
    writeln!(out)?;
    Ok(())
}

fn emit_builtin_add<W: Write>(out: &mut W) -> Result<(), CodegenError> {
    writeln!(out, "global add")?;
    writeln!(out, "add:")?;
    writeln!(out, "    push rbp ; prologue: save caller frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
    writeln!(out, "    push rdx ; preserve continuation code pointer")?;
    writeln!(out, "    push rcx ; preserve continuation env_end pointer")?;
    writeln!(out, "    mov rax, rdi ; load first integer")?;
    writeln!(out, "    add rax, rsi ; add second integer")?;
    writeln!(
        out,
        "    mov r8, [rbp-16] ; keep env_end pointer intact for continuation"
    )?;
    writeln!(
        out,
        "    lea rcx, [r8-8] ; reserve slot for result before metadata"
    )?;
    writeln!(out, "    mov [rcx], rax ; store sum")?;
    writeln!(out, "    mov rax, [rbp-8] ; continuation entry point")?;
    writeln!(
        out,
        "    mov rdi, r8 ; pass env_end pointer (metadata start) unchanged"
    )?;
    writeln!(out, "    leave ; unwind before jump")?;
    writeln!(out, "    jmp rax ; jump into continuation")?;
    writeln!(out)?;
    Ok(())
}

fn emit_builtin_sub<W: Write>(out: &mut W) -> Result<(), CodegenError> {
    writeln!(out, "global sub")?;
    writeln!(out, "sub:")?;
    writeln!(out, "    push rbp ; prologue: save caller frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
    writeln!(out, "    push rdx ; preserve continuation code pointer")?;
    writeln!(out, "    push rcx ; preserve continuation env_end pointer")?;
    writeln!(out, "    mov rax, rdi ; load minuend")?;
    writeln!(out, "    sub rax, rsi ; subtract subtrahend")?;
    writeln!(
        out,
        "    mov r8, [rbp-16] ; keep env_end pointer intact for continuation"
    )?;
    writeln!(
        out,
        "    lea rcx, [r8-8] ; reserve slot for result before metadata"
    )?;
    writeln!(out, "    mov [rcx], rax ; store difference")?;
    writeln!(out, "    mov rax, [rbp-8] ; continuation entry point")?;
    writeln!(
        out,
        "    mov rdi, r8 ; pass env_end pointer (metadata start) unchanged"
    )?;
    writeln!(out, "    leave ; unwind before jump")?;
    writeln!(out, "    jmp rax ; jump into continuation")?;
    writeln!(out)?;
    Ok(())
}

fn emit_builtin_mul<W: Write>(out: &mut W) -> Result<(), CodegenError> {
    writeln!(out, "global mul")?;
    writeln!(out, "mul:")?;
    writeln!(out, "    push rbp ; prologue: save caller frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
    writeln!(out, "    push rdx ; preserve continuation code pointer")?;
    writeln!(out, "    push rcx ; preserve continuation env_end pointer")?;
    writeln!(out, "    mov rax, rdi ; load multiplicand")?;
    writeln!(out, "    imul rax, rsi ; multiply by multiplier")?;
    writeln!(
        out,
        "    mov r8, [rbp-16] ; keep env_end pointer intact for continuation"
    )?;
    writeln!(
        out,
        "    lea rcx, [r8-8] ; reserve slot for result before metadata"
    )?;
    writeln!(out, "    mov [rcx], rax ; store product")?;
    writeln!(out, "    mov rax, [rbp-8] ; continuation entry point")?;
    writeln!(
        out,
        "    mov rdi, r8 ; pass env_end pointer (metadata start) unchanged"
    )?;
    writeln!(out, "    leave ; unwind before jump")?;
    writeln!(out, "    jmp rax ; jump into continuation")?;
    writeln!(out)?;
    Ok(())
}

fn emit_builtin_div<W: Write>(out: &mut W) -> Result<(), CodegenError> {
    writeln!(out, "global div")?;
    writeln!(out, "div:")?;
    writeln!(out, "    push rbp ; prologue: save caller frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
    writeln!(out, "    push rdx ; preserve continuation code pointer")?;
    writeln!(out, "    push rcx ; preserve continuation env_end pointer")?;
    writeln!(out, "    mov rax, rdi ; load dividend")?;
    writeln!(out, "    cqo ; sign extend dividend before divide")?;
    writeln!(out, "    idiv rsi ; divide by divisor")?;
    writeln!(
        out,
        "    mov r8, [rbp-16] ; keep env_end pointer intact for continuation"
    )?;
    writeln!(
        out,
        "    lea rcx, [r8-8] ; reserve slot for result before metadata"
    )?;
    writeln!(out, "    mov [rcx], rax ; store quotient")?;
    writeln!(out, "    mov rax, [rbp-8] ; continuation entry point")?;
    writeln!(
        out,
        "    mov rdi, r8 ; pass env_end pointer (metadata start) unchanged"
    )?;
    writeln!(out, "    leave ; unwind before jump")?;
    writeln!(out, "    jmp rax ; jump into continuation")?;
    writeln!(out)?;
    Ok(())
}

fn emit_builtin_gt<W: Write>(out: &mut W) -> Result<(), CodegenError> {
    writeln!(out, "global gt")?;
    writeln!(out, "gt:")?;
    writeln!(out, "    push rbp ; prologue: save caller frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
    writeln!(out, "    push rdx ; preserve true continuation entry")?;
    writeln!(out, "    push rcx ; preserve true continuation env_end")?;
    writeln!(out, "    push r8 ; preserve false continuation entry")?;
    writeln!(out, "    push r9 ; preserve false continuation env_end")?;
    writeln!(out, "    cmp rdi, rsi ; compare integer arguments")?;
    writeln!(out, "    jg gt_true")?;
    writeln!(out, "gt_false:")?;
    writeln!(
        out,
        "    mov rax, [rbp-24] ; false continuation entry point"
    )?;
    writeln!(
        out,
        "    mov rdi, [rbp-32] ; false continuation env_end pointer"
    )?;
    writeln!(out, "    leave")?;
    writeln!(out, "    jmp rax")?;
    writeln!(out, "gt_true:")?;
    writeln!(out, "    mov rax, [rbp-8] ; true continuation entry point")?;
    writeln!(
        out,
        "    mov rdi, [rbp-16] ; true continuation env_end pointer"
    )?;
    writeln!(out, "    leave")?;
    writeln!(out, "    jmp rax")?;
    writeln!(out)?;
    Ok(())
}

fn emit_builtin_lt<W: Write>(out: &mut W) -> Result<(), CodegenError> {
    writeln!(out, "global lt")?;
    writeln!(out, "lt:")?;
    writeln!(out, "    push rbp ; prologue: save caller frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
    writeln!(out, "    push rdx ; preserve true continuation entry")?;
    writeln!(out, "    push rcx ; preserve true continuation env_end")?;
    writeln!(out, "    push r8 ; preserve false continuation entry")?;
    writeln!(out, "    push r9 ; preserve false continuation env_end")?;
    writeln!(out, "    cmp rdi, rsi ; compare integer arguments")?;
    writeln!(out, "    jl lt_true")?;
    writeln!(out, "lt_false:")?;
    writeln!(
        out,
        "    mov rax, [rbp-24] ; false continuation entry point"
    )?;
    writeln!(
        out,
        "    mov rdi, [rbp-32] ; false continuation env_end pointer"
    )?;
    writeln!(out, "    leave")?;
    writeln!(out, "    jmp rax")?;
    writeln!(out, "lt_true:")?;
    writeln!(out, "    mov rax, [rbp-8] ; true continuation entry point")?;
    writeln!(
        out,
        "    mov rdi, [rbp-16] ; true continuation env_end pointer"
    )?;
    writeln!(out, "    leave")?;
    writeln!(out, "    jmp rax")?;
    writeln!(out)?;
    Ok(())
}

fn emit_builtin_eqi<W: Write>(out: &mut W) -> Result<(), CodegenError> {
    writeln!(out, "global eqi")?;
    writeln!(out, "eqi:")?;
    writeln!(out, "    push rbp ; prologue: save caller frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
    writeln!(out, "    push rdx ; preserve true continuation entry")?;
    writeln!(out, "    push rcx ; preserve true continuation env_end")?;
    writeln!(out, "    push r8 ; preserve false continuation entry")?;
    writeln!(out, "    push r9 ; preserve false continuation env_end")?;
    writeln!(out, "    cmp rdi, rsi ; compare integer arguments")?;
    writeln!(out, "    jne eqi_false")?;
    writeln!(out, "eqi_true:")?;
    writeln!(out, "    mov rax, [rbp-8] ; true continuation entry point")?;
    writeln!(
        out,
        "    mov rdi, [rbp-16] ; true continuation env_end pointer"
    )?;
    writeln!(out, "    leave")?;
    writeln!(out, "    jmp rax")?;
    writeln!(out, "eqi_false:")?;
    writeln!(
        out,
        "    mov rax, [rbp-24] ; false continuation entry point"
    )?;
    writeln!(
        out,
        "    mov rdi, [rbp-32] ; false continuation env_end pointer"
    )?;
    writeln!(out, "    leave")?;
    writeln!(out, "    jmp rax")?;
    writeln!(out)?;
    Ok(())
}

fn emit_builtin_eqs<W: Write>(out: &mut W) -> Result<(), CodegenError> {
    writeln!(out, "global eqs")?;
    writeln!(out, "eqs:")?;
    writeln!(out, "    push rbp ; prologue: save caller frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
    writeln!(out, "    push rdx ; preserve true continuation entry")?;
    writeln!(out, "    push rcx ; preserve true continuation env_end")?;
    writeln!(out, "    push r8 ; preserve false continuation entry")?;
    writeln!(out, "    push r9 ; preserve false continuation env_end")?;
    writeln!(out, "    mov r10, rdi ; pointer to first string")?;
    writeln!(out, "    mov r11, rsi ; pointer to second string")?;
    writeln!(out, "eqs_loop:")?;
    writeln!(out, "    mov al, byte [r10]")?;
    writeln!(out, "    mov dl, byte [r11]")?;
    writeln!(out, "    cmp al, dl")?;
    writeln!(out, "    jne eqs_false")?;
    writeln!(out, "    test al, al")?;
    writeln!(out, "    je eqs_true")?;
    writeln!(out, "    inc r10")?;
    writeln!(out, "    inc r11")?;
    writeln!(out, "    jmp eqs_loop")?;
    writeln!(out, "eqs_true:")?;
    writeln!(out, "    mov rax, [rbp-8] ; true continuation entry point")?;
    writeln!(
        out,
        "    mov rdi, [rbp-16] ; true continuation env_end pointer"
    )?;
    writeln!(out, "    leave")?;
    writeln!(out, "    jmp rax")?;
    writeln!(out, "eqs_false:")?;
    writeln!(
        out,
        "    mov rax, [rbp-24] ; false continuation entry point"
    )?;
    writeln!(
        out,
        "    mov rdi, [rbp-32] ; false continuation env_end pointer"
    )?;
    writeln!(out, "    leave")?;
    writeln!(out, "    jmp rax")?;
    writeln!(out)?;
    Ok(())
}

fn emit_builtin_itoa<W: Write>(ctx: &mut CodegenContext, out: &mut W) -> Result<(), CodegenError> {
    let min_value_label = ctx.string_literal_label("-9223372036854775808");
    writeln!(out, "global itoa")?;
    writeln!(out, "itoa:")?;
    writeln!(out, "    push rbp ; save caller frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; establish new frame")?;
    writeln!(out, "    push rsi ; preserve continuation code pointer")?;
    writeln!(out, "    push rdx ; preserve continuation env pointer")?;
    writeln!(out, "    mov rax, rdi ; capture integer argument")?;
    writeln!(out, "    mov r10, 0x8000000000000000 ; i64 min constant")?;
    writeln!(out, "    cmp rax, r10")?;
    writeln!(out, "    je itoa_min_value")?;
    writeln!(out, "    push rdi ; keep integer while mmap runs")?;
    writeln!(out, "    mov rax, {} ; mmap syscall", SYSCALL_MMAP)?;
    writeln!(out, "    xor rdi, rdi ; addr = NULL hint")?;
    writeln!(out, "    mov rsi, 64 ; allocate buffer for digits")?;
    writeln!(
        out,
        "    mov rdx, {} ; prot = read/write",
        PROT_READ | PROT_WRITE
    )?;
    writeln!(
        out,
        "    mov r10, {} ; flags: private & anonymous",
        MAP_PRIVATE | MAP_ANONYMOUS
    )?;
    writeln!(out, "    mov r8, -1 ; fd = -1")?;
    writeln!(out, "    xor r9, r9 ; offset = 0")?;
    writeln!(out, "    syscall ; allocate buffer pages")?;
    writeln!(out, "    pop rdi ; restore integer argument")?;
    writeln!(out, "    mov r8, rax ; buffer base pointer")?;
    writeln!(out, "    xor r10, r10 ; reuse r10 as sign flag")?;
    writeln!(out, "    mov rax, rdi")?;
    writeln!(out, "    cmp rax, 0")?;
    writeln!(out, "    jge itoa_abs_done")?;
    writeln!(out, "    neg rax")?;
    writeln!(out, "    mov r10, 1")?;
    writeln!(out, "itoa_abs_done:")?;
    writeln!(out, "    lea r9, [r8+64] ; pointer past buffer end")?;
    writeln!(out, "    mov byte [r9-1], 0 ; null terminator")?;
    writeln!(out, "    mov r11, r9 ; cursor for digits")?;
    writeln!(out, "    mov rcx, 10")?;
    writeln!(out, "    cmp rax, 0")?;
    writeln!(out, "    jne itoa_digit_loop")?;
    writeln!(out, "    dec r11")?;
    writeln!(out, "    mov byte [r11], '0'")?;
    writeln!(out, "    jmp itoa_check_sign")?;
    writeln!(out, "itoa_digit_loop:")?;
    writeln!(out, "    xor rdx, rdx")?;
    writeln!(out, "    div rcx")?;
    writeln!(out, "    dec r11")?;
    writeln!(out, "    add dl, '0'")?;
    writeln!(out, "    mov [r11], dl")?;
    writeln!(out, "    test rax, rax")?;
    writeln!(out, "    jne itoa_digit_loop")?;
    writeln!(out, "itoa_check_sign:")?;
    writeln!(out, "    cmp r10, 0")?;
    writeln!(out, "    je itoa_set_pointer")?;
    writeln!(out, "    dec r11")?;
    writeln!(out, "    mov byte [r11], '-'")?;
    writeln!(out, "itoa_set_pointer:")?;
    writeln!(out, "    mov r8, r11 ; string start")?;
    writeln!(out, "    jmp itoa_tail")?;
    writeln!(out, "itoa_min_value:")?;
    writeln!(
        out,
        "    lea r8, [rel {}] ; reuse static string",
        min_value_label
    )?;
    writeln!(out, "    jmp itoa_tail")?;
    writeln!(out, "itoa_tail:")?;
    writeln!(out, "    mov rsi, [rbp-8] ; continuation code pointer")?;
    writeln!(out, "    mov rdx, [rbp-16] ; continuation env pointer")?;
    writeln!(
        out,
        "    sub rsp, 16 ; allocate temp stack for closure state"
    )?;
    writeln!(out, "    mov [rsp], rsi ; save code pointer")?;
    writeln!(out, "    mov [rsp+8], rdx ; save env_end cursor")?;
    writeln!(out, "    mov r10, [rsp+8] ; env_end cursor")?;
    writeln!(out, "    sub r10, 8 ; reserve space for string argument")?;
    writeln!(out, "    mov [r10], r8 ; store string pointer")?;
    writeln!(out, "    mov rax, [rsp] ; restore code pointer")?;
    writeln!(out, "    mov rdx, [rsp+8] ; restore env_end pointer")?;
    writeln!(out, "    add rsp, 16 ; pop temp state")?;
    writeln!(
        out,
        "    mov rdi, rdx ; pass env_end pointer to continuation"
    )?;
    writeln!(out, "    leave ; unwind before jump")?;
    writeln!(out, "    jmp rax ; jump into continuation")?;
    writeln!(out)?;
    Ok(())
}

#[cfg(test)]
mod tests {}
