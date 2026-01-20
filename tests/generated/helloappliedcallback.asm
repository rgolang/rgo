bits 64
default rel
section .text
global bar
bar:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store m1 arg in frame
    mov [rbp-16], rsi ; store m2 arg in frame
    mov [rbp-24], rdx ; store ok arg in frame
    mov rax, [rbp-16] ; load operand
    push rax ; stack arg
    mov rax, [rbp-8] ; load operand
    push rax ; stack arg
    lea rax, [rel _0] ; point to string literal
    push rax ; stack arg
    pop rdi ; restore arg into register
    pop rsi ; restore arg into register
    pop rdx ; restore arg into register
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
    mov r12, [rbp-24] ; load continuation env_end pointer
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
global bar_unwrapper
bar_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-24] ; load m1 env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-16] ; load m2 env field
    mov [rbp-24], rax ; store value
    mov rax, [r12-8] ; load ok env field
    mov [rbp-32], rax ; store value
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    mov rax, [rbp-32] ; load operand
    push rax ; stack arg
    mov rax, [rbp-24] ; load operand
    push rax ; stack arg
    mov rax, [rbp-16] ; load operand
    push rax ; stack arg
    pop rdi ; restore arg into register
    pop rsi ; restore arg into register
    pop rdx ; restore arg into register
    leave ; unwind before named jump
    jmp bar
global bar_deep_release
bar_deep_release:
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
    jg bar_release_skip_2
    mov rax, [r12-8] ; load bar_release_field_2 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
bar_release_skip_2:
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
global bar_deepcopy
bar_deepcopy:
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
    jg bar_deepcopy_skip_2
    mov rcx, [r12-8] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-8], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
bar_deepcopy_skip_2:
    leave
    ret

global foo
foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store cb arg in frame
    mov [rbp-16], rsi ; store ok arg in frame
    mov rbx, [rbp-8] ; load cb closure env_end pointer
    lea rax, [rel _3] ; point to string literal
    mov [rbx-16], rax ; store env field
    mov rax, [rbp-16] ; load operand
    mov [rbx-8], rax ; store env field
    mov rdi, rbx ; pass env_end pointer to closure
    mov rax, [rdi+0] ; load closure unwrapper entry point
    leave ; unwind before jumping
    jmp rax ; tail call into closure
global foo_unwrapper
foo_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-16] ; load cb env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-8] ; load ok env field
    mov [rbp-24], rax ; store value
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    mov rax, [rbp-24] ; load operand
    push rax ; stack arg
    mov rax, [rbp-16] ; load operand
    push rax ; stack arg
    pop rdi ; restore arg into register
    pop rsi ; restore arg into register
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
    mov rbx, 1 ; operand literal
    cmp rax, rbx
    jg foo_release_skip_0
    mov rax, [r12-16] ; load foo_release_field_0 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
foo_release_skip_0:
    mov rax, [rbp-16] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    jg foo_release_skip_1
    mov rax, [r12-8] ; load foo_release_field_1 env field
    mov [rbp-32], rax ; store value
    mov rdi, [rbp-32] ; load operand
    call release_heap_ptr ; release heap pointer
foo_release_skip_1:
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
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
    mov rbx, 1 ; operand literal
    cmp rax, rbx
    jg foo_deepcopy_skip_0
    mov rcx, [r12-16] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-16], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
foo_deepcopy_skip_0:
    mov rax, [rbp-16] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    jg foo_deepcopy_skip_1
    mov rcx, [r12-8] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-8], rax ; store duplicated pointer
    mov [rbp-32], rax ; store value
foo_deepcopy_skip_1:
    leave
    ret

global _7_lambda
_7_lambda:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    ; load exit code
    mov rdi, 0 ; exit code
    call exit ; call libc exit to flush buffers
global _7_lambda_unwrapper
_7_lambda_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave ; unwind before named jump
    jmp _7_lambda
global _7_lambda_deep_release
_7_lambda_deep_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _7_lambda_deepcopy
_7_lambda_deepcopy:
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
    mov rsi, 72 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rbx, rax ; closure env base pointer
    lea rax, [rel _4] ; point to string literal
    mov [rbx+0], rax ; capture arg into env
    mov r12, rbx ; env_end pointer before metadata
    add r12, 24 ; move pointer past env payload
    mov rax, 24 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 72 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [bar_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [bar_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [bar_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 2 ; store num_remaining
    mov rax, r12 ; copy _5_bar closure env_end to rax
    mov [rbp-8], rax ; store value
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
    lea rax, [_7_lambda_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_7_lambda_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_7_lambda_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 0 ; store num_remaining
    mov rax, r12 ; copy _7_lambda closure env_end to rax
    mov [rbp-16], rax ; store value
    mov rax, [rbp-16] ; load operand
    push rax ; stack arg
    mov rax, [rbp-8] ; load operand
    push rax ; stack arg
    pop rdi ; restore arg into register
    pop rsi ; restore arg into register
    leave ; unwind before named jump
    jmp foo
extern exit
extern printf
section .rodata
_0:
    db "msg1: %s, msg2: %s", 10, 0
_3:
    db "bye", 0
_4:
    db "hi", 0
