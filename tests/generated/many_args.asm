bits 64
default rel
section .text
global foo
foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 192 ; reserve stack space for locals
    mov [rbp-8], rdi ; store a1 arg in frame
    mov [rbp-16], rsi ; store a2 arg in frame
    mov [rbp-24], rdx ; store a3 arg in frame
    mov [rbp-32], rcx ; store a4 arg in frame
    mov [rbp-40], r8 ; store a5 arg in frame
    mov [rbp-48], r9 ; store a6 arg in frame
    mov rax, [rbp+8] ; load spilled a7 arg
    mov [rbp-56], rax ; store spilled arg
    mov rax, [rbp+16] ; load spilled a8 arg
    mov [rbp-64], rax ; store spilled arg
    mov rax, [rbp+24] ; load spilled a9 arg
    mov [rbp-72], rax ; store spilled arg
    mov rax, [rbp+32] ; load spilled a10 arg
    mov [rbp-80], rax ; store spilled arg
    mov rax, [rbp+40] ; load spilled a11 arg
    mov [rbp-88], rax ; store spilled arg
    mov rax, [rbp+48] ; load spilled a12 arg
    mov [rbp-96], rax ; store spilled arg
    mov rax, [rbp+56] ; load spilled a13 arg
    mov [rbp-104], rax ; store spilled arg
    mov rax, [rbp+64] ; load spilled a14 arg
    mov [rbp-112], rax ; store spilled arg
    mov rax, [rbp+72] ; load spilled a15 arg
    mov [rbp-120], rax ; store spilled arg
    mov rax, [rbp+80] ; load spilled a16 arg
    mov [rbp-128], rax ; store spilled arg
    mov rax, [rbp+88] ; load spilled a17 arg
    mov [rbp-136], rax ; store spilled arg
    mov rax, [rbp+96] ; load spilled a18 arg
    mov [rbp-144], rax ; store spilled arg
    mov rax, [rbp+104] ; load spilled a19 arg
    mov [rbp-152], rax ; store spilled arg
    mov rax, [rbp+112] ; load spilled a20 arg
    mov [rbp-160], rax ; store spilled arg
    mov rax, [rbp+120] ; load spilled a21 arg
    mov [rbp-168], rax ; store spilled arg
    mov rax, [rbp+128] ; load spilled a22 arg
    mov [rbp-176], rax ; store spilled arg
    mov rax, [rbp+136] ; load spilled ok arg
    mov [rbp-184], rax ; store spilled arg
    mov rax, [rbp-168] ; load operand
    push rax ; stack arg
    mov rax, [rbp-88] ; load operand
    push rax ; stack arg
    mov rax, [rbp-8] ; load operand
    push rax ; stack arg
    lea rax, [rel _0] ; point to string literal
    push rax ; stack arg
    pop rdi ; restore arg into register
    pop rsi ; restore arg into register
    pop rdx ; restore arg into register
    pop rcx ; restore arg into register
    push rbp ; helper prologue
    mov rbp, rsp
    push r12
    mov rax, rsp ; align stack for variadic printf call
    and rax, 15
    mov r12, rax
    sub rsp, r12
    call printf ; invoke libc printf
    add rsp, r12
    pop r12
    pop rbp
    mov r12, [rbp-184] ; load continuation env_end pointer
    mov rax, [r12+0] ; load continuation entry point
    mov rdi, r12 ; pass env_end pointer to continuation
    leave ; unwind before jumping
    jmp rax
global release_heap_ptr
release_heap_ptr:
    push rbp ; save caller frame
    mov rbp, rsp ; establish frame
    push rbx ; preserve rbx
    mov rbx, rdi ; keep env_end pointer
    mov rcx, [rbx+24] ; load env size metadata
    mov rdx, [rbx+32] ; load heap size metadata
    mov rdi, rbx
    sub rdi, rcx ; compute env base pointer
    mov rsi, rdx ; heap size for munmap
    mov rax, 11 ; munmap syscall
    syscall
    pop rbx
    pop rbp
    ret
