bits 64
default rel
section .text
global _1_foo__0_foo__0_foo__0_foo
_1_foo__0_foo__0_foo__0_foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, 0 ; load literal integer
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 32 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rbx, rax ; env base pointer
    mov rax, [rbp-32] ; load scalar from frame
    mov [rbx+0], rax ; store builtin env argument
    mov rdx, rbx ; env base pointer
    add rdx, 8 ; bump pointer past env data
    mov qword [rdx], 8 ; env size metadata
    mov qword [rdx+8], 32 ; heap size metadata
    mov qword [rdx+16], 0 ; pointer count metadata
    mov rax, exit_closure_entry ; load closure entry point
    mov [rbp-48], rax ; update closure code pointer
    mov [rbp-40], rdx ; update closure environment pointer
    mov rax, [rbp-48] ; load closure code pointer
    mov rdx, [rbp-40] ; load closure env_end pointer
    push rax ; preserve continuation code pointer
    push rdx ; preserve continuation env_end pointer
    mov rax, [rbp-16] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    mov r8, rdi ; keep string pointer
    xor rcx, rcx ; reset length counter
_1_foo__0_foo__0_foo__0_foo_write_strlen_loop_0:
    mov dl, byte [r8+rcx] ; load current character
    cmp dl, 0 ; stop at terminator
    je _1_foo__0_foo__0_foo__0_foo_write_strlen_done_0
    inc rcx ; advance char counter
    jmp _1_foo__0_foo__0_foo__0_foo_write_strlen_loop_0
_1_foo__0_foo__0_foo__0_foo_write_strlen_done_0:
    mov rsi, r8 ; buffer start
    mov rdx, rcx ; length to write
    mov rdi, 1 ; stdout fd
    call write ; invoke libc write
    pop rdx ; restore continuation env_end pointer
    pop rax ; restore continuation code pointer
    mov rdi, rdx ; pass env_end pointer to continuation
    leave ; unwind before jumping
    jmp rax ; jump into continuation
global _1_foo__0_foo__0_foo__0_foo_closure_entry
_1_foo__0_foo__0_foo__0_foo_closure_entry:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    sub rax, 8 ; compute env base pointer
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load scalar from frame
    mov rax, [rax+0] ; load scalar env field
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, [rbp-48] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    leave ; unwind before named jump
    jmp _1_foo__0_foo__0_foo__0_foo ; jump to fully applied function
global _1_foo__0_foo
_1_foo__0_foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 32 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 8 ; bump pointer past env header
    mov qword [rdx], 8 ; env size metadata
    mov qword [rdx+8], 32 ; heap size metadata
    mov qword [rdx+16], 0 ; pointer count metadata
    mov rax, _1_foo__0_foo__0_foo__0_foo_closure_entry ; load wrapper entry point
    mov [rbp-32], rax ; update closure code pointer
    mov [rbp-24], rdx ; update closure environment pointer
    mov rax, [rbp-32] ; load closure code pointer
    mov rdx, [rbp-24] ; load closure env_end pointer
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-16] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore closure code into register
    pop rdx ; restore closure env_end into register
    leave ; unwind before named jump
    jmp itoa ; jump to fully applied function
global _1_foo__0_foo_closure_entry
_1_foo__0_foo_closure_entry:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    sub rax, 8 ; compute env base pointer
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load scalar from frame
    mov rax, [rax+0] ; load scalar env field
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, [rbp-48] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    leave ; unwind before named jump
    jmp _1_foo__0_foo ; jump to fully applied function
global foo
foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov [rbp-32], rsi ; store scalar arg in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 32 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 8 ; bump pointer past env header
    mov qword [rdx], 8 ; env size metadata
    mov qword [rdx+8], 32 ; heap size metadata
    mov qword [rdx+16], 0 ; pointer count metadata
    mov rax, _1_foo__0_foo_closure_entry ; load wrapper entry point
    mov [rbp-48], rax ; update closure code pointer
    mov [rbp-40], rdx ; update closure environment pointer
    mov rax, [rbp-48] ; load closure code pointer
    mov rdx, [rbp-40] ; load closure env_end pointer
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-16] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore scalar arg into register
    pop rdx ; restore closure code into register
    pop rcx ; restore closure env_end into register
    leave ; unwind before named jump
    jmp add ; jump to fully applied function
