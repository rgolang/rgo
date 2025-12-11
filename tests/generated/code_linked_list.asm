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
global iterate
iterate:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    mov [rbp-16], rdi ; save closure code pointer
    mov [rbp-8], rsi ; save closure environment pointer
    mov [rbp-32], rdx ; save closure code pointer
    mov [rbp-24], rcx ; save closure environment pointer
    mov [rbp-48], r8 ; save closure code pointer
    mov [rbp-40], r9 ; save closure environment pointer
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 104 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 56 ; bump pointer past env header
    mov qword [rdx], 56 ; env size metadata
    mov qword [rdx+8], 104 ; heap size metadata
    mov qword [rdx+16], 3 ; pointer count metadata
    mov qword [rdx+24], 8 ; closure env pointer slot offset
    mov qword [rdx+32], 24 ; closure env pointer slot offset
    mov qword [rdx+40], 48 ; closure env pointer slot offset
    mov rax, _iterate_1_closure_entry ; load wrapper entry point
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
    mov rax, [rbp-16] ; load closure code pointer
    mov rdx, [rbp-8] ; load closure env_end pointer
    mov [rsp+16], rax ; stash closure code pointer for clone
    mov rbx, rdx ; original closure env_end pointer
    mov r13, [rbx] ; load env size metadata for clone
    mov r14, [rbx+8] ; load heap size metadata for clone
    mov r12, rbx ; compute env base pointer for clone source
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
    mov rdx, rbx ; use cloned env_end pointer for argument
    mov rax, [rsp+16] ; restore closure code pointer after clone
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 56 ; compute slot for next argument
    mov [rbx], rax ; store closure code for arg
    mov [rbx+8], rdx ; store closure env_end for arg
    mov rax, [rbp-48] ; load closure code pointer
    mov rdx, [rbp-40] ; load closure env_end pointer
    mov [rsp+16], rax ; stash closure code pointer for clone
    mov rbx, rdx ; original closure env_end pointer
    mov r13, [rbx] ; load env size metadata for clone
    mov r14, [rbx+8] ; load heap size metadata for clone
    mov r12, rbx ; compute env base pointer for clone source
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
    mov rdx, rbx ; use cloned env_end pointer for argument
    mov rax, [rsp+16] ; restore closure code pointer after clone
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 40 ; compute slot for next argument
    mov [rbx], rax ; store closure code for arg
    mov [rbx+8], rdx ; store closure env_end for arg
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov [rbp-64], rax ; update closure code pointer
    mov [rbp-56], rdx ; update closure environment pointer
    mov rax, [rbp-32] ; load closure code for call
    mov rdx, [rbp-24] ; load closure env_end for call
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rax, [rbp-64] ; load closure code pointer
    mov rdx, [rbp-56] ; load closure env_end pointer
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 32 ; compute slot for next argument
    mov [rbx], rax ; store closure code for arg
    mov [rbx+8], rdx ; store closure env_end for arg
    mov rax, [rbp-48] ; load closure code pointer
    mov rdx, [rbp-40] ; load closure env_end pointer
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
iterate_closure_entry:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish wrapper frame
    sub rsp, 16 ; reserve space for env metadata scratch
    mov [rbp-8], rdi ; stash env_end pointer for release
    push rbx ; preserve base register
    mov rbx, rdi ; rdi points to env_end when invoked
    sub rbx, 48 ; compute env base
    mov rdi, [rbx+0] ; load continuation code pointer
    push rdi ; preserve closure code register
    mov rsi, [rbx+8] ; load continuation env_end pointer
    push rsi ; preserve closure env_end register
    mov rdx, [rbx+16] ; load continuation code pointer
    push rdx ; preserve closure code register
    mov rcx, [rbx+24] ; load continuation env_end pointer
    push rcx ; preserve closure env_end register
    mov r8, [rbx+32] ; load continuation code pointer
    push r8 ; preserve closure code register
    mov r9, [rbx+40] ; load continuation env_end pointer
    push r9 ; preserve closure env_end register
    mov rdx, [rbp-8] ; load saved env_end pointer
    mov rcx, [rdx] ; read env size metadata
    mov rsi, [rdx+8] ; read heap size metadata
    mov rbx, rdx ; env_end pointer for release
    sub rbx, rcx ; compute env base pointer
    mov rdi, rbx ; munmap base pointer
    mov rax, 11 ; munmap syscall
    syscall ; release wrapper closure environment
    pop r9 ; restore parameter register
    pop r8 ; restore parameter register
    pop rcx ; restore parameter register
    pop rdx ; restore parameter register
    pop rsi ; restore parameter register
    pop rdi ; restore parameter register
    pop rbx ; restore saved base register
    leave ; epilogue: restore rbp of caller
    jmp iterate ; jump into actual function
