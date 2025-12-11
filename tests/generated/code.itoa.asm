bits 64
default rel
section .text
global _2
_2:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, 0 ; load literal integer
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load scalar from frame
    mov rdi, rax ; pass exit code
    leave ; unwind before exit
    mov rax, 60 ; exit syscall
    syscall ; exit program
_2_closure_entry:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish wrapper frame
    sub rsp, 16 ; reserve space for env metadata scratch
    mov [rbp-8], rdi ; stash env_end pointer for release
    push rbx ; preserve base register
    mov rbx, rdi ; rdi points to env_end when invoked
    sub rbx, 8 ; compute env base
    mov rdi, [rbx+0] ; load scalar param from env
    push rdi ; preserve parameter register
    mov rdx, [rbp-8] ; load saved env_end pointer
    mov rcx, [rdx] ; read env size metadata
    mov rsi, [rdx+8] ; read heap size metadata
    mov rbx, rdx ; env_end pointer for release
    sub rbx, rcx ; compute env base pointer
    mov rdi, rbx ; munmap base pointer
    mov rax, 11 ; munmap syscall
    syscall ; release wrapper closure environment
    pop rdi ; restore parameter register
    pop rbx ; restore saved base register
    leave ; epilogue: restore rbp of caller
    jmp _2 ; jump into actual function
global _start
_start:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov rax, 1 ; load literal integer
    mov [rbp-16], rax ; save evaluated scalar in frame
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
    mov rax, _2_closure_entry ; load wrapper entry point
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
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
global itoa
itoa:
    push rbp ; save caller frame pointer
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

section .rodata
str_literal_0:
    db "-9223372036854775808", 0
