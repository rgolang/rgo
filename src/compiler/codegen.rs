use std::collections::{HashMap, HashSet};
use std::io::Write;

use crate::compiler::builtins::MirInstKind;
use crate::compiler::error::{Code, Error};
use crate::compiler::mir;
use crate::compiler::mir::{
    DeepCopy, MirArg, MirCall, MirCallKind, MirClosure, MirEnvBase, MirEnvField, MirExec,
    MirExecTarget, MirFunction, MirInstruction, MirStmt, MirSysCall, MirSysCallKind, Release,
    SigItem, SigKind, ValueKind,
};

pub const ENV_METADATA_FIELD_SIZE: usize = 8;
pub const ENV_METADATA_UNWRAPPER_OFFSET: usize = 0;
pub const ENV_METADATA_ENV_SIZE_OFFSET: usize = ENV_METADATA_FIELD_SIZE;
pub const ENV_METADATA_HEAP_SIZE_OFFSET: usize = ENV_METADATA_FIELD_SIZE * 2;
pub const ENV_METADATA_POINTER_COUNT_OFFSET: usize = ENV_METADATA_FIELD_SIZE * 3;
pub const ENV_METADATA_POINTER_LIST_OFFSET: usize = ENV_METADATA_FIELD_SIZE * 4;
pub const ENV_METADATA_SIZE: usize = ENV_METADATA_POINTER_LIST_OFFSET;

pub fn env_metadata_size(pointer_count: usize) -> usize {
    ENV_METADATA_SIZE + pointer_count * ENV_METADATA_FIELD_SIZE
}

use crate::compiler::span::Span;
use crate::{escape_literal_for_rodata, sanitize_function_name};
const WORD_SIZE: usize = 8;

#[derive(Clone, Debug)]
enum Expr {
    Int { value: i64 },
    Ident { name: String, span: Span },
    String { label: String },
}

#[derive(Clone, Copy, Debug)]
struct ArgSplit {
    reg_slots: usize,
    stack_bytes: usize,
}

const ARG_REGS: [&str; 12] = [
    "rdi", "rsi", "rdx", "rcx", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15",
];
const SYSCALL_MMAP: i32 = 9;
const SYSCALL_MUNMAP: i32 = 11;
const SYSCALL_EXIT: i32 = 60;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 2;
const MAP_ANONYMOUS: i32 = 32;
const FMT_BUFFER_SIZE: usize = 1024;

#[derive(Debug, Default)]
pub struct Artifacts {
    string_literals: Vec<(String, String)>,
    pub externs: HashSet<String>,
}

impl Artifacts {
    pub fn collect(mir_functions: &[MirFunction]) -> Self {
        let mut artifacts = Artifacts::default();
        for function in mir_functions {
            for stmt in &function.items {
                artifacts.process_statement(stmt);
            }
        }
        artifacts
    }

    fn process_statement(&mut self, stmt: &MirStmt) {
        match stmt {
            MirStmt::StrDef { name, literal } => {
                self.add_string_literal(name, &literal.value);
            }
            MirStmt::Call(call) => {
                self.externs.insert(call.name.clone());
            }
            _ => {}
        }
    }

    pub fn string_literals(&self) -> &[(String, String)] {
        &self.string_literals
    }

    pub fn add_string_literal(&mut self, label: &str, value: &str) {
        if self
            .string_literals
            .iter()
            .any(|(existing_label, _)| existing_label == label)
        {
            return;
        }
        self.string_literals
            .push((label.to_string(), value.to_string()));
    }
}

pub fn write_preamble<W: Write>(out: &mut W) -> Result<(), Error> {
    writeln!(out, "bits 64")?;
    writeln!(out, "default rel")?;
    writeln!(out, "section .text")?;
    Ok(())
}

fn emit_builtin_function<W: Write>(
    mir: &MirFunction,
    artifacts: &mut Artifacts,
    out: &mut W,
) -> Result<bool, Error> {
    if mir.items.is_empty() {
        match mir.sig.name.as_str() {
            "itoa" => {
                emit_builtin_itoa(artifacts, out)?;
                return Ok(true);
            }
            _ => {}
        }
    }
    Ok(false)
}

pub fn function<W: Write>(
    mir: MirFunction,
    artifacts: &mut Artifacts,
    out: &mut W,
) -> Result<(), Error> {
    if emit_builtin_function(&mir, artifacts, out)? {
        return Ok(());
    }
    let frame = FrameLayout::build(&mir)?;
    let mut emitter = FunctionEmitter::new(mir.clone(), out, frame);
    emitter.emit_function()?;
    Ok(())
}

