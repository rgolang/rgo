bits 64
default rel
section .text
global foo
foo:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-16], rdi ; save closure code pointer
    mov [rbp-8], rsi ; save closure environment pointer
    mov rax, 0 ; load literal integer
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rdi, [rbp-8] ; load closure env_end pointer
    call internal_release_env ; release closure environment
    mov rax, [rbp-32] ; load scalar from frame
    mov rdi, rax ; pass exit code
    leave ; unwind before exit
    mov rax, 60 ; exit syscall
    syscall ; exit program
foo_closure_entry:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish wrapper frame
    sub rsp, 16 ; reserve space for env metadata scratch
    mov [rbp-8], rdi ; stash env_end pointer for release
    push rbx ; preserve base register
    mov rbx, rdi ; rdi points to env_end when invoked
    sub rbx, 16 ; compute env base
    mov rdi, [rbx+0] ; load continuation code pointer
    push rdi ; preserve closure code register
    mov rsi, [rbx+8] ; load continuation env_end pointer
    push rsi ; preserve closure env_end register
    mov rdx, [rbp-8] ; load saved env_end pointer
    mov rcx, [rdx] ; read env size metadata
    mov rsi, [rdx+8] ; read heap size metadata
    mov rbx, rdx ; env_end pointer for release
    sub rbx, rcx ; compute env base pointer
    mov rdi, rbx ; munmap base pointer
    mov rax, 11 ; munmap syscall
    syscall ; release wrapper closure environment
    pop rsi ; restore parameter register
    pop rdi ; restore parameter register
    pop rbx ; restore saved base register
    leave ; epilogue: restore rbp of caller
    jmp foo ; jump into actual function
global _start
_start:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    lea rax, [rel str_literal_0] ; point to string literal
    mov [rbp-16], rax ; save evaluated scalar in frame
    lea rax, [rel str_literal_1] ; point to string literal
    mov [rbp-32], rax ; save evaluated scalar in frame
    lea rax, [rel str_literal_2] ; point to string literal
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 104 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov r14, rax ; stash base pointer for variadic array
    mov rax, [rbp-16] ; load scalar from frame
    mov [r14+0], rax ; store variadic argument '_0'
    mov rax, [rbp-32] ; load scalar from frame
    mov [r14+8], rax ; store variadic argument '_1'
    mov rax, [rbp-48] ; load scalar from frame
    mov [r14+16], rax ; store variadic argument '_2'
    mov qword [r14+24], 3 ; record variadic argument length
    mov rdx, r14 ; env base pointer for array
    add rdx, 72 ; env_end pointer for array closure
    mov qword [rdx], 72 ; env size metadata for array
    mov qword [rdx+8], 104 ; heap size metadata for array
    mov qword [rdx+16], 0 ; pointer count metadata for array
    mov qword [rdx+24], 40 ; exec slot metadata for array
    mov rax, internal_array_str ; builtin array closure entry
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    pop rdi ; restore closure code into register
    pop rsi ; restore closure env_end into register
    leave ; unwind before named jump
    jmp foo ; jump to fully applied function
global internal_array_str_nth
internal_array_str_nth:
    push rbp ; prologue: save caller frame pointer
    mov rbp, rsp ; prologue: establish new frame
    mov r10, rdi ; keep env_end pointer for later
    mov r8, [r10] ; load env size metadata
    mov rax, [r10+16] ; pointer count metadata
    imul rax, 8 ; pointer metadata byte width
    lea r9, [r10+24] ; pointer metadata base
    add r9, rax ; offset to array extras
    mov r9, [r9] ; load exec slot size
    mov r11, r10 ; copy metadata pointer
    sub r11, r8 ; compute env base pointer
    mov rax, r8 ; payload plus slot bytes
    sub rax, r9 ; isolate payload size
    mov rcx, r11 ; start from env base
    add rcx, rax ; advance to payload end
    sub rcx, 8 ; locate stored array length
    mov rdx, [rcx] ; load array length
    mov rax, [r10-40] ; requested index argument
    cmp rax, 0 ; disallow negative indexes
    jl internal_array_str_nth_oob
    cmp rax, rdx ; ensure idx < len
    jge internal_array_str_nth_oob
    imul rax, 8 ; stride by element size
    add rax, r11 ; locate element slot
    mov rax, [rax] ; load string pointer
    mov rsi, [r10-16] ; load 'one' continuation code
    mov rdx, [r10-8] ; load 'one' continuation env_end
    sub rsp, 16 ; allocate temp stack for closure state
    mov [rsp], rsi ; save closure code pointer
    mov [rsp+8], rdx ; save closure env_end pointer
    mov rcx, [rsp+8] ; env_end pointer for argument
    sub rcx, 8 ; slot for string argument
    mov [rcx], rax ; store selected element
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 16 ; drop temp state
    mov rdi, rdx ; pass env_end to continuation
    leave ; epilogue before jump
    jmp rax ; return into 'one' continuation
internal_array_str_nth_oob:
    mov rax, [r10-32] ; load 'none' continuation code
    mov rdx, [r10-24] ; load 'none' continuation env_end
    mov rdi, rdx ; pass env_end pointer
    leave ; epilogue before jump
    jmp rax ; return into 'none' continuation
global internal_array_str
internal_array_str:
    push rbp ; prologue: save caller frame pointer
    mov rbp, rsp ; prologue: establish new frame
    mov r10, rdi ; capture env_end pointer
    mov r8, [r10] ; load env size metadata
    mov rax, [r10+16] ; pointer count metadata
    imul rax, 8 ; pointer metadata byte width
    lea r9, [r10+24] ; pointer metadata base
    add r9, rax ; offset to array extras
    mov r9, [r9] ; load exec slot size
    mov r11, r10 ; duplicate pointer
    sub r11, r8 ; compute env base
    mov rax, r8 ; payload plus slot bytes
    sub rax, r9 ; isolate payload size
    mov rcx, r11 ; start from env base
    add rcx, rax ; advance to payload end
    sub rcx, 8 ; locate stored array length
    mov r9, [rcx] ; load array length
    mov rax, [r10-16] ; load 'ok' continuation code
    mov rdx, [r10-8] ; load 'ok' continuation env_end
    sub rsp, 16 ; allocate temp stack for closure state
    mov [rsp], rax ; save closure code pointer
    mov [rsp+8], rdx ; save closure env_end pointer
    mov rsi, [rsp+8] ; env_end pointer for args
    sub rsi, 24 ; slot for len argument
    mov [rsi], r9 ; write len argument
    mov rsi, [rsp+8] ; env_end pointer for args
    sub rsi, 16 ; slot for nth continuation
    mov qword [rsi], internal_array_str_nth ; install nth code
    mov [rsi+8], r10 ; install nth env_end pointer
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 16 ; drop temp stack
    mov rdi, rdx ; pass env_end pointer
    leave ; epilogue before jump
    jmp rax ; return into 'ok' continuation
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
section .rodata
str_literal_0:
    db "a", 0
str_literal_1:
    db "b", 0
str_literal_2:
    db "c", 0
