bits 64
default rel
section .text
global err
err:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    lea rax, [rel str_literal_0] ; point to string literal
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
    mov rax, _err_2_closure_entry ; load wrapper entry point
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
    mov r8, rdi ; keep string pointer
    xor rcx, rcx ; reset length counter
err_write_strlen_loop_0:
    mov dl, byte [r8+rcx] ; load current character
    cmp dl, 0 ; stop at terminator
    je err_write_strlen_done_0
    inc rcx ; advance char counter
    jmp err_write_strlen_loop_0
err_write_strlen_done_0:
    mov rsi, r8 ; buffer start
    mov rdx, rcx ; length to write
    mov rdi, 1 ; stdout fd
    call write ; invoke libc write
    pop rdx ; restore continuation env_end pointer
    pop rax ; restore continuation code pointer
    mov rdi, rdx ; pass env_end pointer to continuation
    leave ; unwind before jumping
    jmp rax ; jump into continuation
err_closure_entry:
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
    jmp err ; jump into actual function
global _err_2
_err_2:
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
_err_2_closure_entry:
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
    jmp _err_2 ; jump into actual function
global array
array:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-16], rdi ; save closure code pointer
    mov [rbp-8], rsi ; save closure environment pointer
    mov [rbp-32], rdx ; save closure code pointer
    mov [rbp-24], rcx ; save closure environment pointer
    mov rax, [rbp-16] ; load closure code for call
    mov rdx, [rbp-8] ; load closure env_end for call
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
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
array_closure_entry:
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
    jmp array ; jump into actual function
global __4___4_1_2
__4___4_1_2:
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
__4___4_1_2_closure_entry:
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
    jmp __4___4_1_2 ; jump into actual function
global __4_1
__4_1:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov [rbp-32], rsi ; store scalar arg in frame
    lea rax, [rel str_literal_1] ; point to string literal
    mov [rbp-48], rax ; save evaluated scalar in frame
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
    mov rax, __4___4_1_2_closure_entry ; load wrapper entry point
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov [rbp-64], rax ; update closure code pointer
    mov [rbp-56], rdx ; update closure environment pointer
    mov rax, [rbp-64] ; load closure code pointer
    mov rdx, [rbp-56] ; load closure env_end pointer
    push rax ; preserve continuation code pointer
    push rdx ; preserve continuation env_end pointer
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-16] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-48] ; load scalar from frame
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
__4_1_closure_entry:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish wrapper frame
    sub rsp, 16 ; reserve space for env metadata scratch
    mov [rbp-8], rdi ; stash env_end pointer for release
    push rbx ; preserve base register
    mov rbx, rdi ; rdi points to env_end when invoked
    sub rbx, 16 ; compute env base
    mov rdi, [rbx+0] ; load scalar param from env
    push rdi ; preserve parameter register
    mov rsi, [rbx+8] ; load scalar param from env
    push rsi ; preserve parameter register
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
    jmp __4_1 ; jump into actual function
global _4
_4:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 80 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov [rbp-32], rsi ; save closure code pointer
    mov [rbp-24], rdx ; save closure environment pointer
    mov rax, 1 ; load literal integer
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 40 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 16 ; bump pointer past env header
    mov qword [rdx], 16 ; env size metadata
    mov qword [rdx+8], 40 ; heap size metadata
    mov qword [rdx+16], 0 ; pointer count metadata
    mov rax, __4_1_closure_entry ; load wrapper entry point
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
    sub rbx, 16 ; compute slot for next argument
    mov [rbx], rax ; store scalar arg in env
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov [rbp-64], rax ; update closure code pointer
    mov [rbp-56], rdx ; update closure environment pointer
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
    mov rax, err_closure_entry ; load wrapper entry point
    mov [rbp-80], rax ; update closure code pointer
    mov [rbp-72], rdx ; update closure environment pointer
    mov rax, [rbp-32] ; load closure code for call
    mov rdx, [rbp-24] ; load closure env_end for call
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rax, [rbp-48] ; load scalar from frame
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 40 ; compute slot for next argument
    mov [rbx], rax ; store scalar arg in env
    mov rax, [rbp-80] ; load closure code pointer
    mov rdx, [rbp-72] ; load closure env_end pointer
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 32 ; compute slot for next argument
    mov [rbx], rax ; store closure code for arg
    mov [rbx+8], rdx ; store closure env_end for arg
    mov rax, [rbp-64] ; load closure code pointer
    mov rdx, [rbp-56] ; load closure env_end pointer
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
_4_closure_entry:
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
    jmp _4 ; jump into actual function
global _start
_start:
    push rbp ; save caller frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    lea rax, [rel str_literal_2] ; point to string literal
    mov [rbp-16], rax ; save evaluated scalar in frame
    lea rax, [rel str_literal_3] ; point to string literal
    mov [rbp-32], rax ; save evaluated scalar in frame
    lea rax, [rel str_literal_4] ; point to string literal
    mov [rbp-48], rax ; save evaluated scalar in frame
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
    mov rax, _4_closure_entry ; load wrapper entry point
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov [rbp-64], rax ; update closure code pointer
    mov [rbp-56], rdx ; update closure environment pointer
    mov rax, [rbp-64] ; load closure code pointer
    mov rdx, [rbp-56] ; load closure env_end pointer
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
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
    pop rdx ; restore closure code into register
    pop rcx ; restore closure env_end into register
    leave ; unwind before named jump
    jmp array ; jump to fully applied function
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
extern fflush
extern printf
extern stdout
extern write
section .rodata
str_literal_0:
    db "none", 0
str_literal_1:
    db "item at %d: %s", 0
str_literal_2:
    db "a", 0
str_literal_3:
    db "b", 0
str_literal_4:
    db "c", 0