pub fn emit_release_helper<W: Write>(out: &mut W) -> Result<(), Error> {
    writeln!(out, "internal_release_env:")?;
    writeln!(out, "    push rbp ; prologue: save executor frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
    writeln!(out, "    push rbx ; preserve continuation-saved registers")?;
    writeln!(out, "    push r12")?;
    writeln!(out, "    push r13")?;
    writeln!(out, "    push r14")?;
    writeln!(out, "    push r15")?;
    writeln!(out, "    mov r12, rdi ; capture env_end pointer")?;
    writeln!(out, "    test r12, r12 ; skip null releases")?;
    writeln!(out, "    je internal_release_env_done")?;
    writeln!(
        out,
        "    mov rcx, [r12+{}] ; load env size metadata",
        ENV_METADATA_ENV_SIZE_OFFSET
    )?;
    writeln!(
        out,
        "    mov r15, [r12+{}] ; load heap size metadata",
        ENV_METADATA_HEAP_SIZE_OFFSET
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

pub fn emit_deep_copy_helper<W: Write>(out: &mut W) -> Result<(), Error> {
    writeln!(out, "internal_deep_copy_env:")?;
    writeln!(out, "    push rbp ; prologue: save executor frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
    writeln!(out, "    push rbx ; preserve continuation-saved registers")?;
    writeln!(out, "    push r12")?;
    writeln!(out, "    push r13")?;
    writeln!(out, "    push r14")?;
    writeln!(out, "    push r15")?;
    writeln!(out, "    mov r12, rdi ; capture source env_end pointer")?;
    writeln!(out, "    test r12, r12 ; skip null copies")?;
    writeln!(out, "    je internal_deep_copy_env_null_return")?;
    writeln!(
        out,
        "    mov rcx, [r12+{}] ; load source env size metadata",
        ENV_METADATA_ENV_SIZE_OFFSET
    )?;
    writeln!(
        out,
        "    mov r15, [r12+{}] ; load heap size metadata",
        ENV_METADATA_HEAP_SIZE_OFFSET
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
        "    mov rax, [rbx+{}] ; load code pointer from source env",
        ENV_METADATA_UNWRAPPER_OFFSET
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
    emit_memcpy_helper(out)?;
    Ok(())
}

fn emit_memcpy_helper<W: Write>(out: &mut W) -> Result<(), Error> {
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

pub fn emit_externs<W: Write>(externs: &HashSet<String>, out: &mut W) -> Result<(), Error> {
    if externs.is_empty() {
        return Ok(());
    }
    let mut names: Vec<&String> = externs.iter().collect();
    names.sort();
    for name in names {
        writeln!(out, "extern {}", name)?;
    }
    Ok(())
}

pub fn emit_data<W: Write>(string_literals: &[(String, String)], out: &mut W) -> Result<(), Error> {
    if string_literals.is_empty() {
        return Ok(());
    }
    writeln!(out, "section .rodata")?;
    for (label, literal) in string_literals {
        writeln!(out, "{}:", label)?;
        let escaped = escape_literal_for_rodata(literal);
        writeln!(out, "    db {}, 0", escaped)?;
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct Binding {
    offset: i32,
    kind: ValueKind,
    continuation_params: Vec<SigKind>,
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
    fn build(mir: &MirFunction) -> Result<Self, Error> {
        let mut layout = Self {
            bindings: HashMap::new(),
            stack_size: 0,
            next_offset: 0,
        };
        for param in &mir.sig.params {
            let continuation_params = mir::continuation_params_for_type(&param.ty);
            layout.allocate_param(param, &continuation_params)?;
        }
        for stmt in &mir.items {
            if let Some((name, span)) = mir_statement_binding_info(stmt) {
                layout.allocate_word(name, span)?;
            }
        }
        layout.stack_size = align_to(layout.next_offset as usize, 16) as i32;
        Ok(layout)
    }

    fn allocate_param(
        &mut self,
        param: &SigItem,
        continuation_params: &[SigKind],
    ) -> Result<(), Error> {
        let name = &param.name;
        let ty = &param.ty;
        let kind = resolved_type_kind(&ty);
        if kind == ValueKind::Variadic {
            return Ok(());
        }
        let continuation_params = if kind == ValueKind::Closure {
            continuation_params.to_vec()
        } else {
            Vec::new()
        };
        let env_size = if kind == ValueKind::Closure {
            env_size_bytes_from_kinds(&continuation_params)
        } else {
            0
        };
        self.allocate_binding(name, param.span, kind, continuation_params, env_size)
    }

    fn allocate_word(&mut self, name: &str, span: Span) -> Result<(), Error> {
        self.allocate_binding(name, span, ValueKind::Word, Vec::new(), 0)
    }

    fn allocate_binding(
        &mut self,
        name: &str,
        span: Span,
        kind: ValueKind,
        continuation_params: Vec<SigKind>,
        env_size: usize,
    ) -> Result<(), Error> {
        if self.bindings.contains_key(name) {
            return Err(Error::new(
                Code::Codegen,
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

fn mir_statement_binding_info<'a>(stmt: &'a MirStmt) -> Option<(&'a str, Span)> {
    match stmt {
        MirStmt::EnvBase(base) => Some((base.name.as_str(), base.span)),
        MirStmt::EnvField(field) => Some((field.result.as_str(), field.span)),
        MirStmt::StrDef { name, literal } => Some((name.as_str(), literal.span)),
        MirStmt::IntDef { name, literal } => Some((name.as_str(), literal.span)),
        MirStmt::Closure(s) => Some((&s.name, s.span)),
        MirStmt::Exec(..) => None,
        MirStmt::Release(..) => None,
        MirStmt::DeepCopy(copy) => Some((copy.copy.as_str(), copy.span)),
        MirStmt::Op(instr) => instr
            .outputs
            .first()
            .map(|name| (name.as_str(), instr.span)),
        MirStmt::SysCall(syscall) => syscall
            .outputs
            .first()
            .map(|name| (name.as_str(), syscall.span)),
        MirStmt::Call(call) => {
            if call.result.is_empty() {
                None
            } else {
                Some((call.result.as_str(), call.span))
            }
        }
    }
}

#[derive(Clone, Debug)]
struct ClosureState {
    remaining: Vec<SigKind>,
    env_size: usize,
}

impl ClosureState {
    fn new(params: Vec<SigKind>, env_size: usize) -> Self {
        Self {
            remaining: params,
            env_size,
        }
    }

    fn remaining(&self) -> &[SigKind] {
        &self.remaining
    }

    fn env_size(&self) -> usize {
        self.env_size
    }

    fn after_applying(&self, count: usize) -> Self {
        let count = count.min(self.remaining.len());
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
    terminated: bool,
    write_loop_counter: usize,
    label_counter: usize,
}

impl<'a, W: Write> FunctionEmitter<'a, W> {
    fn new(mir: MirFunction, out: &'a mut W, frame: FrameLayout) -> Self {
        Self {
            mir,
            out,
            frame,
            terminated: false,
            write_loop_counter: 0,
            label_counter: 0,
        }
    }

    fn emit_function(&mut self) -> Result<(), Error> {
        self.write_header()?;
        self.write_prologue()?;
        self.store_params()?;
        self.emit_block()?;
        if !self.terminated {
            self.write_epilogue()?;
        }
        Ok(())
    }

    fn write_header(&mut self) -> Result<(), Error> {
        writeln!(self.out, "global {}", self.mir.sig.name)?;
        writeln!(self.out, "{}:", self.mir.sig.name)?;
        Ok(())
    }

    fn write_prologue(&mut self) -> Result<(), Error> {
        writeln!(self.out, "    push rbp ; save executor frame pointer")?;
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

    fn write_epilogue(&mut self) -> Result<(), Error> {
        writeln!(self.out, "    leave ; epilogue: restore rbp and rsp")?;
        writeln!(self.out, "    mov rax, {} ; exit syscall", SYSCALL_EXIT)?;
        writeln!(self.out, "    xor rdi, rdi")?;
        writeln!(self.out, "    syscall")?;
        Ok(())
    }

    fn store_params(&mut self) -> Result<(), Error> {
        let mut slot = 0usize;
        let mut spilled = false;
        let mut stack_offset_bytes = 0usize;
        for param in &self.mir.sig.params {
            let name = &param.name;
            let ty = &param.ty;
            let required = slots_for_type(&ty);
            let kind = resolved_type_kind(&ty);
            if kind == ValueKind::Variadic {
                slot += required;
                continue;
            }
            let binding = self
                .frame
                .binding(name)
                .ok_or_else(|| Error::new(Code::Codegen, "missing binding", param.span))?
                .clone();
            match kind {
                ValueKind::Word => {
                    if !spilled && slot + 1 <= ARG_REGS.len() {
                        let reg = ARG_REGS[slot];
                        writeln!(
                            self.out,
                            "    mov [rbp-{}], {} ; store scalar arg in frame",
                            binding.slot_addr(0),
                            reg
                        )?;
                        slot += 1;
                    } else {
                        spilled = true;
                        let addr = 8 + stack_offset_bytes;
                        writeln!(
                            self.out,
                            "    mov rax, [rbp+{}] ; load spilled scalar arg",
                            addr
                        )?;
                        writeln!(
                            self.out,
                            "    mov [rbp-{}], rax ; store spilled scalar arg",
                            binding.slot_addr(0)
                        )?;
                        stack_offset_bytes += bytes_for_type(&ty);
                    }
                }
                ValueKind::Closure => {
                    if !spilled && slot + 1 < ARG_REGS.len() {
                        let reg_env = ARG_REGS[slot + 1];
                        writeln!(
                            self.out,
                            "    mov [rbp-{}], {} ; save closure env_end pointer",
                            binding.slot_addr(0),
                            reg_env
                        )?;
                        slot += 2;
                    } else {
                        spilled = true;
                        let base = 8 + stack_offset_bytes;
                        writeln!(
                            self.out,
                            "    mov rax, [rbp+{}] ; load spilled closure code",
                            base
                        )?;
                        writeln!(
                            self.out,
                            "    mov rdx, [rbp+{}] ; load spilled closure env",
                            base + 8
                        )?;
                        writeln!(
                            self.out,
                            "    mov [rbp-{}], rdx ; save spilled closure env_end pointer",
                            binding.slot_addr(0)
                        )?;
                        stack_offset_bytes += bytes_for_type(&ty);
                    }
                }
                ValueKind::Variadic => unreachable!(),
            }
        }
        Ok(())
    }

    fn emit_block(&mut self) -> Result<(), Error> {
        let statements = self.mir.items.clone();
        for stmt in statements {
            self.emit_statement(&stmt)?;
            if self.terminated {
                break;
            }
        }
        Ok(())
    }

    fn emit_statement(&mut self, stmt: &MirStmt) -> Result<(), Error> {
        match stmt {
            MirStmt::StrDef { name, literal } => {
                let expr = Expr::String {
                    label: name.clone(),
                };
                let value = self.emit_expr(&expr)?;
                self.store_binding_value(&name, value, literal.span)?;
                return Ok(());
            }
            MirStmt::IntDef { name, literal } => {
                let expr = Expr::Int {
                    value: literal.value,
                };
                let value = self.emit_expr(&expr)?;
                self.store_binding_value(&name, value, literal.span)?;
                return Ok(());
            }
            MirStmt::EnvBase(base) => {
                self.emit_env_base(base)?;
                return Ok(());
            }
            MirStmt::EnvField(field) => {
                self.emit_env_field(field)?;
                return Ok(());
            }
            MirStmt::Closure(s) => {
                let name = s.name.clone();
                let value = self.emit_struct_value(s)?;
                self.store_binding_value(&name, value, s.span)?;
                return Ok(());
            }
            MirStmt::Exec(exec) => {
                let _ = self.emit_exec(exec)?; // TODO: discard unused closures to avoid leaking temporaries

                return Ok(());
            }
            MirStmt::Release(release) => {
                self.emit_release_env(release)?;
                return Ok(());
            }
            MirStmt::DeepCopy(copy) => {
                self.emit_deep_copy(copy)?;
                return Ok(());
            }
            MirStmt::Op(instr) => {
                self.emit_op(instr)?;
                return Ok(());
            }
            MirStmt::SysCall(syscall) => {
                self.emit_syscall(syscall)?;
                return Ok(());
            }
            MirStmt::Call(call) => {
                let value = self.emit_libcall(call)?;
                if !call.result.is_empty() {
                    self.store_binding_value(&call.result, value, call.span)?;
                }
                return Ok(());
            }
        }
    }

    fn emit_op(&mut self, instr: &MirInstruction) -> Result<(), Error> {
        match instr.kind {
            MirInstKind::Add | MirInstKind::Sub | MirInstKind::Mul | MirInstKind::Div => {
                self.emit_builtin_binary_op(instr)
            }
            MirInstKind::EqInt => self.emit_builtin_int_comparison(instr, IntComparison::Equal),
            MirInstKind::Lt => self.emit_builtin_int_comparison(instr, IntComparison::Less),
            MirInstKind::Gt => self.emit_builtin_int_comparison(instr, IntComparison::Greater),
            MirInstKind::EqStr => self.emit_builtin_string_equality(instr),
        }
    }

    fn emit_builtin_binary_op(&mut self, instr: &MirInstruction) -> Result<(), Error> {
        let (first_comment, second_comment, store_comment) = instr.operand_comments;
        writeln!(self.out, "    mov rax, rdi ; {}", first_comment)?;
        if matches!(instr.kind, MirInstKind::Div) {
            writeln!(self.out, "    cqo ; sign extend dividend")?;
            writeln!(self.out, "    idiv rsi ; {}", second_comment)?;
        } else {
            writeln!(
                self.out,
                "    {} rax, rsi ; {}",
                instr.opcode, second_comment
            )?;
        }
        writeln!(
            self.out,
            "    lea rbx, [rcx-8] ; reserve slot for result before metadata"
        )?;
        writeln!(self.out, "    mov [rbx], rax ; {}", store_comment)?;
        writeln!(self.out, "    mov rax, rdx ; continuation entry point")?;
        writeln!(
            self.out,
            "    mov rdi, rcx ; pass env_end pointer unchanged"
        )?;
        writeln!(self.out, "    jmp rax ; jump into continuation")?;
        writeln!(self.out)?;
        self.terminated = true;
        Ok(())
    }

    fn emit_builtin_int_comparison(
        &mut self,
        instr: &MirInstruction,
        comparison: IntComparison,
    ) -> Result<(), Error> {
        let (first_comment, second_comment, _) = instr.operand_comments;
        let true_label = self.new_label("true");
        let false_label = self.new_label("false");
        writeln!(self.out, "    cmp rdi, rsi ; {}", first_comment)?;
        writeln!(
            self.out,
            "    {} {} ; {}",
            comparison.false_jump(),
            false_label,
            second_comment
        )?;
        writeln!(self.out, "{}:", true_label)?;
        self.emit_jump_to_continuation("rdx", "rcx")?;
        writeln!(self.out, "{}:", false_label)?;
        self.emit_jump_to_continuation("r8", "r9")?;
        writeln!(self.out)?;
        Ok(())
    }

    fn emit_builtin_string_equality(&mut self, instr: &MirInstruction) -> Result<(), Error> {
        let (first_comment, second_comment, _) = instr.operand_comments;
        let loop_label = self.new_label("eqs_loop");
        let true_label = self.new_label("eqs_true");
        let false_label = self.new_label("eqs_false");
        writeln!(self.out, "    mov r10, rdi ; {}", first_comment)?;
        writeln!(self.out, "    mov r11, rsi ; {}", second_comment)?;
        writeln!(self.out, "{}:", loop_label)?;
        writeln!(self.out, "    mov al, byte [r10]")?;
        writeln!(self.out, "    mov dl, byte [r11]")?;
        writeln!(self.out, "    cmp al, dl")?;
        writeln!(self.out, "    jne {} ; bytes differ", false_label)?;
        writeln!(self.out, "    test al, al")?;
        writeln!(self.out, "    je {}", true_label)?;
        writeln!(self.out, "    inc r10")?;
        writeln!(self.out, "    inc r11")?;
        writeln!(self.out, "    jmp {}", loop_label)?;
        writeln!(self.out, "{}:", true_label)?;
        self.emit_jump_to_continuation("rdx", "rcx")?;
        writeln!(self.out, "{}:", false_label)?;
        self.emit_jump_to_continuation("r8", "r9")?;
        writeln!(self.out)?;
        Ok(())
    }

    fn emit_syscall(&mut self, syscall: &MirSysCall) -> Result<(), Error> {
        match syscall.kind {
            MirSysCallKind::Exit => self.emit_exit_syscall(syscall),
        }
    }

    fn emit_exit_syscall(&mut self, syscall: &MirSysCall) -> Result<(), Error> {
        let (first_comment, _, exit_comment) = syscall.operand_comments;
        writeln!(self.out, "    ; {}", first_comment)?;
        self.emit_exit_syscall_sequence(exit_comment)
    }

    fn emit_exit_syscall_sequence(&mut self, exit_comment: &str) -> Result<(), Error> {
        writeln!(self.out, "    leave ; unwind before exiting")?;
        writeln!(self.out, "    mov rax, {} ; exit syscall", SYSCALL_EXIT)?;
        writeln!(self.out, "    syscall ; {}", exit_comment)?;
        self.terminated = true;
        Ok(())
    }

    fn emit_jump_to_continuation(&mut self, code_reg: &str, env_reg: &str) -> Result<(), Error> {
        writeln!(
            self.out,
            "    mov rax, {} ; continuation entry point",
            code_reg
        )?;
        writeln!(
            self.out,
            "    mov rdi, {} ; pass env_end pointer to continuation",
            env_reg
        )?;
        writeln!(self.out, "    leave")?;
        writeln!(self.out, "    jmp rax")?;
        self.terminated = true;
        Ok(())
    }

    fn emit_env_base(&mut self, base: &MirEnvBase) -> Result<(), Error> {
        let expr = Expr::Ident {
            name: base.env_end.clone(),
            span: base.span,
        };
        let value = self.emit_expr(&expr)?;
        if let ExprValue::Word = value {
        } else {
            return Err(Error::new(
                Code::Codegen,
                format!("env_end binding '{}' is not a pointer", base.env_end),
                base.span,
            ));
        }
        let env_size_bytes = base.size * WORD_SIZE;
        if env_size_bytes > 0 {
            writeln!(
                self.out,
                "    sub rax, {} ; compute env base pointer",
                env_size_bytes
            )?;
        }
        self.store_binding_value(&base.name, ExprValue::Word, base.span)?;
        Ok(())
    }

    fn emit_env_field(&mut self, field: &MirEnvField) -> Result<(), Error> {
        let expr = Expr::Ident {
            name: field.env_end.clone(),
            span: field.span,
        };
        let value = self.emit_expr(&expr)?;
        if let ExprValue::Word = value {
        } else {
            return Err(Error::new(
                Code::Codegen,
                format!("env_end binding '{}' is not a pointer", field.env_end),
                field.span,
            ));
        }
        let offset_bytes = field.offset_from_end * WORD_SIZE;
        match resolved_type_kind(&field.ty) {
            ValueKind::Word => {
                writeln!(
                    self.out,
                    "    mov rax, [rax-{}] ; load scalar env field",
                    offset_bytes
                )?;
                let expr_value = ExprValue::Word;
                self.store_binding_value(&field.result, expr_value, field.span)?;
            }
            ValueKind::Closure => {
                writeln!(
                    self.out,
                    "    mov r10, rax ; env_end pointer for closure field"
                )?;
                writeln!(
                    self.out,
                    "    mov rdx, [r10-{}] ; load closure env_end pointer",
                    offset_bytes
                )?;
                writeln!(
                    self.out,
                    "    mov rax, [rdx+{}] ; load closure unwrapper entry point",
                    ENV_METADATA_UNWRAPPER_OFFSET
                )?;
                let params = field.continuation_params.clone();
                let env_size = env_size_bytes_from_kinds(&params);
                let state = ClosureState::new(params.clone(), env_size);
                let expr_value = ExprValue::Closure(state);
                self.store_binding_value(&field.result, expr_value, field.span)?;
            }
            ValueKind::Variadic => unreachable!("variadic env entries are not stored"),
        }
        Ok(())
    }

    fn emit_release_env(&mut self, release: &Release) -> Result<(), Error> {
        let span = Span::unknown();
        let binding = self.frame.binding(&release.name).cloned().ok_or_else(|| {
            Error::new(
                Code::Codegen,
                format!("unknown binding '{}'", release.name),
                span,
            )
        })?;
        if binding.kind != ValueKind::Closure {
            return Err(Error::new(
                Code::Codegen,
                format!("cannot release non-closure binding '{}'", release.name),
                span,
            ));
        }
        let env_offset = binding.slot_addr(0);
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

    fn emit_deep_copy(&mut self, copy: &DeepCopy) -> Result<(), Error> {
        let binding = self.frame.binding(&copy.original).cloned().ok_or_else(|| {
            Error::new(
                Code::Codegen,
                format!("unknown binding '{}'", copy.original),
                copy.span,
            )
        })?;
        if binding.kind != ValueKind::Closure {
            return Err(Error::new(
                Code::Codegen,
                format!("cannot deep copy non-closure binding '{}'", copy.original),
                copy.span,
            ));
        }

        let env_offset = binding.slot_addr(0);

        writeln!(
            self.out,
            "    mov rdi, [rbp-{}] ; load original closure env_end pointer",
            env_offset
        )?;
        writeln!(
            self.out,
            "    call internal_deep_copy_env ; deep copy closure environment"
        )?;
        writeln!(
            self.out,
            "    mov rax, rsi ; keep unwrapper pointer for new closure"
        )?;
        writeln!(
            self.out,
            "    mov rdx, rdi ; move new env_end pointer to rdx"
        )?;

        // Store the new closure with the original code pointer and new env
        let copy_state = ClosureState::new(binding.continuation_params.clone(), binding.env_size);
        let copy_value = ExprValue::Closure(copy_state);
        self.store_binding_value(&copy.copy, copy_value, copy.span)?;

        Ok(())
    }

    fn emit_exec_value(&mut self, exec: &MirExec) -> Result<ExprValue, Error> {
        match &exec.target {
            MirExecTarget::Function(sig) => {
                if let Some(mir::MirBuiltin::Call(kind)) = sig.builtin {
                    return self.emit_builtin_exec(kind, &exec.args, exec.span);
                }
                let param_kinds = sig.param_kinds();
                let env_kinds = &param_kinds;
                self.emit_named_function_closure(&sig.name, env_kinds)?;
                let env_size = env_size_bytes_from_kinds(env_kinds);
                let state = ClosureState::new(env_kinds.to_vec(), env_size);
                return self.apply_closure(state, &exec.args, exec.span, false);
            }
            MirExecTarget::Closure { name } => {
                return self.apply_closure_binding(name, &exec.args, exec.span, false);
            }
        }
    }

    fn emit_struct_value(&mut self, structure: &MirClosure) -> Result<ExprValue, Error> {
        if let MirExecTarget::Closure { name } = &structure.target {
            return self.apply_closure_binding(name, &structure.args, structure.span, false);
        }
        let exec = MirExec {
            target: structure.target.clone(),
            args: structure.args.clone(),
            span: structure.span,
        };
        self.emit_exec_value(&exec) // TODO: Should not emit exec here
    }

    fn store_binding_value(
        &mut self,
        name: &str,
        value: ExprValue,
        span: Span,
    ) -> Result<(), Error> {
        let binding = self.frame.binding_mut(name).ok_or_else(|| {
            Error::new(Code::Codegen, format!("unknown binding '{}'", name), span)
        })?;
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
                    "    mov [rbp-{}], rdx ; update closure env_end pointer",
                    binding.slot_addr(0)
                )?;
                binding.kind = ValueKind::Closure;
                binding.continuation_params = state.remaining.clone();
                binding.env_size = state.env_size();
            }
        }
        Ok(())
    }

    fn emit_expr(&mut self, expr: &Expr) -> Result<ExprValue, Error> {
        match expr {
            Expr::Int { value, .. } => {
                writeln!(self.out, "    mov rax, {} ; load literal integer", value)?;
                Ok(ExprValue::Word)
            }
            Expr::String { label, .. } => {
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
                                "    mov rdx, [rbp-{}] ; load closure env_end pointer",
                                binding.slot_addr(0)
                            )?;
                            writeln!(
                                self.out,
                                "    mov rax, [rdx+{}] ; load closure unwrapper entry point",
                                ENV_METADATA_UNWRAPPER_OFFSET
                            )?;
                            Ok(ExprValue::Closure(ClosureState::new(
                                binding.continuation_params.clone(),
                                binding.env_size,
                            )))
                        }
                        ValueKind::Variadic => unreachable!("variadic bindings are not stored"),
                    }
                } else {
                    eprintln!(
                        "{}: missing binding `{}`; available: {:?}",
                        self.mir.sig.name,
                        name,
                        self.frame.bindings.keys().cloned().collect::<Vec<String>>()
                    );
                    Err(Error::new(
                        Code::Codegen,
                        format!("compiler bug: unresolved identifier '{}'", name),
                        *span,
                    ))
                }
            }
        }
    }

    fn emit_mir_arg_value(&mut self, arg: &MirArg, span: Span) -> Result<ExprValue, Error> {
        let expr = Expr::Ident {
            name: arg.name.clone(),
            span,
        };
        self.emit_expr(&expr)
    }

    fn emit_exec(&mut self, exec: &MirExec) -> Result<ExprValue, Error> {
        match &exec.target {
            MirExecTarget::Function(sig) => {
                if let Some(mir::MirBuiltin::Call(kind)) = sig.builtin {
                    self.emit_builtin_exec(kind, &exec.args, exec.span)
                } else {
                    self.emit_named_exec(sig, &exec.args, exec.span)
                }
            }
            MirExecTarget::Closure { name } => {
                self.apply_closure_binding(name, &exec.args, exec.span, true)
            }
        }
    }

    fn emit_builtin_exec(
        &mut self,
        kind: MirCallKind,
        args: &[MirArg],
        span: Span,
    ) -> Result<ExprValue, Error> {
        if let Some(result) = self.emit_builtin_syscall(kind, args, span)? {
            return Ok(result);
        }
        match kind {
            MirCallKind::Printf => self.emit_printf_wrapper_exec(args, span),
            MirCallKind::Puts => self.emit_puts_wrapper_exec(args, span),
            _ => Err(Error::new(
                Code::Codegen,
                format!("unsupported builtin exec '{:?}'", kind),
                span,
            )),
        }
    }

    fn emit_builtin_syscall(
        &mut self,
        kind: MirCallKind,
        args: &[MirArg],
        span: Span,
    ) -> Result<Option<ExprValue>, Error> {
        match kind {
            MirCallKind::Write => Ok(Some(self.emit_write_wrapper_exec(args, span)?)),
            _ => Ok(None),
        }
    }

    fn apply_closure_binding(
        &mut self,
        name: &str,
        args: &[MirArg],
        span: Span,
        invoke_when_ready: bool,
    ) -> Result<ExprValue, Error> {
        let binding = self.frame.binding(name).ok_or_else(|| {
            Error::new(Code::Codegen, format!("unknown binding '{}'", name), span)
        })?;
        if binding.kind != ValueKind::Closure {
            return Err(Error::new(
                Code::Codegen,
                format!("'{}' is not callable", name),
                span,
            ));
        }
        let env_slot = binding.slot_addr(0);
        writeln!(
            self.out,
            "    mov rdx, [rbp-{}] ; load closure env_end for exec",
            env_slot
        )?;
        writeln!(
            self.out,
            "    mov rax, [rdx+{}] ; load closure unwrapper entry point",
            ENV_METADATA_UNWRAPPER_OFFSET
        )?;
        let state = ClosureState::new(binding.continuation_params.clone(), binding.env_size);
        self.apply_closure(state, args, span, invoke_when_ready)
    }

    fn emit_libcall(&mut self, call: &MirCall) -> Result<ExprValue, Error> {
        match call.name.as_str() {
            "printf" => {
                let call_args = &call.args;
                if call_args.is_empty() {
                    return Err(Error::new(
                        Code::Codegen,
                        "printf requires a format string before the continuation",
                        call.span,
                    ));
                }

                let params = &call.arg_kinds;

                self.prepare_mir_args(call_args, params, call.span)?;
                let arg_split = self.move_args_to_registers(params)?;
                self.call_variadic_libc("printf")?;
                if arg_split.stack_bytes > 0 {
                    writeln!(
                        self.out,
                        "    add rsp, {} ; pop stack args after printf",
                        arg_split.stack_bytes
                    )?;
                }

                Ok(ExprValue::Word)
            }
            "sprintf" => {
                let call_args = &call.args;
                if call_args.is_empty() {
                    return Err(Error::new(
                        Code::Codegen,
                        "sprintf requires a format string before the continuation",
                        call.span,
                    ));
                }

                let params = &call.arg_kinds;

                self.prepare_mir_args(call_args, params, call.span)?;

                self.emit_mmap(FMT_BUFFER_SIZE)?;
                writeln!(self.out, "    mov rbx, rax ; keep sprintf buffer pointer")?;
                let arg_split = self.move_args_to_registers(params)?;
                if arg_split.reg_slots == ARG_REGS.len() {
                    return Err(Error::new(
                        Code::Codegen,
                        "sprintf requires at least one register slot for the buffer pointer",
                        call.span,
                    ));
                }
                for i in (0..arg_split.reg_slots).rev() {
                    let dest = ARG_REGS[i + 1];
                    let src = ARG_REGS[i];
                    writeln!(
                        self.out,
                        "    mov {}, {} ; shift sprintf args for buffer insertion",
                        dest, src
                    )?;
                }

                writeln!(
                    self.out,
                    "    mov rdi, rbx ; destination buffer for sprintf"
                )?;
                self.call_variadic_libc("sprintf")?;
                writeln!(
                    self.out,
                    "    mov rax, rbx ; return formatted string pointer"
                )?;
                if arg_split.stack_bytes > 0 {
                    writeln!(
                        self.out,
                        "    add rsp, {} ; pop stack args after sprintf",
                        arg_split.stack_bytes
                    )?;
                }

                Ok(ExprValue::Word)
            }
            "write" => {
                let call_args = &call.args;
                if call_args.is_empty() {
                    return Err(Error::new(
                        Code::Codegen,
                        "write requires a buffer before the continuation",
                        call.span,
                    ));
                }

                let params = &call.arg_kinds;

                self.prepare_mir_args(call_args, params, call.span)?;
                let arg_split = self.move_args_to_registers(params)?;

                writeln!(self.out, "    mov r8, rdi ; keep string pointer")?;
                writeln!(self.out, "    xor rcx, rcx ; reset length counter")?;
                let (loop_label, done_label) = self.next_write_loop_labels();
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

                writeln!(self.out, "    mov rdx, rcx ; length to write")?;
                writeln!(self.out, "    mov rsi, r8 ; buffer start")?;
                writeln!(self.out, "    mov rdi, 1 ; stdout fd")?;

                writeln!(self.out, "    call write ; invoke libc write")?;
                if arg_split.stack_bytes > 0 {
                    writeln!(
                        self.out,
                        "    add rsp, {} ; pop stack args after write",
                        arg_split.stack_bytes
                    )?;
                }

                Ok(ExprValue::Word)
            }
            "puts" => {
                let call_args = &call.args;
                if call_args.is_empty() {
                    return Err(Error::new(
                        Code::Codegen,
                        "puts requires a buffer before the continuation",
                        call.span,
                    ));
                }

                let params = &call.arg_kinds;

                self.prepare_mir_args(call_args, params, call.span)?;
                let arg_split = self.move_args_to_registers(params)?;

                writeln!(self.out, "    call puts ; invoke libc puts")?;
                if arg_split.stack_bytes > 0 {
                    writeln!(
                        self.out,
                        "    add rsp, {} ; pop stack args after puts",
                        arg_split.stack_bytes
                    )?;
                }

                Ok(ExprValue::Word)
            }
            _ => Err(Error::new(
                Code::Codegen,
                format!("unsupported libcall '{}'", call.name),
                call.span,
            )),
        }
    }

    fn call_variadic_libc(&mut self, name: &str) -> Result<(), Error> {
        writeln!(self.out, "    push rbp ; helper prologue")?;
        writeln!(self.out, "    mov rbp, rsp")?;
        writeln!(self.out, "    push r12")?;
        writeln!(
            self.out,
            "    mov rax, rsp ; align stack for variadic {name} call"
        )?;
        writeln!(self.out, "    and rax, 15")?;
        writeln!(self.out, "    mov r12, rax")?;
        writeln!(self.out, "    sub rsp, r12")?;
        writeln!(self.out, "    call {} ; invoke libc {name}", name)?;
        writeln!(self.out, "    add rsp, r12")?;
        writeln!(self.out, "    pop r12")?;
        writeln!(self.out, "    pop rbp")?;
        Ok(())
    }

    fn emit_printf_wrapper_exec(
        &mut self,
        args: &[MirArg],
        span: Span,
    ) -> Result<ExprValue, Error> {
        if args.len() < 2 {
            return Err(Error::new(
                Code::Codegen,
                "printf requires a format string and a continuation",
                span,
            ));
        }

        let continuation_arg = &args[args.len() - 1];
        let continuation_value = self.emit_mir_arg_value(continuation_arg, span)?;
        if !matches!(continuation_value, ExprValue::Closure(_)) {
            return Err(Error::new(
                Code::Codegen,
                format!(
                    "last argument to printf must be a continuation, got {:?}",
                    continuation_value
                ),
                span,
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
            return Err(Error::new(
                Code::Codegen,
                "printf requires a format string before the continuation",
                span,
            ));
        }

        let mut params = Vec::with_capacity(call_args.len());
        params.push(SigKind::Str);
        for _ in 1..call_args.len() {
            params.push(SigKind::Int);
        }

        self.prepare_mir_args(call_args, &params, span)?;
        let arg_split = self.move_args_to_registers(&params)?;

        writeln!(
            self.out,
            "    pop rdx ; restore continuation env_end pointer"
        )?;
        writeln!(self.out, "    pop rax ; restore continuation code pointer")?;
        if arg_split.stack_bytes > 0 {
            writeln!(
                self.out,
                "    add rsp, {} ; pop stack args after printf helper",
                arg_split.stack_bytes
            )?;
        }
        writeln!(
            self.out,
            "    mov rdi, rdx ; pass env_end pointer to continuation"
        )?;
        writeln!(self.out, "    leave ; unwind before jumping")?;
        writeln!(self.out, "    jmp rax ; jump into continuation")?;
        self.terminated = true;

        Ok(ExprValue::Word)
    }

    fn emit_write_wrapper_exec(&mut self, args: &[MirArg], span: Span) -> Result<ExprValue, Error> {
        if args.len() < 2 {
            return Err(Error::new(
                Code::Codegen,
                "write requires a string and a continuation",
                span,
            ));
        }

        let call_args = &args[..args.len() - 1];
        if call_args.len() != 1 {
            return Err(Error::new(
                Code::Codegen,
                "write requires a string before the continuation",
                span,
            ));
        }

        let continuation_arg = &args[args.len() - 1];
        let continuation_value = self.emit_mir_arg_value(continuation_arg, span)?;
        if !matches!(continuation_value, ExprValue::Closure(_)) {
            return Err(Error::new(
                Code::Codegen,
                "last argument to write must be a continuation",
                span,
            ));
        };

        writeln!(
            self.out,
            "    push rax ; preserve continuation code pointer"
        )?;
        writeln!(
            self.out,
            "    push rdx ; preserve continuation env_end pointer"
        )?;

        let params = vec![SigKind::Str];
        self.prepare_mir_args(call_args, &params, span)?;
        let arg_split = self.move_args_to_registers(&params)?;

        writeln!(self.out, "    mov r8, rdi ; keep string pointer")?;
        writeln!(self.out, "    xor rcx, rcx ; reset length counter")?;
        let (loop_label, done_label) = self.next_write_loop_labels();
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
        if arg_split.stack_bytes > 0 {
            writeln!(
                self.out,
                "    add rsp, {} ; pop stack args after write",
                arg_split.stack_bytes
            )?;
        }
        writeln!(self.out, "    leave ; unwind before jumping")?;
        writeln!(self.out, "    jmp rax ; jump into continuation")?;
        self.terminated = true;

        Ok(ExprValue::Word)
    }

    fn emit_puts_wrapper_exec(&mut self, args: &[MirArg], span: Span) -> Result<ExprValue, Error> {
        if args.len() < 2 {
            return Err(Error::new(
                Code::Codegen,
                "puts requires a string and a continuation",
                span,
            ));
        }

        let param_types = vec![SigKind::Str];
        self.emit_ffi_bridge_exec("puts", args, &param_types, span, false, |_| Ok(()))
    }

    /// Generic FFI bridge that handles the System V AMD64 C ABI calling convention.
    ///
    /// Takes arguments, passes them via the ABI (rdi, rsi, rdx, rcx, r8, r9),
    /// calls the external function, and jumps to continuation.
    ///
    /// The `return_handler` closure allows custom handling of the return value (rax).
    /// For simple bridges that ignore return values, it can just do nothing.
    fn emit_ffi_bridge_exec<F>(
        &mut self,
        ffi_name: &str,
        args: &[MirArg],
        param_types: &[SigKind],
        span: Span,
        is_variadic: bool,
        mut return_handler: F,
    ) -> Result<ExprValue, Error>
    where
        F: FnMut(&mut Self) -> Result<(), Error>,
    {
        // Last argument must be a continuation
        if args.is_empty() {
            return Err(Error::new(
                Code::Codegen,
                format!("{} requires a continuation as last argument", ffi_name),
                span,
            ));
        }

        let continuation_arg = &args[args.len() - 1];
        let continuation_value = self.emit_mir_arg_value(continuation_arg, span)?;
        let _closure_state = match continuation_value {
            ExprValue::Closure(state) => state,
            _ => {
                return Err(Error::new(
                    Code::Codegen,
                    format!("last argument to {} must be a continuation", ffi_name),
                    span,
                ));
            }
        };

        // Preserve continuation (code pointer in rax, env_end in rdx)
        writeln!(
            self.out,
            "    push rax ; preserve continuation code pointer"
        )?;
        writeln!(
            self.out,
            "    push rdx ; preserve continuation env_end pointer"
        )?;

        // Prepare call arguments (all except the continuation)
        let call_args = &args[..args.len() - 1];
        if call_args.len() != param_types.len() {
            return Err(Error::new(
                Code::Codegen,
                format!(
                    "{} requires {} arguments plus continuation, got {}",
                    ffi_name,
                    param_types.len(),
                    call_args.len()
                ),
                span,
            ));
        }

        self.prepare_mir_args(call_args, param_types, span)?;
        let arg_split = self.move_args_to_registers(param_types)?;

        // Call the external function

        // Stack alignment: for variadic functions, the stack must be 16-byte aligned before call
        // For non-variadic, we just call directly (the call op will misalign by 8)
        if is_variadic {
            writeln!(
                self.out,
                "    sub rsp, 8 ; align stack to 16-byte boundary for variadic call"
            )?;
        }

        writeln!(self.out, "    call {} ; invoke external function", ffi_name)?;

        if is_variadic {
            writeln!(self.out, "    add rsp, 8 ; restore stack")?;
        }

        if arg_split.stack_bytes > 0 {
            writeln!(
                self.out,
                "    add rsp, {} ; pop stack args after {}",
                arg_split.stack_bytes, ffi_name
            )?;
        }

        // Allow custom return value handling
        return_handler(self)?;

        // Restore continuation pointers
        writeln!(
            self.out,
            "    pop rdx ; restore continuation env_end pointer"
        )?;
        writeln!(self.out, "    pop rax ; restore continuation code pointer")?;

        // Jump to continuation
        writeln!(
            self.out,
            "    mov rdi, rdx ; pass env_end pointer to continuation"
        )?;
        writeln!(self.out, "    leave ; unwind before jumping")?;
        writeln!(self.out, "    jmp rax ; jump into continuation")?;
        self.terminated = true;

        Ok(ExprValue::Word)
    }

    fn emit_named_exec(
        &mut self,
        sig: &mir::FunctionSig,
        args: &[MirArg],
        span: Span,
    ) -> Result<ExprValue, Error> {
        let param_kinds = sig.param_kinds();
        let env_kinds = &param_kinds;
        if args.len() > sig.params.len() {
            return Err(Error::new(
                Code::Codegen,
                format!(
                    "function '{}' expected {} arguments but got {}",
                    sig.name,
                    sig.params.len(),
                    args.len()
                ),
                span,
            ));
        }

        if args.len() < sig.params.len() {
            self.emit_named_function_closure(&sig.name, env_kinds)?;
            let env_size = env_size_bytes_from_kinds(env_kinds);
            let state = ClosureState::new(env_kinds.to_vec(), env_size);
            return self.apply_closure(state, args, span, true);
        }

        self.prepare_mir_args(args, &param_kinds, span)?;
        let arg_split = self.move_args_to_registers(&param_kinds)?;
        if arg_split.stack_bytes > 0 {
            writeln!(self.out, "    sub rsp, 8 ; allocate slot for saved rbp")?;
            writeln!(self.out, "    mov rax, [rbp] ; capture parent rbp")?;
            writeln!(self.out, "    mov [rsp], rax ; stash parent rbp for leave")?;
            writeln!(self.out, "    mov rbp, rsp ; treat slot as current rbp")?;
        }
        writeln!(self.out, "    leave ; unwind before named jump")?;
        writeln!(
            self.out,
            "    jmp {} ; jump to fully applied function",
            sig.name
        )?;
        self.terminated = true;
        Ok(ExprValue::Word)
    }

    fn emit_named_function_closure(
        &mut self,
        name: &str,
        env_param_kinds: &[SigKind],
    ) -> Result<(), Error> {
        let pointer_offsets = env_pointer_offsets_from_kinds(env_param_kinds);
        let metadata_size = env_metadata_size(pointer_offsets.len());
        let env_size = env_size_bytes_from_kinds(env_param_kinds);
        let heap_size = env_size + metadata_size;
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
            "    mov qword [rdx+{}], {} ; env size metadata",
            ENV_METADATA_ENV_SIZE_OFFSET, env_size
        )?;
        writeln!(
            self.out,
            "    mov qword [rdx+{}], {} ; heap size metadata",
            ENV_METADATA_HEAP_SIZE_OFFSET, heap_size
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
        let unwrapper = mir::closure_unwrapper_label(name);
        writeln!(
            self.out,
            "    mov rax, {} ; load unwrapper entry point",
            unwrapper
        )?;
        writeln!(
            self.out,
            "    mov qword [rdx+{}], rax ; store unwrapper entry in metadata",
            ENV_METADATA_UNWRAPPER_OFFSET
        )?;
        Ok(())
    }

    fn emit_mmap(&mut self, size: usize) -> Result<(), Error> {
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

    fn prepare_mir_args(
        &mut self,
        args: &[MirArg],
        _params: &[SigKind],
        span: Span,
    ) -> Result<(), Error> {
        for arg in args.iter().rev() {
            let value = self.emit_mir_arg_value(arg, span)?;
            self.push_value(&value)?;
        }
        Ok(())
    }

    fn push_value(&mut self, value: &ExprValue) -> Result<(), Error> {
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

    fn move_args_to_registers(&mut self, params: &[SigKind]) -> Result<ArgSplit, Error> {
        let mut slot = 0usize;
        let mut spilled = false;
        let mut stack_bytes = 0usize;
        for ty in params {
            let required = slots_for_type(ty);
            match resolved_type_kind(ty) {
                ValueKind::Variadic => {
                    // Variadic kinds are handled separately and do not consume slots here.
                }
                kind => {
                    if !spilled && slot + required <= ARG_REGS.len() {
                        match kind {
                            ValueKind::Word => {
                                let reg = ARG_REGS[slot];
                                writeln!(
                                    self.out,
                                    "    pop {} ; restore scalar arg into register",
                                    reg
                                )?;
                            }
                            ValueKind::Closure => {
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
                            }
                            ValueKind::Variadic => unreachable!(),
                        }
                        slot += required;
                    } else {
                        spilled = true;
                        stack_bytes += bytes_for_type(ty);
                    }
                }
            }
        }
        Ok(ArgSplit {
            reg_slots: slot,
            stack_bytes,
        })
    }

    fn clone_closure_argument(&mut self) -> Result<(), Error> {
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
            "    mov r13, [rbx+{}] ; load env size metadata for clone",
            ENV_METADATA_ENV_SIZE_OFFSET
        )?;
        writeln!(
            self.out,
            "    mov r14, [rbx+{}] ; load heap size metadata for clone",
            ENV_METADATA_HEAP_SIZE_OFFSET
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
        args: &[MirArg],
        span: Span,
        invoke_when_ready: bool,
    ) -> Result<ExprValue, Error> {
        let remaining_len = state.remaining().len();
        let allows_extra_result =
            remaining_len == 0 && args.len() == 1 && args[0].name == "__result";
        if args.len() > remaining_len && !allows_extra_result {
            return Err(Error::new(
                Code::Codegen,
                "too many arguments for closure",
                span,
            ));
        }
        let saved_closure_state = !args.is_empty();
        if saved_closure_state {
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
        }

        let remaining = state.remaining();
        let suffix_sizes = remaining_suffix_sizes(remaining);
        let applied = args.len().min(state.remaining().len());
        let next_state = state.after_applying(applied);
        let needs_clone =
            !invoke_when_ready && !next_state.remaining().is_empty() && !args.is_empty();
        if needs_clone {
            writeln!(
                self.out,
                "    mov rbx, [rsp+8] ; original closure env_end pointer"
            )?;
            writeln!(
                self.out,
                "    mov r13, [rbx+{}] ; load env size metadata for clone",
                ENV_METADATA_ENV_SIZE_OFFSET
            )?;
            writeln!(
                self.out,
                "    mov r14, [rbx+{}] ; load heap size metadata for clone",
                ENV_METADATA_HEAP_SIZE_OFFSET
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
            self.emit_mir_arg_value(arg, span)?;
            match resolved_type_kind(ty) {
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
                    writeln!(
                        self.out,
                        "    mov [rbx], rdx ; store closure env_end for arg"
                    )?;
                }
                ValueKind::Variadic => {}
            }
        }

        let remaining = next_state;
        if saved_closure_state {
            writeln!(
                self.out,
                "    mov rax, [rsp] ; restore closure code pointer"
            )?;
            writeln!(
                self.out,
                "    mov rdx, [rsp+8] ; restore closure env_end pointer"
            )?;
            writeln!(self.out, "    add rsp, 24 ; pop temporary closure state")?;
        }

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

    fn next_write_loop_labels(&mut self) -> (String, String) {
        let idx = self.write_loop_counter;
        self.write_loop_counter += 1;
        let prefix = sanitize_function_name(&self.mir.sig.name);
        (
            format!("{}_write_strlen_loop_{}", prefix, idx),
            format!("{}_write_strlen_done_{}", prefix, idx),
        )
    }

    fn new_label(&mut self, suffix: &str) -> String {
        let idx = self.label_counter;
        self.label_counter += 1;
        format!(
            "{}_{}_{}",
            sanitize_function_name(&self.mir.sig.name),
            suffix,
            idx
        )
    }
}

enum IntComparison {
    Equal,
    Less,
    Greater,
}

impl IntComparison {
    fn false_jump(&self) -> &'static str {
        match self {
            IntComparison::Equal => "jne",
            IntComparison::Less => "jge",
            IntComparison::Greater => "jle",
        }
    }
}

fn align_to(value: usize, align: usize) -> usize {
    if value == 0 {
        return 0;
    }
    ((value + align - 1) / align) * align
}

fn emit_builtin_itoa<W: Write>(artifacts: &mut Artifacts, out: &mut W) -> Result<(), Error> {
    const ITOA_MIN_LABEL: &str = "itoa_min_value";
    const ITOA_MIN_VALUE: &str = "-9223372036854775808";
    artifacts.add_string_literal(ITOA_MIN_LABEL, ITOA_MIN_VALUE);
    writeln!(out, "global itoa")?;
    writeln!(out, "itoa:")?;
    writeln!(out, "    push rbp ; save executor frame pointer")?;
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
        ITOA_MIN_LABEL
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

fn env_size_bytes_from_kinds(params: &[SigKind]) -> usize {
    params.iter().map(bytes_for_type).sum()
}

fn remaining_suffix_sizes(params: &[SigKind]) -> Vec<usize> {
    let mut sizes = Vec::with_capacity(params.len());
    let mut acc = 0;
    for param in params.iter().rev() {
        acc += bytes_for_type(param);
        sizes.push(acc);
    }
    sizes.reverse();
    sizes
}

fn bytes_for_type(ty: &SigKind) -> usize {
    match resolved_type_kind(ty) {
        ValueKind::Word => WORD_SIZE,
        ValueKind::Closure => WORD_SIZE,
        ValueKind::Variadic => 0,
    }
}

fn slots_for_type(ty: &SigKind) -> usize {
    match resolved_type_kind(ty) {
        ValueKind::Word => 1,
        ValueKind::Closure => 2,
        ValueKind::Variadic => 0,
    }
}

fn env_pointer_offsets_from_kinds(params: &[SigKind]) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut current = 0usize;
    for ty in params {
        match resolved_type_kind(ty) {
            ValueKind::Word => current += 8,
            ValueKind::Closure => {
                offsets.push(current);
                current += WORD_SIZE;
            }
            ValueKind::Variadic => {}
        }
    }
    offsets
}

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