global foo_unwrapper
foo_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 192 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-184] ; load a1 env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-176] ; load a2 env field
    mov [rbp-24], rax ; store value
    mov rax, [r12-168] ; load a3 env field
    mov [rbp-32], rax ; store value
    mov rax, [r12-160] ; load a4 env field
    mov [rbp-40], rax ; store value
    mov rax, [r12-152] ; load a5 env field
    mov [rbp-48], rax ; store value
    mov rax, [r12-144] ; load a6 env field
    mov [rbp-56], rax ; store value
    mov rax, [r12-136] ; load a7 env field
    mov [rbp-64], rax ; store value
    mov rax, [r12-128] ; load a8 env field
    mov [rbp-72], rax ; store value
    mov rax, [r12-120] ; load a9 env field
    mov [rbp-80], rax ; store value
    mov rax, [r12-112] ; load a10 env field
    mov [rbp-88], rax ; store value
    mov rax, [r12-104] ; load a11 env field
    mov [rbp-96], rax ; store value
    mov rax, [r12-96] ; load a12 env field
    mov [rbp-104], rax ; store value
    mov rax, [r12-88] ; load a13 env field
    mov [rbp-112], rax ; store value
    mov rax, [r12-80] ; load a14 env field
    mov [rbp-120], rax ; store value
    mov rax, [r12-72] ; load a15 env field
    mov [rbp-128], rax ; store value
    mov rax, [r12-64] ; load a16 env field
    mov [rbp-136], rax ; store value
    mov rax, [r12-56] ; load a17 env field
    mov [rbp-144], rax ; store value
    mov rax, [r12-48] ; load a18 env field
    mov [rbp-152], rax ; store value
    mov rax, [r12-40] ; load a19 env field
    mov [rbp-160], rax ; store value
    mov rax, [r12-32] ; load a20 env field
    mov [rbp-168], rax ; store value
    mov rax, [r12-24] ; load a21 env field
    mov [rbp-176], rax ; store value
    mov rax, [r12-16] ; load a22 env field
    mov [rbp-184], rax ; store value
    mov rax, [r12-8] ; load ok env field
    mov [rbp-192], rax ; store value
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    mov rax, [rbp-192] ; load operand
    push rax ; stack arg
    mov rax, [rbp-184] ; load operand
    push rax ; stack arg
    mov rax, [rbp-176] ; load operand
    push rax ; stack arg
    mov rax, [rbp-168] ; load operand
    push rax ; stack arg
    mov rax, [rbp-160] ; load operand
    push rax ; stack arg
    mov rax, [rbp-152] ; load operand
    push rax ; stack arg
    mov rax, [rbp-144] ; load operand
    push rax ; stack arg
    mov rax, [rbp-136] ; load operand
    push rax ; stack arg
    mov rax, [rbp-128] ; load operand
    push rax ; stack arg
    mov rax, [rbp-120] ; load operand
    push rax ; stack arg
    mov rax, [rbp-112] ; load operand
    push rax ; stack arg
    mov rax, [rbp-104] ; load operand
    push rax ; stack arg
    mov rax, [rbp-96] ; load operand
    push rax ; stack arg
    mov rax, [rbp-88] ; load operand
    push rax ; stack arg
    mov rax, [rbp-80] ; load operand
    push rax ; stack arg
    mov rax, [rbp-72] ; load operand
    push rax ; stack arg
    mov rax, [rbp-64] ; load operand
    push rax ; stack arg
    mov rax, [rbp-56] ; load operand
    push rax ; stack arg
    mov rax, [rbp-48] ; load operand
    push rax ; stack arg
    mov rax, [rbp-40] ; load operand
    push rax ; stack arg
    mov rax, [rbp-32] ; load operand
    push rax ; stack arg
    mov rax, [rbp-24] ; load operand
    push rax ; stack arg
    mov rax, [rbp-16] ; load operand
    push rax ; stack arg
    pop rdi ; restore arg into register
    pop rsi ; restore arg into register
    pop rdx ; restore arg into register
    pop rcx ; restore arg into register
    pop r8 ; restore arg into register
    pop r9 ; restore arg into register
    sub rsp, 8 ; allocate slot for saved rbp
    mov rax, [rbp] ; capture parent rbp
    mov [rsp], rax ; stash parent rbp for leave
    mov rbp, rsp ; treat slot as current rbp
    leave ; unwind before named jump
    jmp foo
global foo_deep_release
foo_deep_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12+40] ; load __num_remaining env field
    mov [rbp-16], rax ; store value
    mov rax, [rbp-16] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    jg foo_release_skip_22
    mov rax, [r12-8] ; load foo_release_field_22 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
