bits 64
default rel
section .text
global foo
foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 400 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov [rbp-32], rsi ; store scalar arg in frame
    mov [rbp-48], rdx ; store scalar arg in frame
    mov [rbp-64], rcx ; store scalar arg in frame
    mov [rbp-80], r8 ; store scalar arg in frame
    mov [rbp-96], r9 ; store scalar arg in frame
    mov [rbp-112], r10 ; store scalar arg in frame
    mov [rbp-128], r11 ; store scalar arg in frame
    mov [rbp-144], r12 ; store scalar arg in frame
    mov [rbp-160], r13 ; store scalar arg in frame
    mov [rbp-176], r14 ; store scalar arg in frame
    mov [rbp-192], r15 ; store scalar arg in frame
    mov rax, [rbp+8] ; load spilled scalar arg
    mov [rbp-208], rax ; store spilled scalar arg
    mov rax, [rbp+16] ; load spilled scalar arg
    mov [rbp-224], rax ; store spilled scalar arg
    mov rax, [rbp+24] ; load spilled scalar arg
    mov [rbp-240], rax ; store spilled scalar arg
    mov rax, [rbp+32] ; load spilled scalar arg
    mov [rbp-256], rax ; store spilled scalar arg
    mov rax, [rbp+40] ; load spilled scalar arg
    mov [rbp-272], rax ; store spilled scalar arg
    mov rax, [rbp+48] ; load spilled scalar arg
    mov [rbp-288], rax ; store spilled scalar arg
    mov rax, [rbp+56] ; load spilled scalar arg
    mov [rbp-304], rax ; store spilled scalar arg
    mov rax, [rbp+64] ; load spilled scalar arg
    mov [rbp-320], rax ; store spilled scalar arg
    mov rax, [rbp+72] ; load spilled scalar arg
    mov [rbp-336], rax ; store spilled scalar arg
    mov rax, [rbp+80] ; load spilled scalar arg
    mov [rbp-352], rax ; store spilled scalar arg
    mov rax, [rbp+88] ; load spilled closure code
    mov rdx, [rbp+96] ; load spilled closure env
    mov [rbp-368], rax ; save spilled closure code pointer
    mov [rbp-360], rdx ; save spilled closure env_end pointer
    lea rax, [rel _0] ; point to string literal
    mov [rbp-384], rax ; save evaluated scalar in frame
    mov rax, [rbp-384] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    mov r8, rdi ; keep string pointer
    xor rcx, rcx ; reset length counter
foo_write_strlen_loop_0:
    mov dl, byte [r8+rcx] ; load current character
    cmp dl, 0 ; stop at terminator
    je foo_write_strlen_done_0
    inc rcx ; advance char counter
    jmp foo_write_strlen_loop_0
foo_write_strlen_done_0:
    mov rdx, rcx ; length to write
    mov rsi, r8 ; buffer start
    mov rdi, 1 ; stdout fd
    call write ; invoke libc write
    mov [rbp-400], rax ; save evaluated scalar in frame
    mov rax, [rbp-368] ; load closure code for exec
    mov rdx, [rbp-360] ; load closure env_end for exec
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov rdi, rdx ; pass env_end pointer as parameter
    leave ; unwind before calling closure
    jmp rax ; jump into fully applied closure
global foo_unwrapper
foo_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 384 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-192] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-184] ; load scalar env field
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-176] ; load scalar env field
    mov [rbp-64], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-168] ; load scalar env field
    mov [rbp-80], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-160] ; load scalar env field
    mov [rbp-96], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-152] ; load scalar env field
    mov [rbp-112], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-144] ; load scalar env field
    mov [rbp-128], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-136] ; load scalar env field
    mov [rbp-144], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-128] ; load scalar env field
    mov [rbp-160], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-120] ; load scalar env field
    mov [rbp-176], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-112] ; load scalar env field
    mov [rbp-192], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-104] ; load scalar env field
    mov [rbp-208], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-96] ; load scalar env field
    mov [rbp-224], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-88] ; load scalar env field
    mov [rbp-240], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-80] ; load scalar env field
    mov [rbp-256], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-72] ; load scalar env field
    mov [rbp-272], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-64] ; load scalar env field
    mov [rbp-288], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-56] ; load scalar env field
    mov [rbp-304], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-48] ; load scalar env field
    mov [rbp-320], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-40] ; load scalar env field
    mov [rbp-336], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-32] ; load scalar env field
    mov [rbp-352], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-24] ; load scalar env field
    mov [rbp-368], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rax, [r10-16] ; load closure code pointer
    mov rdx, [r10-8] ; load closure env_end pointer
    mov [rbp-384], rax ; update closure code pointer
    mov [rbp-376], rdx ; update closure environment pointer
    mov rax, [rbp-384] ; load closure code pointer
    mov rdx, [rbp-376] ; load closure env_end pointer
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-368] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-352] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-336] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-320] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-304] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-288] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-272] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-256] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-240] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-224] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-208] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-192] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-176] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-160] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-144] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-128] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-112] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-96] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-80] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-64] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-48] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore scalar arg into register
    pop rdx ; restore scalar arg into register
    pop rcx ; restore scalar arg into register
    pop r8 ; restore scalar arg into register
    pop r9 ; restore scalar arg into register
    pop r10 ; restore scalar arg into register
    pop r11 ; restore scalar arg into register
    pop r12 ; restore scalar arg into register
    pop r13 ; restore scalar arg into register
    pop r14 ; restore scalar arg into register
    pop r15 ; restore scalar arg into register
    sub rsp, 8 ; allocate slot for saved rbp
    mov rax, [rbp] ; capture parent rbp
    mov [rsp], rax ; stash parent rbp for leave
    mov rbp, rsp ; treat slot as current rbp
    leave ; unwind before named jump
    jmp foo ; jump to fully applied function