global _iterate_1
_iterate_1:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 80 ; reserve stack space for locals
    mov [rbp-16], rdi ; save closure code pointer
    mov [rbp-8], rsi ; save closure environment pointer
    mov [rbp-32], rdx ; save closure code pointer
    mov [rbp-24], rcx ; save closure environment pointer
    mov [rbp-48], r8 ; store scalar arg in frame
    mov [rbp-64], r9 ; save closure code pointer
    mov [rbp-56], r10 ; save closure environment pointer
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 96 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 48 ; bump pointer past env header
    mov qword [rdx], 48 ; env size metadata
    mov qword [rdx+8], 96 ; heap size metadata
    mov qword [rdx+16], 3 ; pointer count metadata
    mov qword [rdx+24], 8 ; closure env pointer slot offset
    mov qword [rdx+32], 24 ; closure env pointer slot offset
    mov qword [rdx+40], 40 ; closure env pointer slot offset
    mov rax, iterate_closure_entry ; load wrapper entry point
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rax, [rbp-16] ; load closure code pointer
    mov rdx, [rbp-8] ; load closure env_end pointer
    mov [rsp+16], rax ; stash closure code pointer for clone
    mov rbx, rdx ; original closure env_end pointer
    mov r13, [rbx] ; load env size metadata for clone
    mov r14, [rbx+8] ; load heap size metadata for clone
    mov r12, rbx ; compute env base pointer for clone source
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
    mov rdx, rbx ; use cloned env_end pointer for argument
    mov rax, [rsp+16] ; restore closure code pointer after clone
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 48 ; compute slot for next argument
    mov [rbx], rax ; store closure code for arg
    mov [rbx+8], rdx ; store closure env_end for arg
    mov rax, [rbp-64] ; load closure code pointer
    mov rdx, [rbp-56] ; load closure env_end pointer
    mov [rsp+16], rax ; stash closure code pointer for clone
    mov rbx, rdx ; original closure env_end pointer
    mov r13, [rbx] ; load env size metadata for clone
    mov r14, [rbx+8] ; load heap size metadata for clone
    mov r12, rbx ; compute env base pointer for clone source
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
    mov rdx, rbx ; use cloned env_end pointer for argument
    mov rax, [rsp+16] ; restore closure code pointer after clone
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 32 ; compute slot for next argument
    mov [rbx], rax ; store closure code for arg
    mov [rbx+8], rdx ; store closure env_end for arg
    mov rax, [rbp-32] ; load closure code pointer
    mov rdx, [rbp-24] ; load closure env_end pointer
    mov [rsp+16], rax ; stash closure code pointer for clone
    mov rbx, rdx ; original closure env_end pointer
    mov r13, [rbx] ; load env size metadata for clone
    mov r14, [rbx+8] ; load heap size metadata for clone
    mov r12, rbx ; compute env base pointer for clone source
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
    mov rdx, rbx ; use cloned env_end pointer for argument
    mov rax, [rsp+16] ; restore closure code pointer after clone
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 16 ; compute slot for next argument
    mov [rbx], rax ; store closure code for arg
    mov [rbx+8], rdx ; store closure env_end for arg
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov [rbp-80], rax ; update closure code pointer
    mov [rbp-72], rdx ; update closure environment pointer
    mov rax, [rbp-16] ; load closure code for call
    mov rdx, [rbp-8] ; load closure env_end for call
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rax, [rbp-48] ; load scalar from frame
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 24 ; compute slot for next argument
    mov [rbx], rax ; store scalar arg in env
    mov rax, [rbp-80] ; load closure code pointer
    mov rdx, [rbp-72] ; load closure env_end pointer
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
_iterate_1_closure_entry:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish wrapper frame
    sub rsp, 16 ; reserve space for env metadata scratch
    mov [rbp-8], rdi ; stash env_end pointer for release
    push rbx ; preserve base register
    mov rbx, rdi ; rdi points to env_end when invoked
    sub rbx, 56 ; compute env base
    mov rdi, [rbx+0] ; load continuation code pointer
    push rdi ; preserve closure code register
    mov rsi, [rbx+8] ; load continuation env_end pointer
    push rsi ; preserve closure env_end register
    mov rdx, [rbx+16] ; load continuation code pointer
    push rdx ; preserve closure code register
    mov rcx, [rbx+24] ; load continuation env_end pointer
    push rcx ; preserve closure env_end register
    mov r8, [rbx+32] ; load scalar param from env
    push r8 ; preserve parameter register
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
    jmp _iterate_1 ; jump into actual function
