bits 64
default rel
section .text
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
global nil
nil:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store cb arg in frame
    mov [rbp-16], rsi ; store end arg in frame
    mov rdi, [rbp-8] ; load cb closure env_end pointer
    call release_heap_ptr ; release cb closure environment
    mov rbx, [rbp-16] ; load end closure env_end pointer
    mov rdi, rbx ; pass env_end pointer to closure
    mov rax, [rdi+0] ; load closure unwrapper entry point
    leave ; unwind before jumping
    jmp rax ; tail call into closure
global nil_unwrapper
nil_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-16] ; load cb env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-8] ; load end env field
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
    jmp nil
global nil_deep_release
nil_deep_release:
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
    jg nil_release_skip_0
    mov rax, [r12-16] ; load nil_release_field_0 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
nil_release_skip_0:
    mov rax, [rbp-16] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    jg nil_release_skip_1
    mov rax, [r12-8] ; load nil_release_field_1 env field
    mov [rbp-32], rax ; store value
    mov rdi, [rbp-32] ; load operand
    call release_heap_ptr ; release heap pointer
nil_release_skip_1:
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
global nil_deepcopy
nil_deepcopy:
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
    jg nil_deepcopy_skip_0
    mov rcx, [r12-16] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-16], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
nil_deepcopy_skip_0:
    mov rax, [rbp-16] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    jg nil_deepcopy_skip_1
    mov rcx, [r12-8] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-8], rax ; store duplicated pointer
    mov [rbp-32], rax ; store value
nil_deepcopy_skip_1:
    leave
    ret

global cons
cons:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store h arg in frame
    mov [rbp-16], rsi ; store t arg in frame
    mov [rbp-24], rdx ; store cb arg in frame
    mov [rbp-32], rcx ; store end arg in frame
    mov rdi, [rbp-32] ; load end closure env_end pointer
    call release_heap_ptr ; release end closure environment
    mov rbx, [rbp-24] ; load cb closure env_end pointer
    mov rax, [rbp-8] ; load operand
    mov [rbx-16], rax ; store env field
    mov rax, [rbp-16] ; load operand
    mov [rbx-8], rax ; store env field
    mov rdi, rbx ; pass env_end pointer to closure
    mov rax, [rdi+0] ; load closure unwrapper entry point
    leave ; unwind before jumping
    jmp rax ; tail call into closure
global cons_unwrapper
cons_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-32] ; load h env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-24] ; load t env field
    mov [rbp-24], rax ; store value
    mov rax, [r12-16] ; load cb env field
    mov [rbp-32], rax ; store value
    mov rax, [r12-8] ; load end env field
    mov [rbp-40], rax ; store value
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
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
    leave ; unwind before named jump
    jmp cons
global cons_deep_release
cons_deep_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12+40] ; load __num_remaining env field
    mov [rbp-16], rax ; store value
    mov rax, [rbp-16] ; load operand
    mov rbx, 2 ; operand literal
    cmp rax, rbx
    jg cons_release_skip_1
    mov rax, [r12-24] ; load cons_release_field_1 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
cons_release_skip_1:
    mov rax, [rbp-16] ; load operand
    mov rbx, 1 ; operand literal
    cmp rax, rbx
    jg cons_release_skip_2
    mov rax, [r12-16] ; load cons_release_field_2 env field
    mov [rbp-32], rax ; store value
    mov rdi, [rbp-32] ; load operand
    call release_heap_ptr ; release heap pointer
cons_release_skip_2:
    mov rax, [rbp-16] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    jg cons_release_skip_3
    mov rax, [r12-8] ; load cons_release_field_3 env field
    mov [rbp-40], rax ; store value
    mov rdi, [rbp-40] ; load operand
    call release_heap_ptr ; release heap pointer
cons_release_skip_3:
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global cons_deepcopy
cons_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12+40] ; load num_remaining env field
    mov [rbp-16], rax ; store value
    mov rax, [rbp-16] ; load operand
    mov rbx, 2 ; operand literal
    cmp rax, rbx
    jg cons_deepcopy_skip_1
    mov rcx, [r12-24] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-24], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
cons_deepcopy_skip_1:
    mov rax, [rbp-16] ; load operand
    mov rbx, 1 ; operand literal
    cmp rax, rbx
    jg cons_deepcopy_skip_2
    mov rcx, [r12-16] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-16], rax ; store duplicated pointer
    mov [rbp-32], rax ; store value
cons_deepcopy_skip_2:
    mov rax, [rbp-16] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    jg cons_deepcopy_skip_3
    mov rcx, [r12-8] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-8], rax ; store duplicated pointer
    mov [rbp-40], rax ; store value
cons_deepcopy_skip_3:
    leave
    ret

