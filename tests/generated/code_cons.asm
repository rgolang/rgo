bits 64
default rel
section .text
global nil
nil:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-16], rdi ; save closure code pointer
    mov [rbp-8], rsi ; save closure environment pointer
    mov [rbp-32], rdx ; save closure code pointer
    mov [rbp-24], rcx ; save closure environment pointer
    mov rax, [rbp-32] ; load closure code for exec
    mov rdx, [rbp-24] ; load closure env_end for exec
    mov rdi, rdx ; pass env_end pointer as parameter
    leave ; unwind before calling closure
    jmp rax ; jump into fully applied closure
global nil_unwrapper
nil_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rax, [r10-32] ; load closure code pointer
    mov rdx, [r10-24] ; load closure env_end pointer
    mov [rbp-32], rax ; update closure code pointer
    mov [rbp-24], rdx ; update closure environment pointer
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
    mov rax, [rbp-32] ; load closure code pointer
    mov rdx, [rbp-24] ; load closure env_end pointer
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    pop rdi ; restore closure code into register
    pop rsi ; restore closure env_end into register
    pop rdx ; restore closure code into register
    pop rcx ; restore closure env_end into register
    leave ; unwind before named jump
    jmp nil ; jump to fully applied function
global cons
cons:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov [rbp-32], rsi ; save closure code pointer
    mov [rbp-24], rdx ; save closure environment pointer
    mov [rbp-48], rcx ; save closure code pointer
    mov [rbp-40], r8 ; save closure environment pointer
    mov [rbp-64], r9 ; save closure code pointer
    mov [rbp-56], r10 ; save closure environment pointer
    mov rax, [rbp-48] ; load closure code for exec
    mov rdx, [rbp-40] ; load closure env_end for exec
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
global cons_unwrapper
cons_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 80 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-56] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rax, [r10-48] ; load closure code pointer
    mov rdx, [r10-40] ; load closure env_end pointer
    mov [rbp-48], rax ; update closure code pointer
    mov [rbp-40], rdx ; update closure environment pointer
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rax, [r10-32] ; load closure code pointer
    mov rdx, [r10-24] ; load closure env_end pointer
    mov [rbp-64], rax ; update closure code pointer
    mov [rbp-56], rdx ; update closure environment pointer
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rax, [r10-16] ; load closure code pointer
    mov rdx, [r10-8] ; load closure env_end pointer
    mov [rbp-80], rax ; update closure code pointer
    mov [rbp-72], rdx ; update closure environment pointer
    mov rax, [rbp-80] ; load closure code pointer
    mov rdx, [rbp-72] ; load closure env_end pointer
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-64] ; load closure code pointer
    mov rdx, [rbp-56] ; load closure env_end pointer
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-48] ; load closure code pointer
    mov rdx, [rbp-40] ; load closure env_end pointer
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore closure code into register
    pop rdx ; restore closure env_end into register
    pop rcx ; restore closure code into register
    pop r8 ; restore closure env_end into register
    pop r9 ; restore closure code into register
    pop r10 ; restore closure env_end into register
    leave ; unwind before named jump
    jmp cons ; jump to fully applied function