global handler
handler:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov [rbp-32], rsi ; save closure code pointer
    mov [rbp-24], rdx ; save closure environment pointer
    lea rax, [rel str_literal_0] ; point to string literal
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load closure code pointer
    mov rdx, [rbp-24] ; load closure env_end pointer
    push rax ; preserve continuation code pointer
    push rdx ; preserve continuation env_end pointer
    mov rax, [rbp-16] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-48] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore scalar arg into register
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
handler_closure_entry:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish wrapper frame
    sub rsp, 16 ; reserve space for env metadata scratch
    mov [rbp-8], rdi ; stash env_end pointer for release
    push rbx ; preserve base register
    mov rbx, rdi ; rdi points to env_end when invoked
    sub rbx, 24 ; compute env base
    mov rdi, [rbx+0] ; load scalar param from env
    push rdi ; preserve parameter register
    mov rsi, [rbx+8] ; load continuation code pointer
    push rsi ; preserve closure code register
    mov rdx, [rbx+16] ; load continuation env_end pointer
    push rdx ; preserve closure env_end register
    mov rdx, [rbp-8] ; load saved env_end pointer
    mov rcx, [rdx] ; read env size metadata
    mov rsi, [rdx+8] ; read heap size metadata
    mov rbx, rdx ; env_end pointer for release
    sub rbx, rcx ; compute env base pointer
    mov rdi, rbx ; munmap base pointer
    mov rax, 11 ; munmap syscall
    syscall ; release wrapper closure environment
    pop rdx ; restore parameter register
    pop rsi ; restore parameter register
    pop rdi ; restore parameter register
    pop rbx ; restore saved base register
    leave ; epilogue: restore rbp of caller
    jmp handler ; jump into actual function
global end
end:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    lea rax, [rel str_literal_1] ; point to string literal
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
    mov rax, _end_2_closure_entry ; load wrapper entry point
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
    push rax ; preserve continuation code pointer
    push rdx ; preserve continuation env_end pointer
    mov rax, [rbp-16] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
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
end_closure_entry:
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
    jmp end ; jump into actual function
global _end_2
_end_2:
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
_end_2_closure_entry:
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
    jmp _end_2 ; jump into actual function
