use std::io::Write;

use crate::compiler::air;
use crate::compiler::codegen::{
    Artifacts, CLOSURE_ENV_REG, ENV_METADATA_DEEP_COPY_OFFSET, ENV_METADATA_ENV_SIZE_OFFSET,
    ENV_METADATA_HEAP_SIZE_OFFSET, MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE, SYSCALL_MMAP,
    SYSCALL_MUNMAP,
};
use crate::compiler::error;

pub fn emit_builtin_function<W: Write>(
    air: &air::AirFunction,
    artifacts: &mut Artifacts,
    out: &mut W,
) -> Result<bool, error::Error> {
    if air.items.is_empty() {
        match air.sig.name.as_str() {
            "itoa" => {
                emit_builtin_itoa(artifacts, out)?;
                return Ok(true);
            }
            "release_heap_ptr" => {
                emit_release_heap_ptr(out)?;
                return Ok(true);
            }
            "deepcopy_heap_ptr" => {
                emit_deepcopy_heap_ptr(out)?;
                return Ok(true);
            }
            "memcpy_helper" => {
                emit_memcpy_helper(out)?;
                return Ok(true);
            }
            _ => {}
        }
    }
    Ok(false)
}

pub fn emit_release_heap_ptr<W: Write>(out: &mut W) -> Result<(), error::Error> {
    writeln!(out, "global release_heap_ptr")?;
    writeln!(out, "release_heap_ptr:")?;
    writeln!(out, "    push rbp ; save caller frame")?;
    writeln!(out, "    mov rbp, rsp ; establish frame")?;
    writeln!(out, "    push rbx ; preserve rbx")?;
    writeln!(out, "    mov rbx, rdi ; keep env_end pointer")?;
    writeln!(
        out,
        "    mov rcx, [rbx+{}] ; load env size metadata",
        ENV_METADATA_ENV_SIZE_OFFSET
    )?;
    writeln!(
        out,
        "    mov rdx, [rbx+{}] ; load heap size metadata",
        ENV_METADATA_HEAP_SIZE_OFFSET
    )?;
    writeln!(out, "    mov rdi, rbx")?;
    writeln!(out, "    sub rdi, rcx ; compute env base pointer")?;
    writeln!(out, "    mov rsi, rdx ; heap size for munmap")?;
    writeln!(out, "    mov rax, {} ; munmap syscall", SYSCALL_MUNMAP)?;
    writeln!(out, "    syscall")?;
    writeln!(out, "    pop rbx")?;
    writeln!(out, "    pop rbp")?;
    writeln!(out, "    ret")?;
    Ok(())
}

pub fn emit_deepcopy_heap_ptr<W: Write>(out: &mut W) -> Result<(), error::Error> {
    writeln!(out, "global deepcopy_heap_ptr")?;
    writeln!(out, "deepcopy_heap_ptr:")?;
    writeln!(out, "    push rbp ; prologue: save executor frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; prologue: establish new frame")?;
    writeln!(out, "    push rbx ; preserve callee-saved registers")?;
    writeln!(out, "    push r12")?;
    writeln!(out, "    push r13")?;
    writeln!(out, "    push r14")?;
    writeln!(out, "    push r15")?;
    writeln!(out, "    mov r12, rdi ; capture env_end pointer")?;
    writeln!(
        out,
        "    mov r14, [r12+{}] ; load env size metadata",
        ENV_METADATA_ENV_SIZE_OFFSET
    )?;
    writeln!(
        out,
        "    mov r15, [r12+{}] ; load heap size metadata",
        ENV_METADATA_HEAP_SIZE_OFFSET
    )?;
    writeln!(out, "    mov rbx, r12 ; keep env_end pointer")?;
    writeln!(out, "    sub rbx, r14 ; compute env base pointer")?;
    writeln!(out, "    mov rdi, 0 ; addr hint so kernel picks mmap base")?;
    writeln!(out, "    mov rsi, r15 ; length = heap size")?;
    writeln!(
        out,
        "    mov rdx, {} ; prot = read/write",
        PROT_READ | PROT_WRITE
    )?;
    writeln!(
        out,
        "    mov r10, {} ; flags = private & anonymous",
        MAP_PRIVATE | MAP_ANONYMOUS
    )?;
    writeln!(out, "    mov r8, -1 ; fd = -1")?;
    writeln!(out, "    xor r9, r9 ; offset = 0")?;
    writeln!(out, "    mov rax, {} ; mmap syscall", SYSCALL_MMAP)?;
    writeln!(out, "    syscall ; allocate new closure env")?;
    writeln!(out, "    mov r13, rax ; new env base pointer")?;
    writeln!(out, "    mov rdi, r13 ; memcpy dest")?;
    writeln!(out, "    mov rsi, rbx ; memcpy src")?;
    writeln!(out, "    mov rdx, r15 ; memcpy length")?;
    writeln!(out, "    call memcpy_helper ; copy env contents")?;
    writeln!(out, "    mov rax, r13 ; compute new env_end pointer")?;
    writeln!(out, "    add rax, r14")?;
    writeln!(out, "    mov r15, rax ; preserve new env_end pointer")?;
    writeln!(
        out,
        "    mov rax, [r15+{}] ; load deep copy helper entry",
        ENV_METADATA_DEEP_COPY_OFFSET
    )?;
    writeln!(out, "    mov rdi, r15 ; pass new env_end pointer")?;
    writeln!(out, "    call rax ; invoke helper")?;
    writeln!(out, "    mov rax, r15 ; return new env_end pointer")?;
    writeln!(out, "    pop r15")?;
    writeln!(out, "    pop r14")?;
    writeln!(out, "    pop r13")?;
    writeln!(out, "    pop r12")?;
    writeln!(out, "    pop rbx")?;
    writeln!(out, "    pop rbp")?;
    writeln!(out, "    ret")?;
    Ok(())
}

