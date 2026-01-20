use crate::compiler::air;
use crate::compiler::air::{
    AirAdd, AirArg, AirCallPtr, AirCallPtrTarget, AirDiv, AirField, AirFunction, AirJump,
    AirJumpArgs, AirJumpClosure, AirJumpEq, AirJumpGt, AirJumpLt, AirLabel, AirMul, AirNewClosure,
    AirOp, AirPin, AirReturn, AirStmt, AirSub, AirSysExit, AirValue, SigKind,
};
use crate::compiler::ast::Lit;
use crate::compiler::builtins;
use crate::compiler::builtins::AirRuntimeHelper;
use crate::compiler::error::{Code, Error};
use crate::compiler::runtime;
use crate::compiler::span::Span;
use std::collections::{HashMap, HashSet};
use std::io::Write;

const WORD_SIZE: usize = 8;
pub const ENV_METADATA_UNWRAPPER_OFFSET: usize = 0;
pub const ENV_METADATA_RELEASE_OFFSET: usize = WORD_SIZE;
pub const ENV_METADATA_DEEP_COPY_OFFSET: usize = WORD_SIZE * 2;
pub const ENV_METADATA_ENV_SIZE_OFFSET: usize = WORD_SIZE * 3;
pub const ENV_METADATA_HEAP_SIZE_OFFSET: usize = WORD_SIZE * 4;
pub const ENV_METADATA_NUM_REMAINING_OFFSET: usize = WORD_SIZE * 5;
pub const ENV_METADATA_SIZE: usize = WORD_SIZE * 6;
pub const CLOSURE_ENV_REG: &str = "r12";
pub const ARG_REGS: [&str; 6] = ["rdi", "rsi", "rdx", "rcx", "r8", "r9"];
pub const SYSCALL_MMAP: i32 = 9;
pub const SYSCALL_MUNMAP: i32 = 11;
pub const SYSCALL_EXIT: i32 = 60;
pub const PROT_READ: i32 = 1;
pub const PROT_WRITE: i32 = 2;
pub const MAP_PRIVATE: i32 = 2;
pub const MAP_ANONYMOUS: i32 = 32;
pub const FMT_BUFFER_SIZE: usize = 1024;

#[derive(Debug, Default)]
pub struct Artifacts {
    string_literals: Vec<(String, String)>,
    pub externs: HashSet<String>,
    builtins_used: HashSet<String>,
}

impl Artifacts {
    pub fn collect(air_functions: &[AirFunction]) -> Self {
        let mut artifacts = Artifacts::default();
        for function in air_functions {
            for stmt in &function.items {
                artifacts.process_statement(stmt);
            }
        }
        artifacts
    }

    fn process_statement(&mut self, stmt: &AirStmt) {
        self.collect_literals_in_stmt(stmt);
        match stmt {
            AirStmt::Op(AirOp::Printf(_))
            | AirStmt::Op(AirOp::Sprintf(_))
            | AirStmt::Op(AirOp::Write(_))
            | AirStmt::Op(AirOp::Puts(_)) => {
                let name = match stmt {
                    AirStmt::Op(AirOp::Printf(_)) => "printf",
                    AirStmt::Op(AirOp::Sprintf(_)) => "sprintf",
                    AirStmt::Op(AirOp::Write(_)) => "write",
                    AirStmt::Op(AirOp::Puts(_)) => "puts",
                    _ => "",
                };
                if !name.is_empty() {
                    self.externs.insert(name.to_string());
                }
            }
            AirStmt::Op(AirOp::SysExit(_)) => {
                // Call libc exit instead of raw syscall to ensure proper cleanup and flushing
                self.externs.insert("exit".to_string());
            }
            AirStmt::Op(AirOp::CallPtr(_)) => {
                self.externs
                    .insert(AirRuntimeHelper::ReleaseHeapPtr.name().to_string());
            }
            AirStmt::Op(AirOp::ReleaseHeap(_)) => {
                self.externs
                    .insert(AirRuntimeHelper::ReleaseHeapPtr.name().to_string());
            }
            AirStmt::Op(AirOp::CopyField(_)) => {
                self.externs
                    .insert(AirRuntimeHelper::DeepCopyHeapPtr.name().to_string());
            }
            _ => {}
        }
    }

    fn collect_literals_in_stmt(&mut self, stmt: &AirStmt) {
        match stmt {
            AirStmt::Op(op) => self.collect_literals_in_op(op),
            _ => {}
        }
    }

    fn collect_literals_in_args(&mut self, args: &[AirArg]) {
        for arg in args {
            if let Some(Lit::Str(value)) = &arg.literal {
                self.add_string_literal(&arg.name, value);
            }
        }
    }