global _start
_start:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 176 ; reserve stack space for locals
    mov rax, 1 ; load literal integer
    mov [rbp-16], rax ; save evaluated scalar in frame
    mov rax, 2 ; load literal integer
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, 3 ; load literal integer
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, 4 ; load literal integer
    mov [rbp-64], rax ; save evaluated scalar in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 72 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 32 ; bump pointer past env header
    mov qword [rdx], 32 ; env size metadata
    mov qword [rdx+8], 72 ; heap size metadata
    mov qword [rdx+16], 2 ; pointer count metadata
    mov qword [rdx+24], 8 ; closure env pointer slot offset
    mov qword [rdx+32], 24 ; closure env pointer slot offset
    mov rax, nil_closure_entry ; load wrapper entry point
    mov [rbp-80], rax ; update closure code pointer
    mov [rbp-72], rdx ; update closure environment pointer
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 104 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 56 ; bump pointer past env header
    mov qword [rdx], 56 ; env size metadata
    mov qword [rdx+8], 104 ; heap size metadata
    mov qword [rdx+16], 3 ; pointer count metadata
    mov qword [rdx+24], 16 ; closure env pointer slot offset
    mov qword [rdx+32], 32 ; closure env pointer slot offset
    mov qword [rdx+40], 48 ; closure env pointer slot offset
    mov rax, cons_closure_entry ; load wrapper entry point
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
    mov rax, [rbp-64] ; load scalar from frame
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 56 ; compute slot for next argument
    mov [rbx], rax ; store scalar arg in env
    mov rax, [rbp-80] ; load closure code pointer
    mov rdx, [rbp-72] ; load closure env_end pointer
    mov [rsp+16], rax ; stash closure code pointer for clone
    mov rbx, rdx ; original closure env_end pointer
    mov r13, [rbx] ; load env size metadata for clone
    mov r14, [rbx+8] ; load heap size metadata for clone
    mov r12, rbx ; compute env base pointer for clone source
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
    mov rdx, rbx ; use cloned env_end pointer for argument
    mov rax, [rsp+16] ; restore closure code pointer after clone
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 48 ; compute slot for next argument
    mov [rbx], rax ; store closure code for arg
    mov [rbx+8], rdx ; store closure env_end for arg
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov [rbp-96], rax ; update closure code pointer
    mov [rbp-88], rdx ; update closure environment pointer
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 104 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 56 ; bump pointer past env header
    mov qword [rdx], 56 ; env size metadata
    mov qword [rdx+8], 104 ; heap size metadata
    mov qword [rdx+16], 3 ; pointer count metadata
    mov qword [rdx+24], 16 ; closure env pointer slot offset
    mov qword [rdx+32], 32 ; closure env pointer slot offset
    mov qword [rdx+40], 48 ; closure env pointer slot offset
    mov rax, cons_closure_entry ; load wrapper entry point
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
    mov rax, [rbp-48] ; load scalar from frame
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 56 ; compute slot for next argument
    mov [rbx], rax ; store scalar arg in env
    mov rax, [rbp-96] ; load closure code pointer
    mov rdx, [rbp-88] ; load closure env_end pointer
    mov [rsp+16], rax ; stash closure code pointer for clone
    mov rbx, rdx ; original closure env_end pointer
    mov r13, [rbx] ; load env size metadata for clone
    mov r14, [rbx+8] ; load heap size metadata for clone
    mov r12, rbx ; compute env base pointer for clone source
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
    mov rdx, rbx ; use cloned env_end pointer for argument
    mov rax, [rsp+16] ; restore closure code pointer after clone
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 48 ; compute slot for next argument
    mov [rbx], rax ; store closure code for arg
    mov [rbx+8], rdx ; store closure env_end for arg
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov [rbp-112], rax ; update closure code pointer
    mov [rbp-104], rdx ; update closure environment pointer
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 104 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 56 ; bump pointer past env header
    mov qword [rdx], 56 ; env size metadata
    mov qword [rdx+8], 104 ; heap size metadata
    mov qword [rdx+16], 3 ; pointer count metadata
    mov qword [rdx+24], 16 ; closure env pointer slot offset
    mov qword [rdx+32], 32 ; closure env pointer slot offset
    mov qword [rdx+40], 48 ; closure env pointer slot offset
    mov rax, cons_closure_entry ; load wrapper entry point
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
    mov rax, [rbp-32] ; load scalar from frame
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 56 ; compute slot for next argument
    mov [rbx], rax ; store scalar arg in env
    mov rax, [rbp-112] ; load closure code pointer
    mov rdx, [rbp-104] ; load closure env_end pointer
    mov [rsp+16], rax ; stash closure code pointer for clone
    mov rbx, rdx ; original closure env_end pointer
    mov r13, [rbx] ; load env size metadata for clone
    mov r14, [rbx+8] ; load heap size metadata for clone
    mov r12, rbx ; compute env base pointer for clone source
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
    mov rdx, rbx ; use cloned env_end pointer for argument
    mov rax, [rsp+16] ; restore closure code pointer after clone
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 48 ; compute slot for next argument
    mov [rbx], rax ; store closure code for arg
    mov [rbx+8], rdx ; store closure env_end for arg
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov [rbp-128], rax ; update closure code pointer
    mov [rbp-120], rdx ; update closure environment pointer
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 104 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 56 ; bump pointer past env header
    mov qword [rdx], 56 ; env size metadata
    mov qword [rdx+8], 104 ; heap size metadata
    mov qword [rdx+16], 3 ; pointer count metadata
    mov qword [rdx+24], 16 ; closure env pointer slot offset
    mov qword [rdx+32], 32 ; closure env pointer slot offset
    mov qword [rdx+40], 48 ; closure env pointer slot offset
    mov rax, cons_closure_entry ; load wrapper entry point
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
    sub rbx, 56 ; compute slot for next argument
    mov [rbx], rax ; store scalar arg in env
    mov rax, [rbp-128] ; load closure code pointer
    mov rdx, [rbp-120] ; load closure env_end pointer
    mov [rsp+16], rax ; stash closure code pointer for clone
    mov rbx, rdx ; original closure env_end pointer
    mov r13, [rbx] ; load env size metadata for clone
    mov r14, [rbx+8] ; load heap size metadata for clone
    mov r12, rbx ; compute env base pointer for clone source
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
    mov rdx, rbx ; use cloned env_end pointer for argument
    mov rax, [rsp+16] ; restore closure code pointer after clone
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 48 ; compute slot for next argument
    mov [rbx], rax ; store closure code for arg
    mov [rbx+8], rdx ; store closure env_end for arg
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov [rbp-144], rax ; update closure code pointer
    mov [rbp-136], rdx ; update closure environment pointer
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 56 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 24 ; bump pointer past env header
    mov qword [rdx], 24 ; env size metadata
    mov qword [rdx+8], 56 ; heap size metadata
    mov qword [rdx+16], 1 ; pointer count metadata
    mov qword [rdx+24], 16 ; closure env pointer slot offset
    mov rax, handler_closure_entry ; load wrapper entry point
    mov [rbp-160], rax ; update closure code pointer
    mov [rbp-152], rdx ; update closure environment pointer
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
    mov rax, end_closure_entry ; load wrapper entry point
    mov [rbp-176], rax ; update closure code pointer
    mov [rbp-168], rdx ; update closure environment pointer
    mov rax, [rbp-176] ; load closure code pointer
    mov rdx, [rbp-168] ; load closure env_end pointer
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-144] ; load closure code pointer
    mov rdx, [rbp-136] ; load closure env_end pointer
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-160] ; load closure code pointer
    mov rdx, [rbp-152] ; load closure env_end pointer
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    pop rdi ; restore closure code into register
    pop rsi ; restore closure env_end into register
    pop rdx ; restore closure code into register
    pop rcx ; restore closure env_end into register
    pop r8 ; restore closure code into register
    pop r9 ; restore closure env_end into register
    leave ; unwind before named jump
    jmp iterate ; jump to fully applied function
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
extern fflush
extern printf
extern stdout
section .rodata
str_literal_0:
    db "%d, ", 0
str_literal_1:
    db "end", 10, 0
