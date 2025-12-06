use std::collections::{HashMap, HashSet};
use std::io::Write;

use crate::compiler::ast::TypeRef;
use crate::compiler::error::CodegenError;
use crate::compiler::hir::{Arg, Block, Invocation, Param, ENTRY_FUNCTION_NAME};
use crate::compiler::mir::{
    DeepCopy, MirEnvAllocation, MirFunction, MirFunctionBinding, MirModule, MirStatement,
    MirVariadicCallInfo, ReleaseEnv,
};
use crate::compiler::runtime::{
    env_metadata_size, ENV_METADATA_FIELD_SIZE, ENV_METADATA_POINTER_COUNT_OFFSET,
    ENV_METADATA_POINTER_LIST_OFFSET,
};
use crate::compiler::span::Span;
use crate::compiler::symbol::{FunctionSig, SymbolRegistry};
use crate::compiler::type_utils::expand_alias_chain;

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

#[derive(Clone, Debug)]
enum CallArgKind<'a> {
    Normal(&'a Arg),
    Variadic(&'a [Arg]),
}

const ARG_REGS: [&str; 8] = ["rdi", "rsi", "rdx", "rcx", "r8", "r9", "r10", "r11"];
const SYSCALL_MMAP: i32 = 9;
const SYSCALL_MUNMAP: i32 = 11;
const SYSCALL_EXIT: i32 = 60;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 2;
const MAP_ANONYMOUS: i32 = 32;
const VARIADIC_LENGTH_FIELD_SIZE: usize = 8;
const ARRAY_METADATA_EXTRA_FIELD_SIZE: usize = 8;
const ARRAY_METADATA_EXTRA_BASE_OFFSET: usize = ENV_METADATA_POINTER_LIST_OFFSET;
const ARRAY_OK_LEN_SLOT_OFFSET: usize = 24;
const ARRAY_OK_NTH_SLOT_OFFSET: usize = 16;
const ARRAY_NTH_IDX_SLOT_OFFSET: usize = 40;
const ARRAY_NTH_ONE_SLOT_OFFSET: usize = 16;
const ARRAY_NTH_NONE_SLOT_OFFSET: usize = 32;
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
    MutableBytes { name: String, size: usize },
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
            "puts" => emit_builtin_puts(ctx, out)?,
            "sub" => emit_builtin_sub(out)?,
            _ => {}
        }
    }
    ctx.emit_array_builtins(out)?;
    ctx.emit_release_helper(out)?;
    ctx.emit_deep_copy_helper(out)?;
    Ok(())
}

pub fn function<W: Write>(
    mir: MirFunction,
    symbols: &SymbolRegistry,
    ctx: &mut CodegenContext,
    out: &mut W,
) -> Result<(), CodegenError> {
    if mir.owns_self {
        ctx.ensure_root_env_slot(&mir.name);
    }
    let frame = FrameLayout::build(&mir, symbols)?;
    let mir_name = mir.name.clone();
    let mut emitter = FunctionEmitter::new(mir.clone(), out, frame, symbols, ctx);
    emitter.emit_function()?;
    ctx.push_mir_function(mir);
    if mir_name != ENTRY_FUNCTION_NAME {
        // Need to re-get mir from ctx for emit_closure_wrapper
        let mir_for_wrapper = ctx.mir_functions.last().unwrap();
        emit_closure_wrapper(mir_for_wrapper, symbols, out)?;
    }
    Ok(())
}

pub struct CodegenContext {
    string_literals: Vec<String>,
    string_map: HashMap<String, usize>,
    externs: HashSet<String>,
    globals: Vec<GlobalValue>,
    global_names: HashSet<String>,
    mir_functions: Vec<MirFunction>,
    array_builtins_emitted: bool,
    release_helper_needed: bool,
    deep_copy_helper_needed: bool,
}