global foo_closure_entry
foo_closure_entry:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    sub rax, 16 ; compute env base pointer
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load scalar from frame
    mov rax, [rax+0] ; load scalar env field
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load scalar from frame
    mov rax, [rax+8] ; load scalar env field
    mov [rbp-64], rax ; save evaluated scalar in frame
    mov rax, [rbp-64] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-48] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore scalar arg into register
    leave ; unwind before named jump
    jmp foo ; jump to fully applied function
global _start
_start:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov rax, 1 ; load literal integer
    mov [rbp-16], rax ; save evaluated scalar in frame
    mov rax, 2 ; load literal integer
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-16] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore scalar arg into register
    leave ; unwind before named jump
    jmp foo ; jump to fully applied function
global add
add:
    mov rax, rdi ;
    add rax, rsi ;
    lea rbx, [rcx-8] ; reserve slot for result before metadata
    mov [rbx], rax ;
    mov rax, rdx ; continuation entry point
    mov rdi, rcx ; pass env_end pointer unchanged
    jmp rax ; jump into continuation

global itoa
itoa:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame
    push rsi ; preserve continuation code pointer
    push rdx ; preserve continuation env pointer
    mov rax, rdi ; capture integer argument
    mov r10, 0x8000000000000000 ; i64 min constant
    cmp rax, r10
    je itoa_min_value
    push rdi ; keep integer while mmap runs
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 64 ; allocate buffer for digits
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate buffer pages
    pop rdi ; restore integer argument
    mov r8, rax ; buffer base pointer
    xor r10, r10 ; reuse r10 as sign flag
    mov rax, rdi
    cmp rax, 0
    jge itoa_abs_done
    neg rax
    mov r10, 1
itoa_abs_done:
    lea r9, [r8+64] ; pointer past buffer end
    mov byte [r9-1], 0 ; null terminator
    mov r11, r9 ; cursor for digits
    mov rcx, 10
    cmp rax, 0
    jne itoa_digit_loop
    dec r11
    mov byte [r11], '0'
    jmp itoa_check_sign
itoa_digit_loop:
    xor rdx, rdx
    div rcx
    dec r11
    add dl, '0'
    mov [r11], dl
    test rax, rax
    jne itoa_digit_loop
itoa_check_sign:
    cmp r10, 0
    je itoa_set_pointer
    dec r11
    mov byte [r11], '-'
itoa_set_pointer:
    mov r8, r11 ; string start
    jmp itoa_tail
itoa_min_value:
    lea r8, [rel str_literal_0] ; reuse static string
    jmp itoa_tail
itoa_tail:
    mov rsi, [rbp-8] ; continuation code pointer
    mov rdx, [rbp-16] ; continuation env pointer
    sub rsp, 16 ; allocate temp stack for closure state
    mov [rsp], rsi ; save code pointer
    mov [rsp+8], rdx ; save env_end cursor
    mov r10, [rsp+8] ; env_end cursor
    sub r10, 8 ; reserve space for string argument
    mov [r10], r8 ; store string pointer
    mov rax, [rsp] ; restore code pointer
    mov rdx, [rsp+8] ; restore env_end pointer
    add rsp, 16 ; pop temp state
    mov rdi, rdx ; pass env_end pointer to continuation
    leave ; unwind before jump
    jmp rax ; jump into continuation

global exit_closure_entry
exit_closure_entry:
    push rbp ; prologue for exit closure
    mov rbp, rsp
    push rbx ; preserve callee-saved register
    push r12 ; preserve callee-saved register for continuation storage
    push r13 ; preserve callee-saved register for continuation storage
    mov rbx, rdi ; env_end pointer
    sub rbx, 8 ; reference exit closure env base
    mov rax, [rbx] ; fetch stored exit code
    pop rbx ; restore callee-saved register
    mov rdi, rax ; pass exit code to syscall
    leave ; unwind entry frame before exiting
    mov rax, 60 ; exit syscall
    syscall ; exit program

extern write
section .rodata
str_literal_0:
    db "-9223372036854775808", 0