global _24_lambda
_24_lambda:
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
global _24_lambda_unwrapper
_24_lambda_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    leave ; unwind before named jump
    jmp _24_lambda ; jump to fully applied function
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
    sub rsp, 368 ; reserve stack space for locals
    mov rax, 1 ; load literal integer
    mov [rbp-16], rax ; save evaluated scalar in frame
    mov rax, 2 ; load literal integer
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, 3 ; load literal integer
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, 4 ; load literal integer
    mov [rbp-64], rax ; save evaluated scalar in frame
    mov rax, 5 ; load literal integer
    mov [rbp-80], rax ; save evaluated scalar in frame
    mov rax, 6 ; load literal integer
    mov [rbp-96], rax ; save evaluated scalar in frame
    mov rax, 7 ; load literal integer
    mov [rbp-112], rax ; save evaluated scalar in frame
    mov rax, 8 ; load literal integer
    mov [rbp-128], rax ; save evaluated scalar in frame
    mov rax, 9 ; load literal integer
    mov [rbp-144], rax ; save evaluated scalar in frame
    mov rax, 10 ; load literal integer
    mov [rbp-160], rax ; save evaluated scalar in frame
    mov rax, 11 ; load literal integer
    mov [rbp-176], rax ; save evaluated scalar in frame
    mov rax, 12 ; load literal integer
    mov [rbp-192], rax ; save evaluated scalar in frame
    mov rax, 13 ; load literal integer
    mov [rbp-208], rax ; save evaluated scalar in frame
    mov rax, 14 ; load literal integer
    mov [rbp-224], rax ; save evaluated scalar in frame
    mov rax, 15 ; load literal integer
    mov [rbp-240], rax ; save evaluated scalar in frame
    mov rax, 16 ; load literal integer
    mov [rbp-256], rax ; save evaluated scalar in frame
    mov rax, 17 ; load literal integer
    mov [rbp-272], rax ; save evaluated scalar in frame
    mov rax, 18 ; load literal integer
    mov [rbp-288], rax ; save evaluated scalar in frame
    mov rax, 19 ; load literal integer
    mov [rbp-304], rax ; save evaluated scalar in frame
    mov rax, 20 ; load literal integer
    mov [rbp-320], rax ; save evaluated scalar in frame
    mov rax, 21 ; load literal integer
    mov [rbp-336], rax ; save evaluated scalar in frame
    mov rax, 22 ; load literal integer
    mov [rbp-352], rax ; save evaluated scalar in frame
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
    mov rax, _24_lambda_unwrapper ; load unwrapper entry point
    mov [rbp-368], rax ; update closure code pointer
    mov [rbp-360], rdx ; update closure environment pointer
    mov rax, [rbp-368] ; load closure code pointer
    mov rdx, [rbp-360] ; load closure env_end pointer
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-352] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-336] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-320] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-304] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-288] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-272] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-256] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-240] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-224] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-208] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-192] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-176] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-160] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-144] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-128] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-112] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-96] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-80] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-64] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-48] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-16] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore scalar arg into register
    pop rdx ; restore scalar arg into register
    pop rcx ; restore scalar arg into register
    pop r8 ; restore scalar arg into register
    pop r9 ; restore scalar arg into register
    pop r10 ; restore scalar arg into register
    pop r11 ; restore scalar arg into register
    pop r12 ; restore scalar arg into register
    pop r13 ; restore scalar arg into register
    pop r14 ; restore scalar arg into register
    pop r15 ; restore scalar arg into register
    sub rsp, 8 ; allocate slot for saved rbp
    mov rax, [rbp] ; capture parent rbp
    mov [rsp], rax ; stash parent rbp for leave
    mov rbp, rsp ; treat slot as current rbp
    leave ; unwind before named jump
    jmp foo ; jump to fully applied function
extern write
section .rodata
_0:
    db "All arguments received successfully.", 10, 0
