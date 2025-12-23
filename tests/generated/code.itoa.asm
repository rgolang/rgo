bits 64
default rel
section .text
global _2_lambda
_2_lambda:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, 0 ; load literal integer
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    leave ; unwind before named jump
    jmp exit ; jump to fully applied function
global _2_lambda_unwrapper
_2_lambda_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-8] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    leave ; unwind before named jump
    jmp _2_lambda ; jump to fully applied function
global exit
exit:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    ; load exit code
    leave ; unwind before exiting
    mov rax, 60 ; exit syscall
    syscall ; terminate program
global exit_unwrapper
exit_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-8] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    leave ; unwind before named jump
    jmp exit ; jump to fully applied function
global _start
_start:
    push rbp ; save executor frame pointer
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
    mov rax, _2_lambda_unwrapper ; load unwrapper entry point
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
    lea r8, [rel itoa_min_value] ; reuse static string
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
itoa_min_value:
    db "-9223372036854775808", 0