impl CodegenContext {
    pub fn new() -> Self {
        Self {
            string_literals: Vec::new(),
            string_map: HashMap::new(),
            externs: HashSet::new(),
            globals: Vec::new(),
            global_names: HashSet::new(),
            mir_functions: Vec::new(),
            array_builtins_emitted: false,
            release_helper_needed: false,
            deep_copy_helper_needed: false,
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
        let has_mutable = self
            .globals
            .iter()
            .any(|global| matches!(global, GlobalValue::MutableBytes { .. }));
        let has_rodata_entries = self
            .globals
            .iter()
            .any(|global| matches!(global, GlobalValue::Str { .. } | GlobalValue::Int { .. }));
        if !has_mutable && self.string_literals.is_empty() && !has_rodata_entries {
            return Ok(());
        }
        if has_mutable {
            writeln!(out, "section .bss")?;
            for global in &self.globals {
                if let GlobalValue::MutableBytes { name, size } = global {
                    writeln!(out, "{}:", name)?;
                    writeln!(out, "    resb {}", size)?;
                }
            }
        }
        if !self.string_literals.is_empty() || has_rodata_entries {
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
                    GlobalValue::MutableBytes { .. } => {}
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

    pub fn ensure_root_env_slot(&mut self, func_name: &str) {
        let symbol = Self::root_env_symbol(func_name);
        if self.global_names.contains(&symbol) {
            return;
        }
        self.global_names.insert(symbol.clone());
        self.globals.push(GlobalValue::MutableBytes {
            name: symbol,
            size: 8,
        });
    }

    pub fn root_env_symbol(func_name: &str) -> String {
        format!("{}_root_env_slot", func_name)
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

    pub fn push_mir_function(&mut self, function: MirFunction) {
        self.mir_functions.push(function);
    }

    pub fn ensure_array_builtins(&mut self) {
        if self.array_builtins_emitted {
            return;
        }
        self.array_builtins_emitted = true;
        self.push_mir_function(MirFunction::builtin_internal_array_str_nth());
        self.push_mir_function(MirFunction::builtin_internal_array_str());
    }

    pub fn ensure_release_helper(&mut self) {
        self.release_helper_needed = true;
    }

    pub fn ensure_deep_copy_helper(&mut self) {
        self.deep_copy_helper_needed = true;
    }

    pub fn emit_array_builtins<W: Write>(&self, out: &mut W) -> Result<(), CodegenError> {
        if !self.array_builtins_emitted {
            return Ok(());
        }
        writeln!(out, "global internal_array_str_nth")?;
        writeln!(out, "internal_array_str_nth:")?;
        writeln!(out, "    push rbp ; prologue: save caller frame pointer")?;
        writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
        writeln!(out, "    mov r10, rdi ; keep env_end pointer for later")?;
        writeln!(out, "    mov r8, [r10] ; load env size metadata")?;
        writeln!(
            out,
            "    mov rax, [r10+{}] ; pointer count metadata",
            ENV_METADATA_POINTER_COUNT_OFFSET
        )?;
        writeln!(
            out,
            "    imul rax, {} ; pointer metadata byte width",
            ENV_METADATA_FIELD_SIZE
        )?;
        writeln!(
            out,
            "    lea r9, [r10+{}] ; pointer metadata base",
            ARRAY_METADATA_EXTRA_BASE_OFFSET
        )?;
        writeln!(out, "    add r9, rax ; offset to array extras")?;
        writeln!(out, "    mov r9, [r9] ; load invocation slot size")?;
        writeln!(out, "    mov r11, r10 ; copy metadata pointer")?;
        writeln!(out, "    sub r11, r8 ; compute env base pointer")?;
        writeln!(out, "    mov rax, r8 ; payload plus slot bytes")?;
        writeln!(out, "    sub rax, r9 ; isolate payload size")?;
        writeln!(out, "    mov rcx, r11 ; start from env base")?;
        writeln!(out, "    add rcx, rax ; advance to payload end")?;
        writeln!(
            out,
            "    sub rcx, {} ; locate stored array length",
            VARIADIC_LENGTH_FIELD_SIZE
        )?;
        writeln!(out, "    mov rdx, [rcx] ; load array length")?;
        writeln!(
            out,
            "    mov rax, [r10-{}] ; requested index argument",
            ARRAY_NTH_IDX_SLOT_OFFSET
        )?;
        writeln!(out, "    cmp rax, 0 ; disallow negative indexes")?;
        writeln!(out, "    jl internal_array_str_nth_oob")?;
        writeln!(out, "    cmp rax, rdx ; ensure idx < len")?;
        writeln!(out, "    jge internal_array_str_nth_oob")?;
        writeln!(out, "    imul rax, 8 ; stride by element size")?;
        writeln!(out, "    add rax, r11 ; locate element slot")?;
        writeln!(out, "    mov rax, [rax] ; load string pointer")?;
        writeln!(
            out,
            "    mov rsi, [r10-{}] ; load 'one' continuation code",
            ARRAY_NTH_ONE_SLOT_OFFSET
        )?;
        writeln!(
            out,
            "    mov rdx, [r10-{}] ; load 'one' continuation env_end",
            ARRAY_NTH_ONE_SLOT_OFFSET - 8
        )?;
        writeln!(
            out,
            "    sub rsp, 16 ; allocate temp stack for closure state"
        )?;
        writeln!(out, "    mov [rsp], rsi ; save closure code pointer")?;
        writeln!(out, "    mov [rsp+8], rdx ; save closure env_end pointer")?;
        writeln!(out, "    mov rcx, [rsp+8] ; env_end pointer for argument")?;
        writeln!(
            out,
            "    sub rcx, {} ; slot for string argument",
            VARIADIC_LENGTH_FIELD_SIZE
        )?;
        writeln!(out, "    mov [rcx], rax ; store selected element")?;
        writeln!(out, "    mov rax, [rsp] ; restore closure code pointer")?;
        writeln!(
            out,
            "    mov rdx, [rsp+8] ; restore closure env_end pointer"
        )?;
        writeln!(out, "    add rsp, 16 ; drop temp state")?;
        writeln!(out, "    mov rdi, rdx ; pass env_end to continuation")?;
        writeln!(out, "    leave ; epilogue before jump")?;
        writeln!(out, "    jmp rax ; return into 'one' continuation")?;
        writeln!(out, "internal_array_str_nth_oob:")?;
        writeln!(
            out,
            "    mov rax, [r10-{}] ; load 'none' continuation code",
            ARRAY_NTH_NONE_SLOT_OFFSET
        )?;
        writeln!(
            out,
            "    mov rdx, [r10-{}] ; load 'none' continuation env_end",
            ARRAY_NTH_NONE_SLOT_OFFSET - 8
        )?;
        writeln!(out, "    mov rdi, rdx ; pass env_end pointer")?;
        writeln!(out, "    leave ; epilogue before jump")?;
        writeln!(out, "    jmp rax ; return into 'none' continuation")?;
        writeln!(out, "global internal_array_str")?;
        writeln!(out, "internal_array_str:")?;
        writeln!(out, "    push rbp ; prologue: save caller frame pointer")?;
        writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
        writeln!(out, "    mov r10, rdi ; capture env_end pointer")?;
        writeln!(out, "    mov r8, [r10] ; load env size metadata")?;
        writeln!(
            out,
            "    mov rax, [r10+{}] ; pointer count metadata",
            ENV_METADATA_POINTER_COUNT_OFFSET
        )?;
        writeln!(
            out,
            "    imul rax, {} ; pointer metadata byte width",
            ENV_METADATA_FIELD_SIZE
        )?;
        writeln!(
            out,
            "    lea r9, [r10+{}] ; pointer metadata base",
            ARRAY_METADATA_EXTRA_BASE_OFFSET
        )?;
        writeln!(out, "    add r9, rax ; offset to array extras")?;
        writeln!(out, "    mov r9, [r9] ; load invocation slot size")?;
        writeln!(out, "    mov r11, r10 ; duplicate pointer")?;
        writeln!(out, "    sub r11, r8 ; compute env base")?;
        writeln!(out, "    mov rax, r8 ; payload plus slot bytes")?;
        writeln!(out, "    sub rax, r9 ; isolate payload size")?;
        writeln!(out, "    mov rcx, r11 ; start from env base")?;
        writeln!(out, "    add rcx, rax ; advance to payload end")?;
        writeln!(
            out,
            "    sub rcx, {} ; locate stored array length",
            VARIADIC_LENGTH_FIELD_SIZE
        )?;
        writeln!(out, "    mov r9, [rcx] ; load array length")?;
        writeln!(
            out,
            "    mov rax, [r10-{}] ; load 'ok' continuation code",
            ARRAY_OK_NTH_SLOT_OFFSET
        )?;
        writeln!(
            out,
            "    mov rdx, [r10-{}] ; load 'ok' continuation env_end",
            ARRAY_OK_NTH_SLOT_OFFSET - 8
        )?;
        writeln!(
            out,
            "    sub rsp, 16 ; allocate temp stack for closure state"
        )?;
        writeln!(out, "    mov [rsp], rax ; save closure code pointer")?;
        writeln!(out, "    mov [rsp+8], rdx ; save closure env_end pointer")?;
        writeln!(out, "    mov rsi, [rsp+8] ; env_end pointer for args")?;
        writeln!(
            out,
            "    sub rsi, {} ; slot for len argument",
            ARRAY_OK_LEN_SLOT_OFFSET
        )?;
        writeln!(out, "    mov [rsi], r9 ; write len argument")?;
        writeln!(out, "    mov rsi, [rsp+8] ; env_end pointer for args")?;
        writeln!(
            out,
            "    sub rsi, {} ; slot for nth continuation",
            ARRAY_OK_NTH_SLOT_OFFSET
        )?;
        writeln!(
            out,
            "    mov qword [rsi], internal_array_str_nth ; install nth code"
        )?;
        writeln!(out, "    mov [rsi+8], r10 ; install nth env_end pointer")?;
        writeln!(out, "    mov rax, [rsp] ; restore closure code pointer")?;
        writeln!(
            out,
            "    mov rdx, [rsp+8] ; restore closure env_end pointer"
        )?;
        writeln!(out, "    add rsp, 16 ; drop temp stack")?;
        writeln!(out, "    mov rdi, rdx ; pass env_end pointer")?;
        writeln!(out, "    leave ; epilogue before jump")?;
        writeln!(out, "    jmp rax ; return into 'ok' continuation")?;
        Ok(())
    }

    pub fn emit_release_helper<W: Write>(&self, out: &mut W) -> Result<(), CodegenError> {
        if !self.release_helper_needed {
            return Ok(());
        }
        writeln!(out, "internal_release_env:")?;
        writeln!(out, "    push rbp ; prologue: save caller frame pointer")?;
        writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
        writeln!(out, "    push rbx ; preserve callee-saved registers")?;
        writeln!(out, "    push r12")?;
        writeln!(out, "    push r13")?;
        writeln!(out, "    push r14")?;
        writeln!(out, "    push r15")?;
        writeln!(out, "    mov r12, rdi ; capture env_end pointer")?;
        writeln!(out, "    test r12, r12 ; skip null releases")?;
        writeln!(out, "    je internal_release_env_done")?;
        writeln!(out, "    mov rcx, [r12] ; load env size metadata")?;
        writeln!(
            out,
            "    mov r15, [r12+{}] ; load heap size metadata",
            ENV_METADATA_FIELD_SIZE
        )?;
        writeln!(out, "    mov rbx, r12 ; copy env_end pointer")?;
        writeln!(out, "    sub rbx, rcx ; compute env base pointer")?;
        writeln!(
            out,
            "    mov r13, [r12+{}] ; load pointer count metadata",
            ENV_METADATA_POINTER_COUNT_OFFSET
        )?;
        writeln!(
            out,
            "    lea r14, [r12+{}] ; pointer metadata base",
            ENV_METADATA_POINTER_LIST_OFFSET
        )?;
        writeln!(out, "    xor r9d, r9d ; reset pointer metadata index")?;
        writeln!(out, "internal_release_env_loop:")?;
        writeln!(out, "    cmp r9, r13 ; finished child pointers?")?;
        writeln!(out, "    jge internal_release_env_children_done")?;
        writeln!(out, "    mov r10, [r14+r9*8] ; load child env offset")?;
        writeln!(out, "    mov r11, [rbx+r10] ; load child env_end pointer")?;
        writeln!(out, "    mov rdi, r11 ; pass child env_end pointer")?;
        writeln!(
            out,
            "    call internal_release_env ; recurse into child closure"
        )?;
        writeln!(out, "    inc r9 ; advance metadata index")?;
        writeln!(out, "    jmp internal_release_env_loop")?;
        writeln!(out, "internal_release_env_children_done:")?;
        writeln!(out, "    mov rdi, rbx ; env base for munmap")?;
        writeln!(out, "    mov rax, {} ; munmap syscall", SYSCALL_MUNMAP)?;
        writeln!(out, "    mov rsi, r15 ; heap size for munmap")?;
        writeln!(out, "    syscall ; release closure environment")?;
        writeln!(out, "internal_release_env_done:")?;
        writeln!(out, "    pop r15")?;
        writeln!(out, "    pop r14")?;
        writeln!(out, "    pop r13")?;
        writeln!(out, "    pop r12")?;
        writeln!(out, "    pop rbx")?;
        writeln!(out, "    pop rbp")?;
        writeln!(out, "    ret")?;
        Ok(())
    }

    pub fn emit_deep_copy_helper<W: Write>(&self, out: &mut W) -> Result<(), CodegenError> {
        if !self.deep_copy_helper_needed {
            return Ok(());
        }
        writeln!(out, "internal_deep_copy_env:")?;
        writeln!(out, "    push rbp ; prologue: save caller frame pointer")?;
        writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
        writeln!(out, "    push rbx ; preserve callee-saved registers")?;
        writeln!(out, "    push r12")?;
        writeln!(out, "    push r13")?;
        writeln!(out, "    push r14")?;
        writeln!(out, "    push r15")?;
        writeln!(out, "    mov r12, rdi ; capture source env_end pointer")?;
        writeln!(out, "    test r12, r12 ; skip null copies")?;
        writeln!(out, "    je internal_deep_copy_env_null_return")?;
        writeln!(out, "    mov rcx, [r12] ; load source env size metadata")?;
        writeln!(
            out,
            "    mov r15, [r12+{}] ; load source heap size metadata",
            ENV_METADATA_FIELD_SIZE
        )?;
        writeln!(out, "    mov rbx, r12 ; copy env_end pointer")?;
        writeln!(out, "    sub rbx, rcx ; compute source env base pointer")?;
        writeln!(out, "    mov rdi, 0 ; addr = 0 for mmap (kernel chooses)")?;
        writeln!(out, "    mov rsi, r15 ; length = heap size")?;
        writeln!(
            out,
            "    mov rdx, {} ; prot = PROT_READ | PROT_WRITE",
            PROT_READ | PROT_WRITE
        )?;
        writeln!(
            out,
            "    mov r10, {} ; flags = MAP_PRIVATE | MAP_ANONYMOUS",
            MAP_PRIVATE | MAP_ANONYMOUS
        )?;
        writeln!(out, "    mov r8, -1 ; fd = -1")?;
        writeln!(out, "    mov r9, 0 ; offset = 0")?;
        writeln!(out, "    mov rax, {} ; mmap syscall", SYSCALL_MMAP)?;
        writeln!(out, "    syscall ; allocate new heap")?;
        writeln!(out, "    mov r13, rax ; save new env base pointer")?;
        writeln!(out, "    mov rdi, r13 ; dest = new env base")?;
        writeln!(out, "    mov rsi, rbx ; src = source env base")?;
        writeln!(out, "    mov rdx, r15 ; count = heap size")?;
        writeln!(out, "    call internal_memcpy ; copy entire heap")?;
        writeln!(
            out,
            "    mov rax, [rbx] ; load code pointer from source env"
        )?;
        writeln!(out, "    mov r14, r13 ; compute new env_end")?;
        writeln!(out, "    add r14, rcx ; env_end = env_base + env_size")?;
        writeln!(out, "    mov rsi, rax ; return code pointer in rsi")?;
        writeln!(out, "    mov rdi, r14 ; return env_end pointer in rdi")?;
        writeln!(out, "    pop r15")?;
        writeln!(out, "    pop r14")?;
        writeln!(out, "    pop r13")?;
        writeln!(out, "    pop r12")?;
        writeln!(out, "    pop rbx")?;
        writeln!(out, "    pop rbp")?;
        writeln!(out, "    ret")?;
        writeln!(out, "internal_deep_copy_env_null_return:")?;
        writeln!(out, "    xor rsi, rsi ; return null code pointer")?;
        writeln!(out, "    xor rdi, rdi ; return null env_end pointer")?;
        writeln!(out, "    pop r15")?;
        writeln!(out, "    pop r14")?;
        writeln!(out, "    pop r13")?;
        writeln!(out, "    pop r12")?;
        writeln!(out, "    pop rbx")?;
        writeln!(out, "    pop rbp")?;
        writeln!(out, "    ret")?;

        // Also emit internal_memcpy if needed
        self.emit_memcpy_helper(out)?;

        Ok(())
    }

    fn emit_memcpy_helper<W: Write>(&self, out: &mut W) -> Result<(), CodegenError> {
        writeln!(out, "internal_memcpy:")?;
        writeln!(out, "    push rbp ; prologue")?;
        writeln!(out, "    mov rbp, rsp")?;
        writeln!(out, "    xor rcx, rcx ; counter = 0")?;
        writeln!(out, "internal_memcpy_loop:")?;
        writeln!(out, "    cmp rcx, rdx ; counter < count?")?;
        writeln!(out, "    jge internal_memcpy_done")?;
        writeln!(out, "    mov rax, [rsi+rcx] ; load 8 bytes from source")?;
        writeln!(out, "    mov [rdi+rcx], rax ; store 8 bytes to destination")?;
        writeln!(out, "    add rcx, 8 ; advance counter by 8")?;
        writeln!(out, "    jmp internal_memcpy_loop")?;
        writeln!(out, "internal_memcpy_done:")?;
        writeln!(out, "    pop rbp")?;
        writeln!(out, "    ret")?;
        Ok(())
    }

    pub fn take_mir_module(&mut self) -> MirModule {
        let functions = std::mem::take(&mut self.mir_functions);
        MirModule::new(functions)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValueKind {
    Word,
    Closure,
}

#[derive(Clone, Debug)]
struct Binding {
    offset: i32,
    kind: ValueKind,
    continuation_params: Vec<TypeRef>,
    env_size: usize,
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
    fn build(mir: &MirFunction, symbols: &SymbolRegistry) -> Result<Self, CodegenError> {
        let mut layout = Self {
            bindings: HashMap::new(),
            stack_size: 0,
            next_offset: 0,
        };
        for param in &mir.params {
            layout.allocate_param(param, symbols)?;
        }
        for stmt in &mir.block.items {
            if let Some((name, span)) = mir_statement_binding_info(stmt) {
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
        let env_size = if kind == ValueKind::Closure {
            env_size_bytes(&continuation_params, symbols)
        } else {
            0
        };
        self.allocate_binding(name, param.span, kind, continuation_params, env_size)
    }

    fn allocate_word(&mut self, name: &str, span: Span) -> Result<(), CodegenError> {
        self.allocate_binding(name, span, ValueKind::Word, Vec::new(), 0)
    }

    fn allocate_binding(
        &mut self,
        name: &str,
        span: Span,
        kind: ValueKind,
        continuation_params: Vec<TypeRef>,
        env_size: usize,
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
                env_size,
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

fn mir_statement_binding_info<'a>(stmt: &'a MirStatement) -> Option<(&'a str, Span)> {
    match stmt {
        MirStatement::FunctionDef(binding) => Some((binding.name.as_str(), binding.span)),
        MirStatement::StrDef(literal) => Some((literal.name.as_str(), literal.span)),
        MirStatement::IntDef(literal) => Some((literal.name.as_str(), literal.span)),
        MirStatement::ApplyDef(apply) => Some((apply.apply.name.as_str(), apply.apply.span)),
        MirStatement::Invocation(invocation) => invocation
            .invocation
            .result
            .as_ref()
            .map(|name| (name.as_str(), invocation.invocation.span)),
        MirStatement::ReleaseEnv(_) => None,
        MirStatement::DeepCopy(copy) => Some((copy.copy.as_str(), copy.span)),
    }
}

#[derive(Clone, Debug)]
struct ClosureState {
    remaining: Vec<TypeRef>,
    env_size: usize,
}

impl ClosureState {
    fn new(params: Vec<TypeRef>, env_size: usize) -> Self {
        Self {
            remaining: params,
            env_size,
        }
    }

    fn remaining(&self) -> &[TypeRef] {
        &self.remaining
    }

    fn env_size(&self) -> usize {
        self.env_size
    }

    fn after_applying(&self, count: usize) -> Self {
        Self {
            remaining: self.remaining[count..].to_vec(),
            env_size: self.env_size,
        }
    }
}

#[derive(Clone, Debug)]
enum ExprValue {
    Word,
    Closure(ClosureState),
}

struct FunctionEmitter<'a, W: Write> {
    mir: MirFunction,
    out: &'a mut W,
    frame: FrameLayout,
    symbols: &'a SymbolRegistry,
    ctx: &'a mut CodegenContext,
    literal_strings: HashMap<String, String>,
    terminated: bool,
    write_loop_counter: usize,
    label_counter: usize,
    root_env_symbol: Option<String>,
}

impl<'a, W: Write> FunctionEmitter<'a, W> {
    fn new(
        mir: MirFunction,
        out: &'a mut W,
        frame: FrameLayout,
        symbols: &'a SymbolRegistry,
        ctx: &'a mut CodegenContext,
    ) -> Self {
        let root_env_symbol = if mir.owns_self {
            Some(CodegenContext::root_env_symbol(&mir.name))
        } else {
            None
        };
        Self {
            mir,
            out,
            frame,
            symbols,
            ctx,
            literal_strings: HashMap::new(),
            terminated: false,
            write_loop_counter: 0,
            label_counter: 0,
            root_env_symbol,
        }
    }

    fn emit_function(&mut self) -> Result<(), CodegenError> {
        self.write_header()?;
        self.write_prologue()?;
        self.store_params()?;
        self.emit_block()?;
        if !self.terminated {
            self.write_epilogue()?;
        }
        Ok(())
    }

    fn write_header(&mut self) -> Result<(), CodegenError> {
        writeln!(self.out, "global {}", self.mir.name)?;
        writeln!(self.out, "{}:", self.mir.name)?;
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
        for param in &self.mir.params {
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
                        self.mir.name,
                        ARG_REGS.len()
                    ),
                    self.mir.span,
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

    fn emit_block(&mut self) -> Result<(), CodegenError> {
        let statements = self.mir.block.items.clone();
        for stmt in statements {
            self.emit_statement(&stmt)?;
        }
        Ok(())
    }

    fn emit_statement(&mut self, stmt: &MirStatement) -> Result<(), CodegenError> {
        match stmt {
            MirStatement::FunctionDef(binding) => {
                let sig = self
                    .symbols
                    .get_function(&binding.function_name)
                    .ok_or_else(|| {
                        CodegenError::new(
                            format!("compiler bug: unknown function '{}'", binding.function_name),
                            binding.span,
                        )
                    })?;
                let state = if self.is_self_owned_binding(binding) {
                    self.emit_self_owned_closure(binding, sig)?
                } else {
                    self.emit_named_function_closure(
                        &binding.function_name,
                        sig,
                        binding.env_allocation.as_ref(),
                    )?;
                    let env_size = binding
                        .env_allocation
                        .as_ref()
                        .map(|alloc| alloc.env_size)
                        .unwrap_or_else(|| env_size_bytes(&sig.params, self.symbols));
                    ClosureState::new(sig.params.clone(), env_size)
                };
                let result = ExprValue::Closure(state);
                self.store_binding_value(&binding.name, result, binding.span)?;
                return Ok(());
            }
            MirStatement::StrDef(literal) => {
                self.literal_strings
                    .insert(literal.name.clone(), literal.value.clone());
                let expr = Expr::String {
                    value: literal.value.clone(),
                    span: literal.span,
                };
                let value = self.emit_expr(&expr)?;
                self.store_binding_value(&literal.name, value, literal.span)?;
                return Ok(());
            }
            MirStatement::IntDef(literal) => {
                let expr = Expr::Int {
                    value: literal.value,
                    span: literal.span,
                };
                let value = self.emit_expr(&expr)?;
                self.store_binding_value(&literal.name, value, literal.span)?;
                return Ok(());
            }
            MirStatement::ApplyDef(apply) => {
                let invocation = Invocation {
                    of: apply.apply.of.clone(),
                    args: apply.apply.args.clone(),
                    span: apply.apply.span,
                    result: None,
                };
                let value = self.emit_invocation_value(&invocation, apply.variadic.as_ref())?;
                self.store_binding_value(&apply.apply.name, value, apply.apply.span)?;
                return Ok(());
            }
            MirStatement::Invocation(invocation) if invocation.invocation.result.is_some() => {
                let name = invocation.invocation.result.as_ref().unwrap().clone();
                let value = self
                    .emit_invocation_value(&invocation.invocation, invocation.variadic.as_ref())?;
                self.store_binding_value(&name, value, invocation.invocation.span)?;
                return Ok(());
            }
            MirStatement::Invocation(invocation) => {
                let result =
                    self.emit_invocation(&invocation.invocation, invocation.variadic.as_ref())?;
                if let ExprValue::Closure(_) = result {
                    // discard unused closures to avoid leaking temporaries
                }
                return Ok(());
            }
            MirStatement::ReleaseEnv(release) => {
                self.emit_release_env(release)?;
                return Ok(());
            }
            MirStatement::DeepCopy(copy) => {
                self.emit_deep_copy(copy)?;
                return Ok(());
            }
        }
    }

    fn emit_release_env(&mut self, release: &ReleaseEnv) -> Result<(), CodegenError> {
        let binding = self.frame.binding(&release.name).cloned().ok_or_else(|| {
            CodegenError::new(format!("unknown binding '{}'", release.name), release.span)
        })?;
        if binding.kind != ValueKind::Closure {
            return Err(CodegenError::new(
                format!("cannot release non-closure binding '{}'", release.name),
                release.span,
            ));
        }
        self.ctx.ensure_release_helper();
        let env_offset = binding.slot_addr(1);
        writeln!(
            self.out,
            "    mov rdi, [rbp-{}] ; load closure env_end pointer",
            env_offset
        )?;
        writeln!(
            self.out,
            "    call internal_release_env ; release closure environment"
        )?;
        Ok(())
    }

    fn emit_deep_copy(&mut self, copy: &DeepCopy) -> Result<(), CodegenError> {
        let binding = self.frame.binding(&copy.original).cloned().ok_or_else(|| {
            CodegenError::new(format!("unknown binding '{}'", copy.original), copy.span)
        })?;
        if binding.kind != ValueKind::Closure {
            return Err(CodegenError::new(
                format!("cannot deep copy non-closure binding '{}'", copy.original),
                copy.span,
            ));
        }

        // Get the original closure's code and env_end pointers
        let code_offset = binding.slot_addr(0);
        let env_offset = binding.slot_addr(1);

        // Save the code pointer
        writeln!(
            self.out,
            "    mov rax, [rbp-{}] ; load original closure code pointer",
            code_offset
        )?;
        writeln!(self.out, "    push rax ; save code pointer")?;

        // Get the env_end pointer and call deep_copy_env
        writeln!(
            self.out,
            "    mov rdi, [rbp-{}] ; load original closure env_end pointer",
            env_offset
        )?;

        // Call deep_copy_env which copies the environment
        // Returns: rsi = code pointer (but we'll use the original), rdi = new env_end pointer
        writeln!(
            self.out,
            "    call internal_deep_copy_env ; deep copy closure environment"
        )?;

        // Restore the code pointer and prepare result
        writeln!(self.out, "    pop rax ; restore original code pointer")?;
        writeln!(
            self.out,
            "    mov rdx, rdi ; move new env_end pointer to rdx"
        )?;

        // Store the new closure with the original code pointer and new env
        let copy_state = ClosureState::new(binding.continuation_params.clone(), binding.env_size);
        let copy_value = ExprValue::Closure(copy_state);
        self.store_binding_value(&copy.copy, copy_value, copy.span)?;

        self.ctx.ensure_deep_copy_helper();

        Ok(())
    }

    fn emit_invocation_value(
        &mut self,
        invocation: &Invocation,
        variadic: Option<&MirVariadicCallInfo>,
    ) -> Result<ExprValue, CodegenError> {
        if let Some(sig) = self.symbols.get_function(&invocation.of) {
            self.emit_named_function_closure(&invocation.of, sig, None)?;
            let env_size = env_size_bytes(&sig.params, self.symbols);
            let state = ClosureState::new(sig.params.clone(), env_size);
            return self.apply_closure(state, &invocation.args, invocation.span, false);
        }
        if let Some(binding) = self.frame.binding(&invocation.of) {
            if binding.kind != ValueKind::Closure {
                return Err(CodegenError::new(
                    format!("'{}' is not callable", invocation.of),
                    invocation.span,
                ));
            }
            writeln!(
                self.out,
                "    mov rax, [rbp-{}] ; load closure code for value call",
                binding.slot_addr(0)
            )?;
            writeln!(
                self.out,
                "    mov rdx, [rbp-{}] ; load closure env_end for value call",
                binding.slot_addr(1)
            )?;
            let state = ClosureState::new(binding.continuation_params.clone(), binding.env_size);
            return self.apply_closure(state, &invocation.args, invocation.span, false);
        }
        self.emit_invocation(invocation, variadic)
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
                binding.env_size = state.env_size();
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
                                binding.env_size,
                            )))
                        }
                    }
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

    fn emit_invocation(
        &mut self,
        invocation: &Invocation,
        variadic: Option<&MirVariadicCallInfo>,
    ) -> Result<ExprValue, CodegenError> {
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
        if name == "puts" {
            return self.emit_puts_call(&invocation.args, span);
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
            let state = ClosureState::new(binding.continuation_params.clone(), binding.env_size);
            return self.apply_closure(state, &invocation.args, span, true);
        }

        if let Some(sig) = self.symbols.get_function(name) {
            return self.emit_named_exec(name, sig, &invocation.args, span, variadic);
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

    fn emit_puts_call(&mut self, args: &[Arg], span: Span) -> Result<ExprValue, CodegenError> {
        if args.len() < 2 {
            return Err(CodegenError::new(
                "puts requires a string and a continuation",
                span,
            ));
        }

        let continuation_arg = &args[args.len() - 1];
        let continuation_value = self.emit_arg_value(continuation_arg)?;
        let closure_state = match continuation_value {
            ExprValue::Closure(state) => state,
            _ => {
                return Err(CodegenError::new(
                    "last argument to puts must be a continuation",
                    continuation_arg.span,
                ))
            }
        };

        if !closure_state.remaining().is_empty() {
            return Err(CodegenError::new(
                "puts continuation must accept no arguments",
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
                "puts requires a string before the continuation",
                span,
            ));
        }

        let params = vec![TypeRef::Str];
        self.prepare_call_args(call_args, &params)?;
        self.move_args_to_registers(&params)?;

        self.ctx.add_extern("puts");
        writeln!(self.out, "    call puts ; invoke libc puts")?;

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

    fn emit_named_exec(
        &mut self,
        of: &str,
        sig: &FunctionSig,
        args: &[Arg],
        span: Span,
        variadic: Option<&MirVariadicCallInfo>,
    ) -> Result<ExprValue, CodegenError> {
        let has_variadic = sig.is_variadic.iter().any(|&flag| flag);
        if !has_variadic && args.len() > sig.params.len() {
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

        if !has_variadic && args.len() < sig.params.len() {
            self.emit_named_function_closure(of, sig, None)?;
            let env_size = env_size_bytes(&sig.params, self.symbols);
            let state = ClosureState::new(sig.params.clone(), env_size);
            return self.apply_closure(state, args, span, true);
        }

        if has_variadic {
            let layout_info = variadic.ok_or_else(|| {
                CodegenError::new(
                    format!(
                        "compiler bug: variadic invocation metadata missing for '{}'",
                        of
                    ),
                    span,
                )
            })?;
            self.push_variadic_call_args(args, &sig.params, of, span, layout_info)?;
        } else {
            self.prepare_call_args(args, &sig.params)?;
        }
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
        allocation: Option<&MirEnvAllocation>,
    ) -> Result<(), CodegenError> {
        let pointer_offsets = env_pointer_offsets(&sig.params, self.symbols);
        let metadata_size = env_metadata_size(pointer_offsets.len());
        let (env_size, heap_size) = if let Some(plan) = allocation {
            (plan.env_size, plan.heap_size)
        } else {
            let env_size = env_size_bytes(&sig.params, self.symbols);
            (env_size, env_size + metadata_size)
        };
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
        writeln!(
            self.out,
            "    mov qword [rdx+{}], {} ; pointer count metadata",
            ENV_METADATA_POINTER_COUNT_OFFSET,
            pointer_offsets.len()
        )?;
        for (idx, offset) in pointer_offsets.iter().enumerate() {
            let meta_offset = ENV_METADATA_POINTER_LIST_OFFSET + idx * ENV_METADATA_FIELD_SIZE;
            writeln!(
                self.out,
                "    mov qword [rdx+{}], {} ; closure env pointer slot offset",
                meta_offset, offset
            )?;
        }
        let wrapper = closure_wrapper_label(name);
        writeln!(
            self.out,
            "    mov rax, {} ; load wrapper entry point",
            wrapper
        )?;
        Ok(())
    }

    fn is_self_owned_binding(&self, binding: &MirFunctionBinding) -> bool {
        self.mir.owns_self
            && binding.function_name == self.mir.name
            && self.root_env_symbol.is_some()
    }

    fn emit_self_owned_closure(
        &mut self,
        binding: &MirFunctionBinding,
        sig: &FunctionSig,
    ) -> Result<ClosureState, CodegenError> {
        let slot = self
            .root_env_symbol
            .as_ref()
            .ok_or_else(|| CodegenError::new("missing root env slot", binding.span))?
            .clone();
        let reuse_label = self.next_internal_label("root_env_reuse");
        let done_label = self.next_internal_label("root_env_done");
        writeln!(
            self.out,
            "    mov rdx, [rel {}] ; load cached root env pointer",
            slot
        )?;
        writeln!(self.out, "    test rdx, rdx ; check for cached env")?;
        writeln!(self.out, "    jne {}", reuse_label)?;
        self.emit_named_function_closure(
            &binding.function_name,
            sig,
            binding.env_allocation.as_ref(),
        )?;
        writeln!(
            self.out,
            "    mov [rel {}], rdx ; cache root env pointer",
            slot
        )?;
        writeln!(self.out, "    jmp {}", done_label)?;
        writeln!(self.out, "{}:", reuse_label)?;
        let wrapper = closure_wrapper_label(&binding.function_name);
        writeln!(
            self.out,
            "    mov rax, {} ; reuse cached root entry point",
            wrapper
        )?;
        writeln!(self.out, "{}:", done_label)?;
        let env_size = binding
            .env_allocation
            .as_ref()
            .map(|allocation| allocation.env_size)
            .unwrap_or_else(|| env_size_bytes(&sig.params, self.symbols));
        Ok(ClosureState::new(sig.params.clone(), env_size))
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
                self.mir.span,
            ));
        }
        for (arg, ty) in args.iter().zip(params).rev() {
            let value = self.emit_arg_value(arg)?;
            self.ensure_value_matches(&value, ty, arg.span)?;
            self.push_value(&value)?;
        }
        Ok(())
    }

    fn push_call_layout(
        &mut self,
        layout: &[CallArgKind],
        params: &[TypeRef],
        callee: &str,
        span: Span,
    ) -> Result<(), CodegenError> {
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
                self.mir.span,
            ));
        }
        for (kind, ty) in layout.iter().zip(params).rev() {
            match kind {
                CallArgKind::Normal(arg) => {
                    let value = self.emit_arg_value(arg)?;
                    self.ensure_value_matches(&value, ty, arg.span)?;
                    self.push_value(&value)?;
                }
                CallArgKind::Variadic(slice) => {
                    let value = self.emit_variadic_array_arg(callee, ty, slice, span)?;
                    self.push_value(&value)?;
                }
            }
        }
        Ok(())
    }

    fn push_variadic_call_args(
        &mut self,
        args: &[Arg],
        params: &[TypeRef],
        callee: &str,
        span: Span,
        layout: &MirVariadicCallInfo,
    ) -> Result<(), CodegenError> {
        if args.len() < layout.required_arguments() {
            return Err(CodegenError::new(
                format!(
                    "function '{callee}' expected at least {} arguments but got {}",
                    layout.required_arguments(),
                    args.len()
                ),
                span,
            ));
        }
        let prefix_required = layout.prefix_len;
        let suffix_required = layout.suffix_len;
        let suffix_arg_start = args.len() - suffix_required;
        if suffix_arg_start < prefix_required {
            return Err(CodegenError::new(
                format!(
                    "function '{callee}' expected at least {} arguments but got {}",
                    layout.required_arguments(),
                    args.len()
                ),
                span,
            ));
        }
        let mut call_layout = Vec::with_capacity(params.len());
        for idx in 0..prefix_required {
            call_layout.push(CallArgKind::Normal(&args[idx]));
        }
        call_layout.push(CallArgKind::Variadic(
            &args[prefix_required..suffix_arg_start],
        ));
        for idx in 0..suffix_required {
            call_layout.push(CallArgKind::Normal(&args[suffix_arg_start + idx]));
        }
        self.push_call_layout(&call_layout, params, callee, span)
    }

    fn emit_variadic_array_arg(
        &mut self,
        callee: &str,
        param_ty: &TypeRef,
        args: &[Arg],
        span: Span,
    ) -> Result<ExprValue, CodegenError> {
        let element_ty = variadic_element_type(param_ty, self.symbols).ok_or_else(|| {
            CodegenError::new(
                format!(
                    "cannot determine element type for variadic parameter of '{}'",
                    callee
                ),
                span,
            )
        })?;
        self.ctx.ensure_array_builtins();
        let element_kind = resolved_type_kind(&element_ty, self.symbols);
        let element_size = bytes_for_type(&element_ty, self.symbols);
        let payload_size = element_size * args.len() + VARIADIC_LENGTH_FIELD_SIZE;
        let params = continuation_params_for_type(param_ty, self.symbols);
        let call_slot_size = env_size_bytes(&params, self.symbols);
        let mut nth_slot_size = 0usize;
        if let Some(ok_type) = params.get(0) {
            let ok_params = continuation_params_for_type(ok_type, self.symbols);
            if let Some(nth_type) = ok_params.get(1) {
                let nth_params = continuation_params_for_type(nth_type, self.symbols);
                nth_slot_size = env_size_bytes(&nth_params, self.symbols);
            }
        }
        let invocation_slot_size = call_slot_size.max(nth_slot_size);
        let env_size = payload_size + invocation_slot_size;
        let pointer_count = if element_kind == ValueKind::Closure {
            args.len()
        } else {
            0
        };
        let metadata_size = env_metadata_size(pointer_count);
        let heap_size = env_size + metadata_size + ARRAY_METADATA_EXTRA_FIELD_SIZE;
        self.emit_mmap(heap_size)?;
        writeln!(
            self.out,
            "    mov r14, rax ; stash base pointer for variadic array"
        )?;
        let mut offset = 0usize;
        let mut closure_offsets = Vec::with_capacity(args.len());
        for arg in args {
            let value = self.emit_arg_value(arg)?;
            self.ensure_value_matches(&value, &element_ty, arg.span)?;
            match element_kind {
                ValueKind::Word => {
                    writeln!(
                        self.out,
                        "    mov [r14+{}], rax ; store variadic argument '{}'",
                        offset, arg.name
                    )?;
                }
                ValueKind::Closure => {
                    writeln!(
                        self.out,
                        "    mov [r14+{}], rax ; store variadic closure code",
                        offset
                    )?;
                    writeln!(
                        self.out,
                        "    mov [r14+{}], rdx ; store variadic closure env_end",
                        offset + 8
                    )?;
                    closure_offsets.push(offset + 8);
                }
            }
            offset += element_size;
        }
        writeln!(
            self.out,
            "    mov qword [r14+{}], {} ; record variadic argument length",
            payload_size - VARIADIC_LENGTH_FIELD_SIZE,
            args.len()
        )?;
        writeln!(self.out, "    mov rdx, r14 ; env base pointer for array")?;
        if env_size > 0 {
            writeln!(
                self.out,
                "    add rdx, {} ; env_end pointer for array closure",
                env_size
            )?;
        }
        writeln!(
            self.out,
            "    mov qword [rdx], {} ; env size metadata for array",
            env_size
        )?;
        writeln!(
            self.out,
            "    mov qword [rdx+{}], {} ; heap size metadata for array",
            ENV_METADATA_FIELD_SIZE, heap_size
        )?;
        writeln!(
            self.out,
            "    mov qword [rdx+{}], {} ; pointer count metadata for array",
            ENV_METADATA_POINTER_COUNT_OFFSET,
            closure_offsets.len()
        )?;
        for (idx, slot_offset) in closure_offsets.iter().enumerate() {
            let meta_offset = ENV_METADATA_POINTER_LIST_OFFSET + idx * ENV_METADATA_FIELD_SIZE;
            writeln!(
                self.out,
                "    mov qword [rdx+{}], {} ; closure env pointer offset",
                meta_offset, slot_offset
            )?;
        }
        let array_extra_offset =
            ARRAY_METADATA_EXTRA_BASE_OFFSET + closure_offsets.len() * ENV_METADATA_FIELD_SIZE;
        writeln!(
            self.out,
            "    mov qword [rdx+{}], {} ; invocation slot metadata for array",
            array_extra_offset, invocation_slot_size
        )?;
        writeln!(
            self.out,
            "    mov rax, internal_array_str ; builtin array closure entry"
        )?;
        Ok(ExprValue::Closure(ClosureState::new(params, env_size)))
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
                            self.mir.span,
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

    fn clone_closure_argument(&mut self) -> Result<(), CodegenError> {
        writeln!(
            self.out,
            "    mov [rsp+16], rax ; stash closure code pointer for clone"
        )?;
        writeln!(
            self.out,
            "    mov rbx, rdx ; original closure env_end pointer"
        )?;
        writeln!(
            self.out,
            "    mov r13, [rbx] ; load env size metadata for clone"
        )?;
        writeln!(
            self.out,
            "    mov r14, [rbx+{}] ; load heap size metadata for clone",
            ENV_METADATA_FIELD_SIZE
        )?;
        writeln!(
            self.out,
            "    mov r12, rbx ; compute env base pointer for clone source"
        )?;
        writeln!(
            self.out,
            "    sub r12, r13 ; env base pointer for clone source"
        )?;
        writeln!(self.out, "    mov rax, {} ; mmap syscall", SYSCALL_MMAP)?;
        writeln!(self.out, "    xor rdi, rdi ; addr = NULL hint")?;
        writeln!(self.out, "    mov rsi, r14 ; length for cloned environment")?;
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
        writeln!(self.out, "    syscall ; allocate cloned env pages")?;
        writeln!(
            self.out,
            "    mov r15, rax ; cloned closure env base pointer"
        )?;
        writeln!(
            self.out,
            "    mov rsi, r12 ; source env base for clone copy"
        )?;
        writeln!(
            self.out,
            "    mov rdi, r15 ; destination env base for clone copy"
        )?;
        writeln!(self.out, "    mov rcx, r14 ; bytes to copy for cloned env")?;
        writeln!(self.out, "    cld ; ensure forward copy for env clone")?;
        writeln!(self.out, "    rep movsb ; duplicate closure env data")?;
        writeln!(self.out, "    mov rbx, r15 ; start from cloned env base")?;
        writeln!(
            self.out,
            "    add rbx, r13 ; compute cloned env_end pointer"
        )?;
        writeln!(
            self.out,
            "    mov rdx, rbx ; use cloned env_end pointer for argument"
        )?;
        writeln!(
            self.out,
            "    mov rax, [rsp+16] ; restore closure code pointer after clone"
        )?;
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
            "    sub rsp, 24 ; allocate temporary stack for closure state"
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
        let next_state = state.after_applying(args.len());
        let needs_clone =
            !invoke_when_ready && !next_state.remaining().is_empty() && !args.is_empty();
        if needs_clone {
            writeln!(
                self.out,
                "    mov rbx, [rsp+8] ; original closure env_end pointer"
            )?;
            writeln!(
                self.out,
                "    mov r13, [rbx] ; load env size metadata for clone"
            )?;
            writeln!(
                self.out,
                "    mov r14, [rbx+{}] ; load heap size metadata for clone",
                ENV_METADATA_FIELD_SIZE
            )?;
            writeln!(
                self.out,
                "    mov r12, rbx ; compute env base pointer for clone"
            )?;
            writeln!(
                self.out,
                "    sub r12, r13 ; env base pointer for clone source"
            )?;
            writeln!(self.out, "    mov rax, {} ; mmap syscall", SYSCALL_MMAP)?;
            writeln!(self.out, "    xor rdi, rdi ; addr = NULL hint")?;
            writeln!(self.out, "    mov rsi, r14 ; length for cloned environment")?;
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
            writeln!(self.out, "    syscall ; allocate cloned env pages")?;
            writeln!(
                self.out,
                "    mov r15, rax ; cloned closure env base pointer"
            )?;
            writeln!(
                self.out,
                "    mov rsi, r12 ; source env base for clone copy"
            )?;
            writeln!(
                self.out,
                "    mov rdi, r15 ; destination env base for clone copy"
            )?;
            writeln!(self.out, "    mov rcx, r14 ; bytes to copy for cloned env")?;
            writeln!(self.out, "    cld ; ensure forward copy for env clone")?;
            writeln!(self.out, "    rep movsb ; duplicate closure env data")?;
            writeln!(self.out, "    mov rbx, r15 ; start from cloned env base")?;
            writeln!(
                self.out,
                "    add rbx, r13 ; compute cloned env_end pointer"
            )?;
            writeln!(
                self.out,
                "    mov [rsp+8], rbx ; operate on cloned closure env"
            )?;
        }
        for (idx, (arg, ty)) in args.iter().zip(remaining.iter()).enumerate() {
            let value = self.emit_arg_value(arg)?;
            self.ensure_value_matches(&value, ty, arg.span)?;
            match resolved_type_kind(ty, self.symbols) {
                ValueKind::Word => {
                    writeln!(self.out, "    mov rbx, [rsp+8] ; env_end pointer")?;
                    writeln!(
                        self.out,
                        "    sub rbx, {} ; compute slot for next argument",
                        suffix_sizes[idx]
                    )?;
                    writeln!(self.out, "    mov [rbx], rax ; store scalar arg in env")?;
                }
                ValueKind::Closure => {
                    if !invoke_when_ready {
                        self.clone_closure_argument()?;
                    }
                    writeln!(self.out, "    mov rbx, [rsp+8] ; env_end pointer")?;
                    writeln!(
                        self.out,
                        "    sub rbx, {} ; compute slot for next argument",
                        suffix_sizes[idx]
                    )?;
                    writeln!(self.out, "    mov [rbx], rax ; store closure code for arg")?;
                    writeln!(
                        self.out,
                        "    mov [rbx+8], rdx ; store closure env_end for arg"
                    )?;
                }
            }
        }

        let remaining = next_state;
        writeln!(
            self.out,
            "    mov rax, [rsp] ; restore closure code pointer"
        )?;
        writeln!(
            self.out,
            "    mov rdx, [rsp+8] ; restore closure env_end pointer"
        )?;
        writeln!(self.out, "    add rsp, 24 ; pop temporary closure state")?;

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

    fn next_internal_label(&mut self, kind: &str) -> String {
        let idx = self.label_counter;
        self.label_counter += 1;
        format!("{}_{}_{}", self.sanitized_func_name(), kind, idx)
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
        self.mir
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

fn emit_closure_wrapper<W: Write>(
    mir: &MirFunction,
    symbols: &SymbolRegistry,
    out: &mut W,
) -> Result<(), CodegenError> {
    let total_slots: usize = mir
        .params
        .iter()
        .map(|p| slots_for_type(&p.ty, symbols))
        .sum();
    if total_slots > ARG_REGS.len() {
        return Err(CodegenError::new(
            format!(
                "function '{}' exceeds supported continuation argument slots",
                mir.name
            ),
            mir.span,
        ));
    }
    let label = closure_wrapper_label(&mir.name);
    writeln!(out, "{}:", label)?;
    writeln!(out, "    push rbp ; save caller frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; establish wrapper frame")?;
    writeln!(
        out,
        "    sub rsp, 16 ; reserve space for env metadata scratch"
    )?;
    writeln!(
        out,
        "    mov [rbp-8], rdi ; stash env_end pointer for release"
    )?;
    writeln!(out, "    push rbx ; preserve base register")?;
    writeln!(out, "    mov rbx, rdi ; rdi points to env_end when invoked")?;
    let param_types: Vec<TypeRef> = mir.params.iter().map(|p| p.ty.clone()).collect();
    let env_size = env_size_bytes(&param_types, symbols);
    if env_size > 0 {
        writeln!(out, "    sub rbx, {} ; compute env base", env_size)?;
    }
    let mut reg_slot = 0usize;
    let mut saved_regs: Vec<&str> = Vec::new();
    for (idx, param) in mir.params.iter().enumerate() {
        match resolved_type_kind(&param.ty, symbols) {
            ValueKind::Word => {
                let offset = env_offset(&mir.params, idx, symbols);
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
                let offset = env_offset(&mir.params, idx, symbols);
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
    writeln!(out, "    mov rdx, [rbp-8] ; load saved env_end pointer")?;
    writeln!(out, "    mov rcx, [rdx] ; read env size metadata")?;
    writeln!(
        out,
        "    mov rsi, [rdx+{}] ; read heap size metadata",
        ENV_METADATA_FIELD_SIZE
    )?;
    writeln!(out, "    mov rbx, rdx ; env_end pointer for release")?;
    writeln!(out, "    sub rbx, rcx ; compute env base pointer")?;
    writeln!(out, "    mov rdi, rbx ; munmap base pointer")?;
    writeln!(out, "    mov rax, {} ; munmap syscall", SYSCALL_MUNMAP)?;
    writeln!(out, "    syscall ; release wrapper closure environment")?;
    for reg in saved_regs.iter().rev() {
        writeln!(out, "    pop {} ; restore parameter register", reg)?;
    }
    writeln!(out, "    pop rbx ; restore saved base register")?;
    writeln!(out, "    leave ; epilogue: restore rbp of caller")?;
    writeln!(out, "    jmp {} ; jump into actual function", mir.name)?;
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

fn env_pointer_offsets(params: &[TypeRef], symbols: &SymbolRegistry) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut current = 0usize;
    for ty in params {
        match resolved_type_kind(ty, symbols) {
            ValueKind::Word => current += 8,
            ValueKind::Closure => {
                offsets.push(current + 8);
                current += 16;
            }
        }
    }
    offsets
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
        TypeRef::AliasInstance { name, .. } => {
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
        TypeRef::Generic(_) => ValueKind::Word,
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
        TypeRef::AliasInstance { name, .. } => {
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
        TypeRef::Generic(_) => Vec::new(),
        _ => Vec::new(),
    }
}

fn variadic_element_type(ty: &TypeRef, symbols: &SymbolRegistry) -> Option<TypeRef> {
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

fn emit_builtin_puts<W: Write>(ctx: &mut CodegenContext, out: &mut W) -> Result<(), CodegenError> {
    writeln!(out, "global rgo_puts")?;
    writeln!(out, "rgo_puts:")?;
    writeln!(out, "    push rbp ; prologue: save caller frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
    writeln!(out, "    push rsi ; preserve continuation code pointer")?;
    writeln!(out, "    push rdx ; preserve continuation env_end pointer")?;
    writeln!(out, "    ; rdi already holds the string pointer for puts")?;
    ctx.add_extern("puts");
    writeln!(out, "    call puts ; invoke libc puts")?;
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
