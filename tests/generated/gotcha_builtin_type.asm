bits 64
default rel
section .text
global w
w:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov [rbp-32], rsi ; save closure code pointer
    mov [rbp-24], rdx ; save closure environment pointer
    lea rax, [rel _0] ; point to string literal
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-48] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore scalar arg into register
    call printf_aligned
    mov rdi, [rel stdout] ; flush stdout
    sub rsp, 8 ; align stack for fflush
    call fflush
    add rsp, 8
    mov [rbp-64], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load closure code for exec
    mov rdx, [rbp-24] ; load closure env_end for exec
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov rdi, rdx ; pass env_end pointer as parameter
    leave ; unwind before calling closure
    jmp rax ; jump into fully applied closure
global w_unwrapper
w_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-24] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rax, [r10-16] ; load closure code pointer
    mov rdx, [r10-8] ; load closure env_end pointer
    mov [rbp-48], rax ; update closure code pointer
    mov [rbp-40], rdx ; update closure environment pointer
    mov rax, [rbp-48] ; load closure code pointer
    mov rdx, [rbp-40] ; load closure env_end pointer
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore closure code into register
    pop rdx ; restore closure env_end into register
    leave ; unwind before named jump
    jmp w ; jump to fully applied function
global _3_lambda
_3_lambda:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov rax, 0 ; load literal integer
    mov [rbp-16], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    leave ; unwind before named jump
    jmp exit ; jump to fully applied function
global _3_lambda_unwrapper
_3_lambda_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    leave ; unwind before named jump
    jmp _3_lambda ; jump to fully applied function
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
    lea rax, [rel _1] ; point to string literal
    mov [rbp-16], rax ; save evaluated scalar in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 24 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    mov qword [rdx], 0 ; env size metadata
    mov qword [rdx+8], 24 ; heap size metadata
    mov qword [rdx+16], 0 ; pointer count metadata
    mov rax, _3_lambda_unwrapper ; load unwrapper entry point
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
    jmp w ; jump to fully applied function
global printf_aligned
printf_aligned:
    push rbp ; save caller base pointer
    mov rbp, rsp ; establish helper frame
    push r12 ; preserve alignment register
    mov rax, rsp ; capture pointer for alignment
    and rax, 15
    mov r12, rax
    sub rsp, r12 ; align stack for variadic printf call
    call printf
    add rsp, r12
    pop r12
    leave
    ret
extern fflush
extern printf
extern stdout
section .rodata
_0:
    db "Gotcha: %s", 0
_1:
    db "one", 0
