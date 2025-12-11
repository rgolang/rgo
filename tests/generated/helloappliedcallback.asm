bits 64
default rel
section .text
global bar
bar:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov [rbp-32], rsi ; store scalar arg in frame
    mov [rbp-48], rdx ; save closure code pointer
    mov [rbp-40], rcx ; save closure environment pointer
    lea rax, [rel str_literal_0] ; point to string literal
    mov [rbp-64], rax ; save evaluated scalar in frame
    mov rax, [rbp-48] ; load closure code pointer
    mov rdx, [rbp-40] ; load closure env_end pointer
    push rax ; preserve continuation code pointer
    push rdx ; preserve continuation env_end pointer
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-16] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-64] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore scalar arg into register
    pop rdx ; restore scalar arg into register
    sub rsp, 8 ; align stack for variadic call
    call printf ; invoke libc printf
    add rsp, 8
    mov rdi, [rel stdout] ; flush stdout
    sub rsp, 8 ; align stack for fflush
    call fflush
    add rsp, 8
    pop rdx ; restore continuation env_end pointer
    pop rax ; restore continuation code pointer
    mov rdi, rdx ; pass env_end pointer to continuation
    leave ; unwind before jumping
    jmp rax ; jump into continuation
bar_closure_entry:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish wrapper frame
    sub rsp, 16 ; reserve space for env metadata scratch
    mov [rbp-8], rdi ; stash env_end pointer for release
    push rbx ; preserve base register
    mov rbx, rdi ; rdi points to env_end when invoked
    sub rbx, 32 ; compute env base
    mov rdi, [rbx+0] ; load scalar param from env
    push rdi ; preserve parameter register
    mov rsi, [rbx+8] ; load scalar param from env
    push rsi ; preserve parameter register
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
    jmp bar ; jump into actual function
global foo
foo:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-16], rdi ; save closure code pointer
    mov [rbp-8], rsi ; save closure environment pointer
    mov [rbp-32], rdx ; save closure code pointer
    mov [rbp-24], rcx ; save closure environment pointer
    lea rax, [rel str_literal_1] ; point to string literal
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load closure code for call
    mov rdx, [rbp-8] ; load closure env_end for call
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rax, [rbp-48] ; load scalar from frame
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
foo_closure_entry:
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
    jmp foo ; jump into actual function
global _3
_3:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov rax, 0 ; load literal integer
    mov [rbp-16], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rdi, rax ; pass exit code
    leave ; unwind before exit
    mov rax, 60 ; exit syscall
    syscall ; exit program
_3_closure_entry:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish wrapper frame
    sub rsp, 16 ; reserve space for env metadata scratch
    mov [rbp-8], rdi ; stash env_end pointer for release
    push rbx ; preserve base register
    mov rbx, rdi ; rdi points to env_end when invoked
    mov rdx, [rbp-8] ; load saved env_end pointer
    mov rcx, [rdx] ; read env size metadata
    mov rsi, [rdx+8] ; read heap size metadata
    mov rbx, rdx ; env_end pointer for release
    sub rbx, rcx ; compute env base pointer
    mov rdi, rbx ; munmap base pointer
    mov rax, 11 ; munmap syscall
    syscall ; release wrapper closure environment
    pop rbx ; restore saved base register
    leave ; epilogue: restore rbp of caller
    jmp _3 ; jump into actual function
global _start
_start:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    lea rax, [rel str_literal_2] ; point to string literal
    mov [rbp-16], rax ; save evaluated scalar in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 64 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 32 ; bump pointer past env header
    mov qword [rdx], 32 ; env size metadata
    mov qword [rdx+8], 64 ; heap size metadata
    mov qword [rdx+16], 1 ; pointer count metadata
    mov qword [rdx+24], 24 ; closure env pointer slot offset
    mov rax, bar_closure_entry ; load wrapper entry point
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rbx, [rsp+8] ; original closure env_end pointer
    mov r13, [rbx] ; load env size metadata for clone
    mov r14, [rbx+8] ; load heap size metadata for clone
    mov r12, rbx ; compute env base pointer for clone
    sub r12, r13 ; env base pointer for clone source
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, r14 ; length for cloned environment
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate cloned env pages
    mov r15, rax ; cloned closure env base pointer
    mov rsi, r12 ; source env base for clone copy
    mov rdi, r15 ; destination env base for clone copy
    mov rcx, r14 ; bytes to copy for cloned env
    cld ; ensure forward copy for env clone
    rep movsb ; duplicate closure env data
    mov rbx, r15 ; start from cloned env base
    add rbx, r13 ; compute cloned env_end pointer
    mov [rsp+8], rbx ; operate on cloned closure env
    mov rax, [rbp-16] ; load scalar from frame
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 32 ; compute slot for next argument
    mov [rbx], rax ; store scalar arg in env
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov [rbp-32], rax ; update closure code pointer
    mov [rbp-24], rdx ; update closure environment pointer
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
    mov rax, _3_closure_entry ; load wrapper entry point
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
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
    jmp foo ; jump to fully applied function
extern fflush
extern printf
extern stdout
section .rodata
str_literal_0:
    db "msg1: %s, msg2: %s", 10, 0
str_literal_1:
    db "bye", 0
str_literal_2:
    db "hi", 0