pub fn emit_memcpy_helper<W: Write>(out: &mut W) -> Result<(), error::Error> {
    writeln!(out, "global memcpy_helper")?;
    writeln!(out, "memcpy_helper:")?;
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

pub fn emit_builtin_itoa<W: Write>(
    artifacts: &mut Artifacts,
    out: &mut W,
) -> Result<(), error::Error> {
    const ITOA_MIN_LABEL: &str = "itoa_min_value";
    const ITOA_MIN_VALUE: &str = "-9223372036854775808";
    artifacts.add_string_literal(ITOA_MIN_LABEL, ITOA_MIN_VALUE);
    writeln!(out, "global itoa")?;
    writeln!(out, "itoa:")?;
    writeln!(out, "    push rbp ; save executor frame pointer")?;
    writeln!(out, "    mov rbp, rsp ; establish new frame")?;
    writeln!(out, "    push rsi ; preserve continuation code pointer")?;
    writeln!(
        out,
        "    push {} ; preserve continuation env pointer",
        CLOSURE_ENV_REG
    )?;
    writeln!(out, "    mov rax, rdi ; capture integer argument")?;
    writeln!(out, "    mov r10, 0x8000000000000000 ; i64 min constant")?;
    writeln!(out, "    cmp rax, r10")?;
    writeln!(out, "    je itoa_min_value")?;
    writeln!(out, "    push rdi ; keep integer while mmap runs")?;
    writeln!(out, "    mov rax, {} ; mmap syscall", SYSCALL_MMAP)?;
    writeln!(
        out,
        "    xor rdi, rdi ; addr hint for kernel base selection"
    )?;
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
    writeln!(out, "    mov byte [r9-1], 0 ; string terminator")?;
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
    writeln!(
        out,
        "    mov {}, [rbp-16] ; continuation env pointer",
        CLOSURE_ENV_REG
    )?;
    writeln!(
        out,
        "    sub rsp, 16 ; allocate temp stack for closure state"
    )?;
    writeln!(out, "    mov [rsp], rsi ; save code pointer")?;
    writeln!(
        out,
        "    mov [rsp+8], {} ; save env_end cursor",
        CLOSURE_ENV_REG
    )?;
    writeln!(out, "    mov r10, [rsp+8] ; env_end cursor")?;
    writeln!(out, "    sub r10, 8 ; reserve space for string argument")?;
    writeln!(out, "    mov [r10], r8 ; store string pointer")?;
    writeln!(out, "    mov rax, [rsp] ; restore code pointer")?;
    writeln!(
        out,
        "    mov {}, [rsp+8] ; restore env_end pointer",
        CLOSURE_ENV_REG
    )?;
    writeln!(out, "    add rsp, 16 ; pop temp state")?;
    writeln!(
        out,
        "    mov rdi, {} ; pass env_end pointer to continuation",
        CLOSURE_ENV_REG
    )?;
    writeln!(out, "    leave ; unwind before jump")?;
    writeln!(out, "    jmp rax ; jump into continuation")?;
    writeln!(out)?;
    Ok(())
}