    fn collect_literals_in_op(&mut self, op: &AirOp) {
        match op {
            AirOp::JumpClosure(jump) => self.collect_literals_in_args(&jump.args),
            AirOp::NewClosure(closure) => self.collect_literals_in_args(&closure.args),
            AirOp::SetField(set) => self.collect_literals_in_args(std::slice::from_ref(&set.value)),
            AirOp::JumpEqInt(eq) | AirOp::JumpEqStr(eq) => {
                self.collect_literals_in_args(&eq.args);
            }
            AirOp::Add(op) => self.collect_literals_in_args(&op.inputs),
            AirOp::Sub(op) => self.collect_literals_in_args(&op.inputs),
            AirOp::Mul(op) => self.collect_literals_in_args(&op.inputs),
            AirOp::Div(op) => self.collect_literals_in_args(&op.inputs),
            AirOp::Printf(call) => self.collect_literals_in_args(&call.args),
            AirOp::Sprintf(call) => self.collect_literals_in_args(&call.args),
            AirOp::Write(call) => self.collect_literals_in_args(&call.args),
            AirOp::Puts(call) => self.collect_literals_in_args(&call.args),
            AirOp::JumpArgs(call) => self.collect_literals_in_args(&call.args),
            AirOp::SysExit(syscall) => self.collect_literals_in_args(&syscall.args),
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

pub fn function<W: Write>(
    air: AirFunction,
    artifacts: &mut Artifacts,
    out: &mut W,
) -> Result<(), Error> {
    emit_runtime_helpers(&air, artifacts, out)?;
    if !artifacts.builtins_used.insert(air.sig.name.clone()) {
        return Ok(());
    }
    if runtime::emit_builtin_function(&air, artifacts, out)? {
        return Ok(());
    }
    let frame = FrameLayout::build(&air)?;
    let mut emitter = FunctionEmitter::new(air.clone(), out, frame);
    emitter.emit_function()?;
    Ok(())
}

fn emit_runtime_helpers<W: Write>(
    air: &AirFunction,
    artifacts: &mut Artifacts,
    out: &mut W,
) -> Result<(), Error> {
    let mut needs_release = false;
    let mut needs_deepcopy = false;
    for stmt in &air.items {
        match stmt {
            AirStmt::Op(AirOp::ReleaseHeap(_)) => needs_release = true,
            AirStmt::Op(AirOp::CopyField(_)) => needs_deepcopy = true,
            AirStmt::Op(AirOp::CallPtr(_)) => needs_release = true,
            _ => {}
        }
    }

    if needs_release {
        emit_runtime_helper_once(AirRuntimeHelper::ReleaseHeapPtr, artifacts, out)?;
    }
    if needs_deepcopy {
        emit_runtime_helper_once(AirRuntimeHelper::DeepCopyHeapPtr, artifacts, out)?;
        emit_runtime_helper_once(AirRuntimeHelper::MemcpyHelper, artifacts, out)?;
    }
    Ok(())
}

fn emit_runtime_helper_once<W: Write>(
    helper: AirRuntimeHelper,
    artifacts: &mut Artifacts,
    out: &mut W,
) -> Result<(), Error> {
    if !artifacts.builtins_used.insert(helper.name().to_string()) {
        return Ok(());
    }
    artifacts.externs.remove(helper.name());
    match helper {
        AirRuntimeHelper::ReleaseHeapPtr => runtime::emit_release_heap_ptr(out),
        AirRuntimeHelper::DeepCopyHeapPtr => runtime::emit_deepcopy_heap_ptr(out),
        AirRuntimeHelper::MemcpyHelper => runtime::emit_memcpy_helper(out),
    }
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
        let escaped = crate::escape_literal_for_rodata(literal);
        writeln!(out, "    db {}, 0", escaped)?;
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct Binding {
    offset: i32,
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
    fn build(air: &AirFunction) -> Result<Self, Error> {
        let mut layout = Self {
            bindings: HashMap::new(),
            stack_size: 0,
            next_offset: 0,
        };
        for param in &air.sig.params {
            layout.allocate_word(&param.name)?;
        }
        for stmt in &air.items {
            if let Some((name, ..)) = air_statement_binding_info(stmt) {
                layout.allocate_word(name)?;
            }
        }
        layout.stack_size = align_to(layout.next_offset as usize, 16) as i32;
        Ok(layout)
    }

    fn allocate_word(&mut self, name: &str) -> Result<(), Error> {
        self.next_offset += WORD_SIZE as i32;
        self.bindings.insert(
            name.to_string(),
            Binding {
                offset: self.next_offset,
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

fn air_statement_binding_info<'a>(stmt: &'a AirStmt) -> Option<(&'a str, Span)> {
    match stmt {
        AirStmt::Op(AirOp::NewClosure(s)) => Some((&s.name, s.span)),
        AirStmt::Op(AirOp::CloneClosure(s)) => Some((&s.dst, s.span)),
        AirStmt::Op(AirOp::Field(field)) => Some((field.result.as_str(), field.span)),
        AirStmt::Op(AirOp::CopyField(field)) => Some((field.result.as_str(), field.span)),
        AirStmt::Op(AirOp::ReleaseHeap(..)) => None,
        AirStmt::Label(_) => None,
        AirStmt::Op(_) => None,
    }
}

#[derive(Clone, Copy, Debug)]
struct ArgSplit {
    reg_slots: usize,
    stack_bytes: usize,
}

struct FunctionEmitter<'a, W: Write> {
    air: AirFunction,
    out: &'a mut W,
    frame: FrameLayout,
    terminated: bool,
    write_loop_counter: usize,
    label_counter: usize,
}

impl<'a, W: Write> FunctionEmitter<'a, W> {
    fn new(air: AirFunction, out: &'a mut W, frame: FrameLayout) -> Self {
        Self {
            air,
            out,
            frame,
            terminated: false,
            write_loop_counter: 0,
            label_counter: 0,
        }
    }

    fn emit_function(&mut self) -> Result<(), Error> {
        writeln!(self.out, "global {}", self.air.sig.name)?;
        writeln!(self.out, "{}:", self.air.sig.name)?;
        writeln!(self.out, "    push rbp ; save executor frame pointer")?;
        writeln!(self.out, "    mov rbp, rsp ; establish new frame base")?;
        if self.frame.stack_size > 0 {
            writeln!(
                self.out,
                "    sub rsp, {} ; reserve stack space for locals",
                self.frame.stack_size
            )?;
        }
        self.store_params()?;
        self.emit_block()?;
        Ok(())
    }

    fn emit_pin(&mut self, pin: &AirPin) -> Result<(), Error> {
        self.load_value_into_reg(&pin.value, CLOSURE_ENV_REG)?;
        Ok(())
    }

    fn store_params(&mut self) -> Result<(), Error> {
        let mut slot = 0usize;
        let mut spilled = false;
        let mut stack_offset_bytes = 0usize;
        for param in &self.air.sig.params {
            let name = &param.name;
            let binding = self
                .frame
                .binding_mut(name)
                .ok_or_else(|| Error::new(Code::Codegen, "missing binding", param.span))?;
            let slot_addr = binding.slot_addr(0);

            if !spilled && slot + 1 <= ARG_REGS.len() {
                let reg = ARG_REGS[slot];
                writeln!(
                    self.out,
                    "    mov [rbp-{}], {} ; store {} arg in frame",
                    slot_addr, reg, name
                )?;
                slot += 1;
            } else {
                spilled = true;
                let addr = 8 + stack_offset_bytes;
                writeln!(
                    self.out,
                    "    mov rax, [rbp+{}] ; load spilled {} arg",
                    addr, name
                )?;
                writeln!(
                    self.out,
                    "    mov [rbp-{}], rax ; store spilled arg",
                    slot_addr
                )?;
                stack_offset_bytes += WORD_SIZE;
            }
        }
        Ok(())
    }

    fn emit_block(&mut self) -> Result<(), Error> {
        let statements = self.air.items.clone();
        for stmt in statements {
            if self.terminated {
                if let AirStmt::Label(_) = stmt {
                    self.terminated = false;
                } else {
                    continue;
                }
            }
            self.emit_statement(&stmt)?;
        }
        Ok(())
    }

    fn emit_statement(&mut self, stmt: &AirStmt) -> Result<(), Error> {
        match stmt {
            AirStmt::Label(label) => {
                self.emit_label(label)?;
                return Ok(());
            }
            AirStmt::Op(op) => {
                self.emit_air_op(op)?;
                return Ok(());
            }
        }
    }

    fn emit_air_op(&mut self, op: &AirOp) -> Result<(), Error> {
        match op {
            AirOp::NewClosure(closure) => {
                let name = closure.name.clone();
                self.emit_new_closure(closure)?;
                self.store_binding_value(&name, closure.span)
            }
            AirOp::CloneClosure(clone) => {
                self.emit_clone_closure(clone)?;
                self.store_binding_value(&clone.dst, clone.span)
            }
            AirOp::Jump(jump) => self.emit_jump(jump),
            AirOp::JumpEqInt(eq) => self.emit_eq_int_jump(eq),
            AirOp::JumpEqStr(eq) => self.emit_eq_str_jump(eq),
            AirOp::JumpLt(jump) => self.emit_lt_jump(jump),
            AirOp::ReleaseHeap(release) => self.emit_release_heap_ptr(&release.name, release.span),
            AirOp::Pin(pin) => self.emit_pin(pin),
            AirOp::Field(field) => self.emit_get_field(field),
            AirOp::SetField(set) => self.emit_set_field(set),
            AirOp::CopyField(field) => self.emit_copy_field(field),
            AirOp::Add(op) => self.emit_add(op),
            AirOp::Sub(op) => self.emit_sub(op),
            AirOp::Mul(op) => self.emit_mul(op),
            AirOp::Div(op) => self.emit_div(op),
            AirOp::JumpGt(jump) => self.emit_gt_jump(jump),
            AirOp::Printf(op) => self.emit_libc_op(
                builtins::Builtin::Printf,
                &op.args,
                &op.arg_kinds,
                &op.target,
                op.span,
            ),
            AirOp::Sprintf(op) => self.emit_libc_op(
                builtins::Builtin::Sprintf,
                &op.args,
                &op.arg_kinds,
                &op.target,
                op.span,
            ),
            AirOp::Write(op) => self.emit_libc_op(
                builtins::Builtin::Write,
                &op.args,
                &op.arg_kinds,
                &op.target,
                op.span,
            ),
            AirOp::Puts(op) => self.emit_libc_op(
                builtins::Builtin::Puts,
                &op.args,
                &op.arg_kinds,
                &op.target,
                op.span,
            ),
            AirOp::CallPtr(call) => self.emit_call_ptr(call),
            AirOp::SysExit(syscall) => self.emit_exit_syscall(syscall),
            AirOp::JumpArgs(call) => self.emit_jump_args(call),
            AirOp::JumpClosure(jump) => self.emit_jump_closure(jump),
            AirOp::Return(ret) => self.emit_return(ret),
        }
    }

    fn emit_set_field(&mut self, set: &air::AirSetField) -> Result<(), Error> {
        let base_reg = CLOSURE_ENV_REG;
        self.load_arg_into_reg(&set.value, "rcx")?;
        self.store_at(base_reg, set.offset, "rcx")?;
        Ok(())
    }

    fn emit_clone_closure(&mut self, clone: &air::AirCloneClosure) -> Result<(), Error> {
        let src_binding = self.frame.binding(&clone.src).cloned().ok_or_else(|| {
            Error::new(
                Code::Codegen,
                format!("unknown binding '{}'", clone.src),
                clone.span,
            )
        })?;

        writeln!(
            self.out,
            "    mov rbx, [rbp-{}] ; original closure {} to {} env_end pointer for clone",
            src_binding.slot_addr(0),
            clone.src,
            clone.dst
        )?;
        self.emit_clone_env_from_env_end("rbx", CLOSURE_ENV_REG)?;
        writeln!(
            self.out,
            "    mov rax, {} ; copy cloned env_end pointer",
            CLOSURE_ENV_REG
        )?;

        Ok(())
    }

    fn store_at(&mut self, base_reg: &str, offset: isize, value_reg: &str) -> Result<(), Error> {
        let addr = self.env_field_operand(base_reg, offset);
        writeln!(
            self.out,
            "    mov [{}], {} ; store env field",
            addr, value_reg
        )?;
        Ok(())
    }

    fn env_field_operand(&self, base_reg: &str, offset: isize) -> String {
        let offset_bytes = offset * WORD_SIZE as isize;
        let abs_offset_bytes = offset_bytes.abs() as i32;
        if offset_bytes >= 0 {
            format!("{base_reg}+{abs_offset_bytes}")
        } else {
            format!("{base_reg}-{abs_offset_bytes}")
        }
    }

    fn emit_eq_int_jump(&mut self, eq: &AirJumpEq) -> Result<(), Error> {
        self.emit_builtin_int_condition(&eq.args, &eq.target)
    }

    fn emit_eq_str_jump(&mut self, eq: &AirJumpEq) -> Result<(), Error> {
        let false_label = self.new_label("eqs_false");
        self.emit_builtin_string_condition(&eq.args, &eq.target, &false_label)?;
        writeln!(self.out, "{}:", false_label)?;
        Ok(())
    }

    fn emit_lt_jump(&mut self, jump: &AirJumpLt) -> Result<(), Error> {
        self.load_value_into_reg(&jump.left, "rax")?;
        self.load_value_into_reg(&jump.right, "rbx")?;
        writeln!(self.out, "    cmp rax, rbx")?;
        writeln!(self.out, "    jl {}", jump.target)?;
        Ok(())
    }

    fn emit_gt_jump(&mut self, jump: &AirJumpGt) -> Result<(), Error> {
        self.load_value_into_reg(&jump.left, "rax")?;
        self.load_value_into_reg(&jump.right, "rbx")?;
        writeln!(self.out, "    cmp rax, rbx")?;
        writeln!(self.out, "    jg {}", jump.target)?;
        Ok(())
    }
    fn emit_add(&mut self, op: &AirAdd) -> Result<(), Error> {
        self.emit_binary_op(
            &op.inputs,
            &op.target,
            op.span,
            "add",
            "add second integer",
            false,
        )
    }

    fn emit_sub(&mut self, op: &AirSub) -> Result<(), Error> {
        self.emit_binary_op(
            &op.inputs,
            &op.target,
            op.span,
            "sub",
            "subtract subtrahend",
            false,
        )
    }

    fn emit_mul(&mut self, op: &AirMul) -> Result<(), Error> {
        self.emit_binary_op(
            &op.inputs,
            &op.target,
            op.span,
            "mul",
            "multiply by multiplier",
            false,
        )
    }

    fn emit_div(&mut self, op: &AirDiv) -> Result<(), Error> {
        self.emit_binary_op(
            &op.inputs,
            &op.target,
            op.span,
            "div",
            "divide by divisor",
            true,
        )
    }

    fn emit_binary_op(
        &mut self,
        inputs: &[AirArg],
        target: &str,
        span: Span,
        opcode: &str,
        second_comment: &str,
        is_div: bool,
    ) -> Result<(), Error> {
        if inputs.len() != 2 {
            return Err(Error::new(
                Code::Codegen,
                format!("{opcode} requires two arguments"),
                span,
            ));
        }
        self.load_arg_into_reg(&inputs[0], "rax")?;
        self.load_arg_into_reg(&inputs[1], "rbx")?;
        if is_div {
            writeln!(self.out, "    cqo ; sign extend dividend")?;
            writeln!(self.out, "    idiv rbx ; {}", second_comment)?;
        } else {
            writeln!(self.out, "    {} rax, rbx ; {}", opcode, second_comment)?;
        }
        self.emit_value_jump(target, span, true)?;
        Ok(())
    }

    fn emit_libc_op(
        &mut self,
        builtin: builtins::Builtin,
        args: &[AirArg],
        arg_kinds: &[SigKind],
        target: &str,
        span: Span,
    ) -> Result<(), Error> {
            let has_result = self.emit_libc_call(builtin, args, arg_kinds, span)?;
            self.emit_value_jump(target, span, has_result)?;
            Ok(())
        }

    fn emit_value_jump(&mut self, target: &str, span: Span, has_result: bool) -> Result<(), Error> {
        let binding = self.frame.binding(target).cloned().ok_or_else(|| {
            Error::new(Code::Codegen, format!("unknown binding '{}'", target), span)
        })?;
        writeln!(
            self.out,
            "    mov {}, [rbp-{}] ; load continuation env_end pointer",
            CLOSURE_ENV_REG,
            binding.slot_addr(0)
        )?;
        if has_result {
            self.store_at(CLOSURE_ENV_REG, -1, "rax")?;
        }
        writeln!(
            self.out,
            "    mov rax, [{}+{}] ; load continuation entry point",
            CLOSURE_ENV_REG, ENV_METADATA_UNWRAPPER_OFFSET
        )?;
        writeln!(
            self.out,
            "    mov rdi, {} ; pass env_end pointer to continuation",
            CLOSURE_ENV_REG
        )?;
        writeln!(self.out, "    leave ; unwind before jumping")?;
        writeln!(self.out, "    jmp rax")?;
        self.terminated = true;
        Ok(())
    }

    fn emit_jump_closure(&mut self, jump: &AirJumpClosure) -> Result<(), Error> {
        let binding = self.frame.binding(&jump.env_end).cloned().ok_or_else(|| {
            Error::new(
                Code::Codegen,
                format!("unknown binding '{}'", jump.env_end),
                jump.span,
            )
        })?;
        writeln!(
            self.out,
            "    mov rbx, [rbp-{}] ; load {} closure env_end pointer",
            binding.slot_addr(0),
            jump.env_end
        )?;
        let base_reg = "rbx".to_string();
        let total_args = jump.args.len();
        for (idx, arg) in jump.args.iter().enumerate() {
            self.load_arg_into_reg(arg, "rax")?;
            let offset_words = (total_args - idx) as isize;
            self.store_at(base_reg.as_str(), -offset_words, "rax")?;
        }
        writeln!(
            self.out,
            "    mov rdi, {} ; pass env_end pointer to closure",
            base_reg
        )?;
        writeln!(
            self.out,
            "    mov rax, [rdi+{}] ; load closure unwrapper entry point",
            ENV_METADATA_UNWRAPPER_OFFSET
        )?;
        writeln!(self.out, "    leave ; unwind before jumping")?;
        writeln!(self.out, "    jmp rax ; tail call into closure")?;
        self.terminated = true;
        Ok(())
    }

    fn emit_exit_syscall(&mut self, _syscall: &AirSysExit) -> Result<(), Error> {
        let (first_comment, _, _) = Self::exit_syscall_comments();
        writeln!(self.out, "    ; {}", first_comment)?;
        // Call libc exit() instead of raw exit syscall to ensure stdout is flushed
        writeln!(self.out, "    mov rdi, 0 ; exit code")?;
        writeln!(self.out, "    call exit ; call libc exit to flush buffers")?;
        self.terminated = true;
        Ok(())
    }

    fn exit_syscall_comments() -> (&'static str, &'static str, &'static str) {
        ("load exit code", "", "terminate program")
    }

    fn emit_get_field(&mut self, field: &AirField) -> Result<(), Error> {
        let base_reg = CLOSURE_ENV_REG;
        let addr = self.env_field_operand(base_reg, field.offset);
        let name = field.result.clone();
        writeln!(
            self.out,
            "    mov rax, [{}] ; load {} env field",
            addr, name
        )?;
        self.store_binding_value(&field.result, field.span)?;
        Ok(())
    }

    fn emit_release_heap_ptr(&mut self, name: &str, _span: Span) -> Result<(), Error> {
        if let Some(binding) = self.frame.binding(name) {
            let binding = binding.clone();
            let env_offset = binding.slot_addr(0);
            writeln!(
                self.out,
                "    mov rdi, [rbp-{}] ; load {} closure env_end pointer",
                env_offset, name
            )?;
        } else {
            writeln!(
                self.out,
                "    mov rdi, {} ; use pinned {} env_end pointer",
                CLOSURE_ENV_REG, name
            )?;
        }
        writeln!(
            self.out,
            "    call {} ; release {} closure environment",
            AirRuntimeHelper::ReleaseHeapPtr.name(),
            name
        )?;
        Ok(())
    }

    fn emit_copy_field(&mut self, field: &AirField) -> Result<(), Error> {
        let field_addr = self.env_field_operand(CLOSURE_ENV_REG, field.offset);
        writeln!(
            self.out,
            "    mov rcx, [{}] ; load field pointer",
            field_addr
        )?;
        writeln!(
            self.out,
            "    mov rdi, rcx ; copy pointer argument for deepcopy"
        )?;
        writeln!(
            self.out,
            "    call {} ; duplicate heap pointer",
            AirRuntimeHelper::DeepCopyHeapPtr.name()
        )?;
        writeln!(
            self.out,
            "    mov [{}], rax ; store duplicated pointer",
            field_addr
        )?;
        self.store_binding_value(&field.result, field.span)?;
        Ok(())
    }

    fn emit_call_ptr(&mut self, call: &AirCallPtr) -> Result<(), Error> {
        let name = match &call.target {
            AirCallPtrTarget::Binding(name) => name,
        };
        self.load_value_into_reg(&AirValue::Binding(name.clone()), "rdi")?;
        writeln!(
            self.out,
            "    call {} ; release heap pointer",
            AirRuntimeHelper::ReleaseHeapPtr.name()
        )?;
        Ok(())
    }

    fn emit_label(&mut self, label: &AirLabel) -> Result<(), Error> {
        writeln!(self.out, "{}:", label.name)?;
        Ok(())
    }

    fn emit_jump(&mut self, jump: &AirJump) -> Result<(), Error> {
        writeln!(self.out, "    jmp {}", jump.target)?;
        Ok(())
    }

    fn emit_builtin_int_condition(
        &mut self,
        args: &[AirArg],
        true_label: &str,
    ) -> Result<(), Error> {
        if args.len() < 2 {
            return Err(Error::new(
                Code::Codegen,
                "eq builtin requires two arguments",
                Span::unknown(),
            ));
        }
        self.load_arg_into_reg(&args[0], "rax")?;
        self.load_arg_into_reg(&args[1], "rbx")?;
        writeln!(self.out, "    cmp rax, rbx")?;
        writeln!(self.out, "    je {}", true_label)?;
        Ok(())
    }

    fn emit_builtin_string_condition(
        &mut self,
        args: &[AirArg],
        true_label: &str,
        false_label: &str,
    ) -> Result<(), Error> {
        if args.len() < 2 {
            return Err(Error::new(
                Code::Codegen,
                "eqs builtin requires two arguments",
                Span::unknown(),
            ));
        }

        self.load_arg_into_reg(&args[0], "rax")?;
        self.load_arg_into_reg(&args[1], "rbx")?;
        writeln!(self.out, "    mov r10, rax ; load first string pointer")?;
        writeln!(self.out, "    mov r11, rbx ; load second string pointer")?;

        let loop_label = self.new_label("eqs_loop");
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
        Ok(())
    }

    fn load_value_into_reg(&mut self, value: &AirValue, reg: &str) -> Result<(), Error> {
        match value {
            AirValue::Binding(name) => {
                let binding = self.frame.binding(name).cloned().ok_or_else(|| {
                    Error::new(
                        Code::Codegen,
                        format!("unknown binding '{}'", name),
                        Span::unknown(),
                    )
                })?;
                writeln!(
                    self.out,
                    "    mov {}, [rbp-{}] ; load operand",
                    reg,
                    binding.slot_addr(0)
                )?;
            }
            AirValue::Literal(value) => {
                writeln!(self.out, "    mov {}, {} ; operand literal", reg, value)?;
            }
        }
        Ok(())
    }

    fn load_arg_into_reg(&mut self, arg: &AirArg, reg: &str) -> Result<(), Error> {
        if let Some(literal) = &arg.literal {
            return match literal {
                Lit::Int(value) => {
                    self.load_value_into_reg(&AirValue::Literal(*value as i64), reg)
                }
                Lit::Str(_) => self.load_literal_into_reg(literal, &arg.name, reg),
            };
        }
        self.load_value_into_reg(&AirValue::Binding(arg.name.clone()), reg)
    }

    fn load_literal_into_reg(
        &mut self,
        literal: &Lit,
        label: &str,
        reg: &str,
    ) -> Result<(), Error> {
        match literal {
            Lit::Int(value) => {
                writeln!(
                    self.out,
                    "    mov {}, {} ; load literal integer",
                    reg, value
                )?;
            }
            Lit::Str(_) => {
                writeln!(
                    self.out,
                    "    lea {}, [rel {}] ; point to string literal",
                    reg, label
                )?;
            }
        }
        Ok(())
    }

    fn emit_return(&mut self, ret: &AirReturn) -> Result<(), Error> {
        if let Some(name) = &ret.value {
            let binding = self.frame.binding(name).cloned().ok_or_else(|| {
                Error::new(
                    Code::Codegen,
                    format!("unknown binding '{}'", name),
                    Span::unknown(),
                )
            })?;
            writeln!(
                self.out,
                "    mov rax, [rbp-{}] ; load return value",
                binding.slot_addr(0)
            )?;
        }
        writeln!(self.out, "    leave")?;
        writeln!(self.out, "    ret")?;
        writeln!(self.out)?;
        self.terminated = true;
        Ok(())
    }

    fn emit_new_closure(&mut self, c: &AirNewClosure) -> Result<(), Error> {
        let sig = &c.target;
        let args = &c.args;

        let kinds = &sig.param_kinds();
        let env_size = kinds.len() * WORD_SIZE;
        let heap_size = env_size + ENV_METADATA_SIZE;

        self.emit_mmap(heap_size)?;
        writeln!(self.out, "    mov rbx, rax ; closure env base pointer")?;

        let mut offset_words = 0usize;
        for (arg, kind) in args.iter().zip(kinds.iter()) {
            self.load_arg_into_reg(arg, "rax")?;
            let kind_words = 1;
            if kind_words == 0 {
                continue;
            }
            let offset_bytes = offset_words * WORD_SIZE;
            if matches!(*kind, SigKind::Sig(_)) {
                writeln!(
                    self.out,
                    "    mov {}, rax ; shadow closure env_end pointer",
                    CLOSURE_ENV_REG
                )?;
                writeln!(self.out, "    push rbx ; save env base pointer")?;
                self.clone_closure_argument()?;
                writeln!(self.out, "    pop rbx ; restore env base pointer")?;
                writeln!(
                    self.out,
                    "    mov [rbx+{}], {} ; capture cloned closure pointer",
                    offset_bytes, CLOSURE_ENV_REG
                )?;
            } else {
                writeln!(
                    self.out,
                    "    mov [rbx+{}], rax ; capture arg into env",
                    offset_bytes
                )?;
            }
            offset_words += kind_words;
        }

        writeln!(
            self.out,
            "    mov {}, rbx ; env_end pointer before metadata",
            CLOSURE_ENV_REG
        )?;
        if env_size > 0 {
            writeln!(
                self.out,
                "    add {}, {} ; move pointer past env payload",
                CLOSURE_ENV_REG, env_size
            )?;
        }

        writeln!(
            self.out,
            "    mov rax, {} ; store env size metadata",
            env_size
        )?;
        writeln!(
            self.out,
            "    mov qword [{}+{}], rax ; env size metadata",
            CLOSURE_ENV_REG, ENV_METADATA_ENV_SIZE_OFFSET
        )?;
        writeln!(
            self.out,
            "    mov rax, {} ; store heap size metadata",
            heap_size
        )?;
        writeln!(
            self.out,
            "    mov qword [{}+{}], rax ; heap size metadata",
            CLOSURE_ENV_REG, ENV_METADATA_HEAP_SIZE_OFFSET
        )?;

        let unwrapper = c.unwrapper_label();
        writeln!(
            self.out,
            "    lea rax, [{}] ; load unwrapper entry point",
            unwrapper
        )?;
        writeln!(
            self.out,
            "    mov qword [{}+{}], rax ; store unwrapper entry in metadata",
            CLOSURE_ENV_REG, ENV_METADATA_UNWRAPPER_OFFSET
        )?;

        let release_helper = c.deep_release_label();
        writeln!(
            self.out,
            "    lea rax, [{}] ; load release helper entry point",
            release_helper
        )?;
        writeln!(
            self.out,
            "    mov qword [{}+{}], rax ; store release pointer in metadata",
            CLOSURE_ENV_REG, ENV_METADATA_RELEASE_OFFSET
        )?;

        let deep_copy_helper = c.deepcopy_label();
        writeln!(
            self.out,
            "    lea rax, [{}] ; load deep copy helper entry point",
            deep_copy_helper
        )?;
        writeln!(
            self.out,
            "    mov qword [{}+{}], rax ; store deep copy pointer in metadata",
            CLOSURE_ENV_REG, ENV_METADATA_DEEP_COPY_OFFSET
        )?;

        let num_remaining = kinds.len().saturating_sub(args.len());
        writeln!(
            self.out,
            "    mov qword [{}+{}], {} ; store num_remaining",
            CLOSURE_ENV_REG, ENV_METADATA_NUM_REMAINING_OFFSET, num_remaining
        )?;

        writeln!(
            self.out,
            "    mov rax, {} ; copy {} closure env_end to rax",
            CLOSURE_ENV_REG, c.name
        )?;

        Ok(())
    }

    fn store_binding_value(&mut self, name: &str, span: Span) -> Result<(), Error> {
        let binding = self.frame.binding_mut(name).ok_or_else(|| {
            Error::new(Code::Codegen, format!("unknown binding '{}'", name), span)
        })?;
        writeln!(
            self.out,
            "    mov [rbp-{}], rax ; store value",
            binding.slot_addr(0)
        )?;
        Ok(())
    }

    fn emit_jump_args(&mut self, ja: &AirJumpArgs) -> Result<(), Error> {
        let sig = &ja.target;
        let args = &ja.args;
        self.prepare_args(args)?;
        let arg_split = self.move_args_to_registers(&sig.param_kinds())?;
        let spilled_bytes = arg_split.stack_bytes;
        if spilled_bytes > 0 {
            writeln!(self.out, "    sub rsp, 8 ; allocate slot for saved rbp")?;
            writeln!(self.out, "    mov rax, [rbp] ; capture parent rbp")?;
            writeln!(self.out, "    mov [rsp], rax ; stash parent rbp for leave")?;
            writeln!(self.out, "    mov rbp, rsp ; treat slot as current rbp")?;
        }
        writeln!(self.out, "    leave ; unwind before named jump")?;
        writeln!(self.out, "    jmp {}", &sig.name)?;
        self.terminated = true;
        Ok(())
    }

    fn emit_libc_call(
        &mut self,
        builtin: builtins::Builtin,
        args: &[AirArg],
        arg_kinds: &[SigKind],
        span: Span,
        ) -> Result<bool, Error> {
            match builtin {
            builtins::Builtin::Printf => {
                if args.is_empty() {
                    return Err(Error::new(
                        Code::Codegen,
                        "printf requires a format string before the continuation",
                        span,
                    ));
                }

                self.prepare_args(args)?;
                let arg_split = self.move_args_to_registers(arg_kinds)?;
                self.emit_variadic_libc_call(builtin.name())?;
                self.cleanup_libc_stack(arg_split.stack_bytes)?;

                Ok(false)
            }
            builtins::Builtin::Sprintf => {
                if args.is_empty() {
                    return Err(Error::new(
                        Code::Codegen,
                        "sprintf requires a format string before the continuation",
                        span,
                    ));
                }

                self.prepare_args(args)?;

                self.emit_mmap(FMT_BUFFER_SIZE)?;
                writeln!(self.out, "    mov rbx, rax ; keep sprintf buffer pointer")?;
                let arg_split = self.move_args_to_registers(arg_kinds)?;
                if arg_split.reg_slots == ARG_REGS.len() {
                    return Err(Error::new(
                        Code::Codegen,
                        "sprintf requires at least one register slot for the buffer pointer",
                        span,
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
                self.emit_variadic_libc_call(builtin.name())?;
                writeln!(
                    self.out,
                    "    mov rax, rbx ; return formatted string pointer"
                )?;
                self.cleanup_libc_stack(arg_split.stack_bytes)?;

                Ok(true)
            }
            builtins::Builtin::Write => {
                if args.is_empty() {
                    return Err(Error::new(
                        Code::Codegen,
                        "write requires a buffer before the continuation",
                        span,
                    ));
                }

                self.prepare_args(args)?;
                let arg_split = self.move_args_to_registers(arg_kinds)?;

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
                self.cleanup_libc_stack(arg_split.stack_bytes)?;

                Ok(false)
            }
            builtins::Builtin::Puts => {
                if args.is_empty() {
                    return Err(Error::new(
                        Code::Codegen,
                        "puts requires a buffer before the continuation",
                        span,
                    ));
                }

                self.prepare_args(args)?;
                let arg_split = self.move_args_to_registers(arg_kinds)?;

                writeln!(self.out, "    call puts ; invoke libc puts")?;
                self.cleanup_libc_stack(arg_split.stack_bytes)?;

                Ok(false)
            }
            _ => Err(Error::new(
                Code::Codegen,
                format!("unsupported libc call '{}'", builtin.name()),
                span,
            )),
        }
    }

    fn cleanup_libc_stack(&mut self, stack_bytes: usize) -> Result<(), Error> {
        if stack_bytes > 0 {
            writeln!(
                self.out,
                "    add rsp, {} ; pop stack args after libc call",
                stack_bytes
            )?;
        }
        Ok(())
    }

    fn emit_variadic_libc_call(&mut self, name: &str) -> Result<(), Error> {
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

    fn emit_mmap(&mut self, size: usize) -> Result<(), Error> {
        writeln!(self.out, "    mov rax, {} ; mmap syscall", SYSCALL_MMAP)?;
        writeln!(
            self.out,
            "    xor rdi, rdi ; addr hint for kernel base selection"
        )?;
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

    fn prepare_args(&mut self, args: &[AirArg]) -> Result<(), Error> {
        for arg in args.iter().rev() {
            self.load_arg_into_reg(arg, "rax")?;
            writeln!(self.out, "    push rax ; stack arg")?;
        }
        Ok(())
    }

    fn move_args_to_registers(&mut self, params: &[SigKind]) -> Result<ArgSplit, Error> {
        let mut slot = 0usize;
        let mut spilled = false;
        let mut stack_bytes = 0usize;
        for _ty in params {
            let required = 1;
            if !spilled && slot + required <= ARG_REGS.len() {
                let reg = ARG_REGS[slot];
                writeln!(self.out, "    pop {} ; restore arg into register", reg)?;
                slot += required;
            } else {
                spilled = true;
                stack_bytes += WORD_SIZE;
            }
        }
        Ok(ArgSplit {
            reg_slots: slot,
            stack_bytes,
        })
    }

    fn emit_clone_env_from_env_end(
        &mut self,
        src_env_end_reg: &str,
        dst_env_end_reg: &str,
    ) -> Result<(), Error> {
        writeln!(
            self.out,
            "    mov rbx, {} ; clone source env_end pointer",
            src_env_end_reg
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
        writeln!(self.out, "    mov r12, rbx ; compute env base pointer for clone")?;
        writeln!(self.out, "    sub r12, r13 ; env base pointer for clone source")?;
        writeln!(self.out, "    mov rax, {} ; mmap syscall", SYSCALL_MMAP)?;
        writeln!(
            self.out,
            "    xor rdi, rdi ; addr hint for kernel base selection"
        )?;
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
            "    mov {}, rbx ; cloned env_end pointer",
            dst_env_end_reg
        )?;
        writeln!(
            self.out,
            "    mov rax, [{}+{}] ; load deepcopy helper entry point",
            dst_env_end_reg, ENV_METADATA_DEEP_COPY_OFFSET
        )?;
        writeln!(
            self.out,
            "    push {} ; preserve cloned env_end pointer",
            dst_env_end_reg
        )?;
        writeln!(
            self.out,
            "    mov rdi, {} ; pass env_end pointer to deepcopy helper",
            dst_env_end_reg
        )?;
        writeln!(self.out, "    call rax ; deepcopy reference fields")?;
        writeln!(
            self.out,
            "    pop {} ; restore cloned env_end pointer",
            dst_env_end_reg
        )?;
        Ok(())
    }

    fn clone_closure_argument(&mut self) -> Result<(), Error> {
        self.emit_clone_env_from_env_end(CLOSURE_ENV_REG, CLOSURE_ENV_REG)?;
        writeln!(
            self.out,
            "    mov rax, {} ; copy closure env_end to rax",
            CLOSURE_ENV_REG
        )?;
        Ok(())
    }

    fn next_write_loop_labels(&mut self) -> (String, String) {
        let idx = self.write_loop_counter;
        self.write_loop_counter += 1;
        let prefix = crate::sanitize_function_name(&self.air.sig.name);
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
            crate::sanitize_function_name(&self.air.sig.name),
            suffix,
            idx
        )
    }
}

fn align_to(value: usize, align: usize) -> usize {
    if value == 0 {
        return 0;
    }
    ((value + align - 1) / align) * align
}
