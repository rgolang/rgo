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
    mov rdi, 0 ; addr NULL so kernel picks mmap base
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
    call internal_memcpy ; copy env contents
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
internal_memcpy:
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
global _4_foo
_4_foo:
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
global _4_foo_unwrapper
_4_foo_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rdi, [rbp-16] ; load closure env_end pointer
    call release_heap_ptr ; release closure environment
    leave ; unwind before named jump
    jmp _4_foo ; jump to fully applied function
global _4_foo_release
_4_foo_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    leave
    ret

global _4_foo_deepcopy
_4_foo_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    leave
    ret

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
    mov rdi, [rbp-16] ; load closure env_end pointer
    call release_heap_ptr ; release closure environment
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    leave ; unwind before named jump
    jmp exit ; jump to fully applied function
global foo
foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    mov [rbp-16], rsi ; save closure env_end pointer
    lea rax, [rel _2] ; point to string literal
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 48 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    mov qword [rdx+24], 0 ; env size metadata
    mov qword [rdx+32], 48 ; heap size metadata
    mov rax, _4_foo_unwrapper ; load unwrapper entry point
    mov qword [rdx+0], rax ; store unwrapper entry in metadata
    mov rax, _4_foo_release ; load release helper entry point
    mov qword [rdx+8], rax ; store release pointer in metadata
    mov rax, _4_foo_deepcopy ; load deep copy helper entry point
    mov qword [rdx+16], rax ; store deep copy pointer in metadata
    xor rax, rax ; zero num_curried metadata
    mov qword [rdx+40], rax ; store num_curried
    mov [rbp-48], rdx ; update closure env_end pointer
    mov rax, [rbp-32] ; load scalar from frame
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
    mov [rbp-64], rax ; save evaluated scalar in frame
    mov rdi, [rbp-16] ; load closure env_end pointer
    call release_heap_ptr ; release closure environment
    mov rdx, [rbp-48] ; load closure env_end for exec
    mov rax, [rdx+0] ; load closure unwrapper entry point
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
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rdx, [r10-8] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rbp-32], rdx ; update closure env_end pointer
    mov rdi, [rbp-16] ; load closure env_end pointer
    call release_heap_ptr ; release closure environment
    mov rdx, [rbp-32] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    pop rdi ; restore closure code into register
    pop rsi ; restore closure env_end into register
    leave ; unwind before named jump
    jmp foo ; jump to fully applied function
global foo_release
foo_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax+40] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load operand
    mov rbx, 1 ; operand literal
    cmp rax, rbx
    jl foo_release_if_num_curried_lt_1
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-8] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    jmp foo_release_done
foo_release_if_num_curried_lt_1:
foo_release_done:
    leave
    ret

global foo_deepcopy
foo_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax+40] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load operand
    mov rbx, 1 ; operand literal
    cmp rax, rbx
    jl foo_deepcopy_if_num_curried_lt_1
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-8] ; load field pointer
    test rcx, rcx ; skip null field
    je foo_deepcopy_copy_field_null_0
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-8], rax ; store duplicated pointer
    jmp foo_deepcopy_copy_field_done_1
foo_deepcopy_copy_field_null_0:
    xor rax, rax ; null copy result
foo_deepcopy_copy_field_done_1:
    mov [rbp-48], rax ; save evaluated scalar in frame
    jmp foo_deepcopy_done
foo_deepcopy_if_num_curried_lt_1:
foo_deepcopy_done:
    leave
    ret

extern deepcopy_heap_ptr
extern release_heap_ptr
extern write
section .rodata
_2:
    db "done", 0