foo_release_skip_22:
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global deepcopy_heap_ptr
deepcopy_heap_ptr:
    push rbp ; prologue: save executor frame pointer
    mov rbp, rsp ; prologue: establish new frame
    push rbx ; preserve callee-saved registers
    push r12
    push r13
    push r14
    push r15
    mov r12, rdi ; capture env_end pointer
    mov r14, [r12+24] ; load env size metadata
    mov r15, [r12+32] ; load heap size metadata
    mov rbx, r12 ; keep env_end pointer
    sub rbx, r14 ; compute env base pointer
    mov rdi, 0 ; addr hint so kernel picks mmap base
    mov rsi, r15 ; length = heap size
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags = private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    mov rax, 9 ; mmap syscall
    syscall ; allocate new closure env
    mov r13, rax ; new env base pointer
    mov rdi, r13 ; memcpy dest
    mov rsi, rbx ; memcpy src
    mov rdx, r15 ; memcpy length
    call memcpy_helper ; copy env contents
    mov rax, r13 ; compute new env_end pointer
    add rax, r14
    mov r15, rax ; preserve new env_end pointer
    mov rax, [r15+16] ; load deep copy helper entry
    mov rdi, r15 ; pass new env_end pointer
    call rax ; invoke helper
    mov rax, r15 ; return new env_end pointer
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    ret
global memcpy_helper
memcpy_helper:
    push rbp ; prologue
    mov rbp, rsp
    xor rcx, rcx ; counter = 0
internal_memcpy_loop:
    cmp rcx, rdx ; counter < count?
    jge internal_memcpy_done
    mov rax, [rsi+rcx] ; load 8 bytes from source
    mov [rdi+rcx], rax ; store 8 bytes to destination
    add rcx, 8 ; advance counter by 8
    jmp internal_memcpy_loop
internal_memcpy_done:
    pop rbp
    ret
global foo_deepcopy
foo_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12+40] ; load num_remaining env field
    mov [rbp-16], rax ; store value
    mov rax, [rbp-16] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    jg foo_deepcopy_skip_22
    mov rcx, [r12-8] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-8], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
foo_deepcopy_skip_22:
    leave
    ret

global _24_lambda
_24_lambda:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    ; load exit code
    mov rdi, 0 ; exit code
    call exit ; call libc exit to flush buffers
global _24_lambda_unwrapper
_24_lambda_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave ; unwind before named jump
    jmp _24_lambda
global _24_lambda_deep_release
_24_lambda_deep_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _24_lambda_deepcopy
_24_lambda_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    leave
    ret

global _start
_start:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
    mov rsi, 48 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rbx, rax ; closure env base pointer
    mov r12, rbx ; env_end pointer before metadata
    mov rax, 0 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 48 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [_24_lambda_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_24_lambda_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_24_lambda_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 0 ; store num_remaining
    mov rax, r12 ; copy _24_lambda closure env_end to rax
    mov [rbp-8], rax ; store value
    mov rax, [rbp-8] ; load operand
    push rax ; stack arg
    mov rax, 22 ; operand literal
    push rax ; stack arg
    mov rax, 21 ; operand literal
    push rax ; stack arg
    mov rax, 20 ; operand literal
    push rax ; stack arg
    mov rax, 19 ; operand literal
    push rax ; stack arg
    mov rax, 18 ; operand literal
    push rax ; stack arg
    mov rax, 17 ; operand literal
    push rax ; stack arg
    mov rax, 16 ; operand literal
    push rax ; stack arg
    mov rax, 15 ; operand literal
    push rax ; stack arg
    mov rax, 14 ; operand literal
    push rax ; stack arg
    mov rax, 13 ; operand literal
    push rax ; stack arg
    mov rax, 12 ; operand literal
    push rax ; stack arg
    mov rax, 11 ; operand literal
    push rax ; stack arg
    mov rax, 10 ; operand literal
    push rax ; stack arg
    mov rax, 9 ; operand literal
    push rax ; stack arg
    mov rax, 8 ; operand literal
    push rax ; stack arg
    mov rax, 7 ; operand literal
    push rax ; stack arg
    mov rax, 6 ; operand literal
    push rax ; stack arg
    mov rax, 5 ; operand literal
    push rax ; stack arg
    mov rax, 4 ; operand literal
    push rax ; stack arg
    mov rax, 3 ; operand literal
    push rax ; stack arg
    mov rax, 2 ; operand literal
    push rax ; stack arg
    mov rax, 1 ; operand literal
    push rax ; stack arg
    pop rdi ; restore arg into register
    pop rsi ; restore arg into register
    pop rdx ; restore arg into register
    pop rcx ; restore arg into register
    pop r8 ; restore arg into register
    pop r9 ; restore arg into register
    sub rsp, 8 ; allocate slot for saved rbp
    mov rax, [rbp] ; capture parent rbp
    mov [rsp], rax ; stash parent rbp for leave
    mov rbp, rsp ; treat slot as current rbp
    leave ; unwind before named jump
    jmp foo
extern exit
extern printf
section .rodata
_0:
    db "a1: %d, a11: %d, a21: %d", 10, 0
