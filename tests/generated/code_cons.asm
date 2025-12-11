bits 64
default rel
section .text
global nil
nil:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-16], rdi ; save closure code pointer
    mov [rbp-8], rsi ; save closure environment pointer
    mov [rbp-32], rdx ; save closure code pointer
    mov [rbp-24], rcx ; save closure environment pointer
    mov rdi, [rbp-8] ; load closure env_end pointer
    call internal_release_env ; release closure environment
    mov rax, [rbp-32] ; load closure code for call
    mov rdx, [rbp-24] ; load closure env_end for call
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov rdi, rdx ; pass env_end pointer as parameter
    leave ; unwind before calling closure
    jmp rax ; jump into fully applied closure
nil_closure_entry:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish wrapper frame
    sub rsp, 16 ; reserve space for env metadata scratch
    mov [rbp-8], rdi ; stash env_end pointer for release
    push rbx ; preserve base register
    mov rbx, rdi ; rdi points to env_end when invoked
    sub rbx, 32 ; compute env base
    mov rdi, [rbx+0] ; load continuation code pointer
    push rdi ; preserve closure code register
    mov rsi, [rbx+8] ; load continuation env_end pointer
    push rsi ; preserve closure env_end register
    mov rdx, [rbx+16] ; load continuation code pointer
    push rdx ; preserve closure code register
    mov rcx, [rbx+24] ; load continuation env_end pointer
    push rcx ; preserve closure env_end register
    mov rdx, [rbp-8] ; load saved env_end pointer
    mov rcx, [rdx] ; read env size metadata
    mov rsi, [rdx+8] ; read heap size metadata
    mov rbx, rdx ; env_end pointer for release
    sub rbx, rcx ; compute env base pointer
    mov rdi, rbx ; munmap base pointer
    mov rax, 11 ; munmap syscall
    syscall ; release wrapper closure environment
    pop rcx ; restore parameter register
    pop rdx ; restore parameter register
    pop rsi ; restore parameter register
    pop rdi ; restore parameter register
    pop rbx ; restore saved base register
    leave ; epilogue: restore rbp of caller
    jmp nil ; jump into actual function
global cons
cons:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov [rbp-32], rsi ; save closure code pointer
    mov [rbp-24], rdx ; save closure environment pointer
    mov [rbp-48], rcx ; save closure code pointer
    mov [rbp-40], r8 ; save closure environment pointer
    mov [rbp-64], r9 ; save closure code pointer
    mov [rbp-56], r10 ; save closure environment pointer
    mov rdi, [rbp-56] ; load closure env_end pointer
    call internal_release_env ; release closure environment
    mov rax, [rbp-48] ; load closure code for call
    mov rdx, [rbp-40] ; load closure env_end for call
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rax, [rbp-16] ; load scalar from frame
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 24 ; compute slot for next argument
    mov [rbx], rax ; store scalar arg in env
    mov rax, [rbp-32] ; load closure code pointer
    mov rdx, [rbp-24] ; load closure env_end pointer
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 16 ; compute slot for next argument
    mov [rbx], rax ; store closure code for arg
    mov [rbx+8], rdx ; store closure env_end for arg
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov rdi, rdx ; pass env_end pointer as parameter
    leave ; unwind before calling closure
    jmp rax ; jump into fully applied closure
cons_closure_entry:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish wrapper frame
    sub rsp, 16 ; reserve space for env metadata scratch
    mov [rbp-8], rdi ; stash env_end pointer for release
    push rbx ; preserve base register
    mov rbx, rdi ; rdi points to env_end when invoked
    sub rbx, 56 ; compute env base
    mov rdi, [rbx+0] ; load scalar param from env
    push rdi ; preserve parameter register
    mov rsi, [rbx+8] ; load continuation code pointer
    push rsi ; preserve closure code register
    mov rdx, [rbx+16] ; load continuation env_end pointer
    push rdx ; preserve closure env_end register
    mov rcx, [rbx+24] ; load continuation code pointer
    push rcx ; preserve closure code register
    mov r8, [rbx+32] ; load continuation env_end pointer
    push r8 ; preserve closure env_end register
    mov r9, [rbx+40] ; load continuation code pointer
    push r9 ; preserve closure code register
    mov r10, [rbx+48] ; load continuation env_end pointer
    push r10 ; preserve closure env_end register
    mov rdx, [rbp-8] ; load saved env_end pointer
    mov rcx, [rdx] ; read env size metadata
    mov rsi, [rdx+8] ; read heap size metadata
    mov rbx, rdx ; env_end pointer for release
    sub rbx, rcx ; compute env base pointer
    mov rdi, rbx ; munmap base pointer
    mov rax, 11 ; munmap syscall
    syscall ; release wrapper closure environment
    pop r10 ; restore parameter register
    pop r9 ; restore parameter register
    pop r8 ; restore parameter register
    pop rcx ; restore parameter register
    pop rdx ; restore parameter register
    pop rsi ; restore parameter register
    pop rdi ; restore parameter register
    pop rbx ; restore saved base register
    leave ; epilogue: restore rbp of caller
    jmp cons ; jump into actual function
global _start
_start:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    leave ; epilogue: restore rbp and rsp
    mov rax, 60 ; exit syscall
    xor rdi, rdi
    syscall
internal_release_env:
    push rbp ; prologue: save caller frame pointer
    mov rbp, rsp ; prologue: establish new frame
    push rbx ; preserve callee-saved registers
    push r12
    push r13
    push r14
    push r15
    mov r12, rdi ; capture env_end pointer
    test r12, r12 ; skip null releases
    je internal_release_env_done
    mov rcx, [r12] ; load env size metadata
    mov r15, [r12+8] ; load heap size metadata
    mov rbx, r12 ; copy env_end pointer
    sub rbx, rcx ; compute env base pointer
    mov r13, [r12+16] ; load pointer count metadata
    lea r14, [r12+24] ; pointer metadata base
    xor r9d, r9d ; reset pointer metadata index
internal_release_env_loop:
    cmp r9, r13 ; finished child pointers?
    jge internal_release_env_children_done
    mov r10, [r14+r9*8] ; load child env offset
    mov r11, [rbx+r10] ; load child env_end pointer
    mov rdi, r11 ; pass child env_end pointer
    call internal_release_env ; recurse into child closure
    inc r9 ; advance metadata index
    jmp internal_release_env_loop
internal_release_env_children_done:
    mov rdi, rbx ; env base for munmap
    mov rax, 11 ; munmap syscall
    mov rsi, r15 ; heap size for munmap
    syscall ; release closure environment
internal_release_env_done:
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    ret
