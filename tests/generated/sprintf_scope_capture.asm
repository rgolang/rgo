bits 64
default rel
section .text
global _8_foo
_8_foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    ; load exit code
    mov rdi, 0 ; exit code
    call exit ; call libc exit to flush buffers
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
global _8_foo_unwrapper
_8_foo_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave ; unwind before named jump
    jmp _8_foo
global _8_foo_deep_release
_8_foo_deep_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _8_foo_deepcopy
_8_foo_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    leave
    ret

global _6_foo
_6_foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store s arg in frame
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
    lea rax, [_8_foo_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_8_foo_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_8_foo_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 0 ; store num_remaining
    mov rax, r12 ; copy _8_foo closure env_end to rax
    mov [rbp-16], rax ; store value
    mov rax, [rbp-8] ; load operand
    push rax ; stack arg
    pop rdi ; restore arg into register
    mov r8, rdi ; keep string pointer
    xor rcx, rcx ; reset length counter
_6_foo_write_strlen_loop_0:
    mov dl, byte [r8+rcx] ; load current character
    cmp dl, 0 ; stop at terminator
    je _6_foo_write_strlen_done_0
    inc rcx ; advance char counter
    jmp _6_foo_write_strlen_loop_0
_6_foo_write_strlen_done_0:
    mov rdx, rcx ; length to write
    mov rsi, r8 ; buffer start
    mov rdi, 1 ; stdout fd
    call write ; invoke libc write
    mov r12, [rbp-16] ; load continuation env_end pointer
    mov rax, [r12+0] ; load continuation entry point
    mov rdi, r12 ; pass env_end pointer to continuation
    leave ; unwind before jumping
    jmp rax
global _6_foo_unwrapper
_6_foo_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-8] ; load s env field
    mov [rbp-16], rax ; store value
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    mov rax, [rbp-16] ; load operand
    push rax ; stack arg
    pop rdi ; restore arg into register
    leave ; unwind before named jump
    jmp _6_foo
global _6_foo_deep_release
_6_foo_deep_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _6_foo_deepcopy
_6_foo_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    leave
    ret

global _3_foo
_3_foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store x arg in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
    mov rsi, 56 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rbx, rax ; closure env base pointer
    mov r12, rbx ; env_end pointer before metadata
    add r12, 8 ; move pointer past env payload
    mov rax, 8 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 56 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [_6_foo_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_6_foo_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_6_foo_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 1 ; store num_remaining
    mov rax, r12 ; copy _6_foo closure env_end to rax
    mov [rbp-16], rax ; store value
    mov rax, [rbp-8] ; load operand
    push rax ; stack arg
    lea rax, [rel _4] ; point to string literal
    push rax ; stack arg
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
    mov rsi, 1024 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rbx, rax ; keep sprintf buffer pointer
    pop rdi ; restore arg into register
    pop rsi ; restore arg into register
    mov rdx, rsi ; shift sprintf args for buffer insertion
    mov rsi, rdi ; shift sprintf args for buffer insertion
    mov rdi, rbx ; destination buffer for sprintf
    push rbp ; helper prologue
    mov rbp, rsp
    push r12
    mov rax, rsp ; align stack for variadic sprintf call
    and rax, 15
    mov r12, rax
    sub rsp, r12
    call sprintf ; invoke libc sprintf
    add rsp, r12
    pop r12
    pop rbp
    mov rax, rbx ; return formatted string pointer
    mov r12, [rbp-16] ; load continuation env_end pointer
    mov [r12-8], rax ; store env field
    mov rax, [r12+0] ; load continuation entry point
    mov rdi, r12 ; pass env_end pointer to continuation
    leave ; unwind before jumping
    jmp rax
global _3_foo_unwrapper
_3_foo_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-8] ; load x env field
    mov [rbp-16], rax ; store value
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    mov rax, [rbp-16] ; load operand
    push rax ; stack arg
    pop rdi ; restore arg into register
    leave ; unwind before named jump
    jmp _3_foo
global _3_foo_deep_release
_3_foo_deep_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _3_foo_deepcopy
_3_foo_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    leave
    ret

global foo
foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
    mov rsi, 56 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rbx, rax ; closure env base pointer
    mov r12, rbx ; env_end pointer before metadata
    add r12, 8 ; move pointer past env payload
    mov rax, 8 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 56 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [_3_foo_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_3_foo_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_3_foo_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 1 ; store num_remaining
    mov rax, r12 ; copy _3_foo closure env_end to rax
    mov [rbp-8], rax ; store value
    mov rax, 1 ; operand literal
    mov rbx, 2 ; operand literal
    add rax, rbx ; add second integer
    mov r12, [rbp-8] ; load continuation env_end pointer
    mov [r12-8], rax ; store env field
    mov rax, [r12+0] ; load continuation entry point
    mov rdi, r12 ; pass env_end pointer to continuation
    leave ; unwind before jumping
    jmp rax
global foo_unwrapper
foo_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave ; unwind before named jump
    jmp foo
global foo_deep_release
foo_deep_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global foo_deepcopy
foo_deepcopy:
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
    leave ; unwind before named jump
    jmp foo
extern exit
extern sprintf
extern write
section .rodata
_4:
    db "result: %d", 0
