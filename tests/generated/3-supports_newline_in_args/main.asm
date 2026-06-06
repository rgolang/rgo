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
global as_int
as_int:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store x arg in frame
    mov [rbp-16], rsi ; store ok arg in frame
    mov [rbp-24], rdx ; store _5_as_int arg in frame
    mov rdi, [rbp-24] ; load _5_as_int closure env_end pointer
    call release_heap_ptr ; release _5_as_int closure environment
    mov rbx, [rbp-16] ; load ok closure env_end pointer
    mov rax, [rbp-8] ; load operand
    mov [rbx-8], rax ; store env field
    mov rdi, rbx ; pass env_end pointer to closure
    mov rax, [rdi+0] ; load closure unwrapper entry point
    leave ; unwind before jumping
    jmp rax ; tail call into closure
global as_int_unwrapper
as_int_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-24] ; load x env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-16] ; load ok env field
    mov [rbp-24], rax ; store value
    mov rax, [r12-8] ; load _5_as_int env field
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
    jmp as_int
global as_int_deep_release
as_int_deep_release:
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
    jg as_int_release_skip_1
    mov rax, [r12-16] ; load as_int_release_field_1 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
as_int_release_skip_1:
    mov rax, [rbp-16] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    jg as_int_release_skip_2
    mov rax, [r12-8] ; load as_int_release_field_2 env field
    mov [rbp-32], rax ; store value
    mov rdi, [rbp-32] ; load operand
    call release_heap_ptr ; release heap pointer
as_int_release_skip_2:
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
global as_int_deepcopy
as_int_deepcopy:
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
    jg as_int_deepcopy_skip_1
    mov rcx, [r12-16] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-16], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
as_int_deepcopy_skip_1:
    mov rax, [rbp-16] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    jg as_int_deepcopy_skip_2
    mov rcx, [r12-8] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-8], rax ; store duplicated pointer
    mov [rbp-32], rax ; store value
as_int_deepcopy_skip_2:
    leave
    ret

global _28_main
_28_main:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    ; load exit code
    mov rdi, 0 ; exit code
    call exit ; call libc exit to flush buffers
global _28_main_unwrapper
_28_main_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave ; unwind before named jump
    jmp _28_main
global _28_main_deep_release
_28_main_deep_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _28_main_deepcopy
_28_main_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    leave
    ret

global _15_foo
_15_foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store ok arg in frame
    mov [rbp-16], rsi ; store s arg in frame
    mov rax, [rbp-16] ; load operand
    push rax ; stack arg
    pop rdi ; restore arg into register
    mov r8, rdi ; keep string pointer
    xor rcx, rcx ; reset length counter
_15_foo_write_strlen_loop_0:
    mov dl, byte [r8+rcx] ; load current character
    cmp dl, 0 ; stop at terminator
    je _15_foo_write_strlen_done_0
    inc rcx ; advance char counter
    jmp _15_foo_write_strlen_loop_0
_15_foo_write_strlen_done_0:
    mov rdx, rcx ; length to write
    mov rsi, r8 ; buffer start
    mov rdi, 1 ; stdout fd
    call write ; invoke libc write
    mov r12, [rbp-8] ; load continuation env_end pointer
    mov rax, [r12+0] ; load continuation entry point
    mov rdi, r12 ; pass env_end pointer to continuation
    leave ; unwind before jumping
    jmp rax
global _15_foo_unwrapper
_15_foo_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-16] ; load ok env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-8] ; load s env field
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
    jmp _15_foo
global _15_foo_deep_release
_15_foo_deep_release:
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
    jg _15_foo_release_skip_0
    mov rax, [r12-16] ; load _15_foo_release_field_0 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
_15_foo_release_skip_0:
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _15_foo_deepcopy
_15_foo_deepcopy:
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
    jg _15_foo_deepcopy_skip_0
    mov rcx, [r12-16] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-16], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
_15_foo_deepcopy_skip_0:
    leave
    ret

global _12_foo
_12_foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store ok arg in frame
    mov [rbp-16], rsi ; store x arg in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
    mov rsi, 64 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rbx, rax ; closure env base pointer
    mov rax, [rbp-8] ; load operand
    mov r12, rax ; shadow closure env_end pointer
    push rbx ; save env base pointer
    mov rbx, r12 ; clone source env_end pointer
    mov r13, [rbx+24] ; load env size metadata for clone
    mov r14, [rbx+32] ; load heap size metadata for clone
    mov r12, rbx ; compute env base pointer for clone
    sub r12, r13 ; env base pointer for clone source
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
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
    mov r12, rbx ; cloned env_end pointer
    mov rax, [r12+16] ; load deepcopy helper entry point
    push r12 ; preserve cloned env_end pointer
    mov rdi, r12 ; pass env_end pointer to deepcopy helper
    call rax ; deepcopy reference fields
    pop r12 ; restore cloned env_end pointer
    mov rax, r12 ; copy closure env_end to rax
    pop rbx ; restore env base pointer
    mov [rbx+0], r12 ; capture cloned closure pointer
    mov r12, rbx ; env_end pointer before metadata
    add r12, 16 ; move pointer past env payload
    mov rax, 16 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 64 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [_15_foo_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_15_foo_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_15_foo_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 1 ; store num_remaining
    mov rax, r12 ; copy _16_foo closure env_end to rax
    mov [rbp-24], rax ; store value
    mov rax, [rbp-16] ; load operand
    push rax ; stack arg
    lea rax, [rel _13] ; point to string literal
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
    mov r12, [rbp-24] ; load continuation env_end pointer
    mov [r12-8], rax ; store env field
    mov rax, [r12+0] ; load continuation entry point
    mov rdi, r12 ; pass env_end pointer to continuation
    leave ; unwind before jumping
    jmp rax
global _12_foo_unwrapper
_12_foo_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-16] ; load ok env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-8] ; load x env field
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
    jmp _12_foo
global _12_foo_deep_release
_12_foo_deep_release:
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
    jg _12_foo_release_skip_0
    mov rax, [r12-16] ; load _12_foo_release_field_0 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
_12_foo_release_skip_0:
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _12_foo_deepcopy
_12_foo_deepcopy:
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
    jg _12_foo_deepcopy_skip_0
    mov rcx, [r12-16] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-16], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
_12_foo_deepcopy_skip_0:
    leave
    ret

global _22_foo
_22_foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store ok arg in frame
    mov [rbp-16], rsi ; store s arg in frame
    mov rax, [rbp-16] ; load operand
    push rax ; stack arg
    pop rdi ; restore arg into register
    mov r8, rdi ; keep string pointer
    xor rcx, rcx ; reset length counter
_22_foo_write_strlen_loop_0:
    mov dl, byte [r8+rcx] ; load current character
    cmp dl, 0 ; stop at terminator
    je _22_foo_write_strlen_done_0
    inc rcx ; advance char counter
    jmp _22_foo_write_strlen_loop_0
_22_foo_write_strlen_done_0:
    mov rdx, rcx ; length to write
    mov rsi, r8 ; buffer start
    mov rdi, 1 ; stdout fd
    call write ; invoke libc write
    mov r12, [rbp-8] ; load continuation env_end pointer
    mov rax, [r12+0] ; load continuation entry point
    mov rdi, r12 ; pass env_end pointer to continuation
    leave ; unwind before jumping
    jmp rax
global _22_foo_unwrapper
_22_foo_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-16] ; load ok env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-8] ; load s env field
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
    jmp _22_foo
global _22_foo_deep_release
_22_foo_deep_release:
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
    jg _22_foo_release_skip_0
    mov rax, [r12-16] ; load _22_foo_release_field_0 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
_22_foo_release_skip_0:
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _22_foo_deepcopy
_22_foo_deepcopy:
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
    jg _22_foo_deepcopy_skip_0
    mov rcx, [r12-16] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-16], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
_22_foo_deepcopy_skip_0:
    leave
    ret

global _19_foo
_19_foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store ok arg in frame
    mov [rbp-16], rsi ; store x arg in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
    mov rsi, 64 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rbx, rax ; closure env base pointer
    mov rax, [rbp-8] ; load operand
    mov r12, rax ; shadow closure env_end pointer
    push rbx ; save env base pointer
    mov rbx, r12 ; clone source env_end pointer
    mov r13, [rbx+24] ; load env size metadata for clone
    mov r14, [rbx+32] ; load heap size metadata for clone
    mov r12, rbx ; compute env base pointer for clone
    sub r12, r13 ; env base pointer for clone source
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
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
    mov r12, rbx ; cloned env_end pointer
    mov rax, [r12+16] ; load deepcopy helper entry point
    push r12 ; preserve cloned env_end pointer
    mov rdi, r12 ; pass env_end pointer to deepcopy helper
    call rax ; deepcopy reference fields
    pop r12 ; restore cloned env_end pointer
    mov rax, r12 ; copy closure env_end to rax
    pop rbx ; restore env base pointer
    mov [rbx+0], r12 ; capture cloned closure pointer
    mov r12, rbx ; env_end pointer before metadata
    add r12, 16 ; move pointer past env payload
    mov rax, 16 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 64 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [_22_foo_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_22_foo_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_22_foo_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 1 ; store num_remaining
    mov rax, r12 ; copy _23_foo closure env_end to rax
    mov [rbp-24], rax ; store value
    mov rax, [rbp-16] ; load operand
    push rax ; stack arg
    lea rax, [rel _20] ; point to string literal
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
    mov r12, [rbp-24] ; load continuation env_end pointer
    mov [r12-8], rax ; store env field
    mov rax, [r12+0] ; load continuation entry point
    mov rdi, r12 ; pass env_end pointer to continuation
    leave ; unwind before jumping
    jmp rax
global _19_foo_unwrapper
_19_foo_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-16] ; load ok env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-8] ; load x env field
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
    jmp _19_foo
global _19_foo_deep_release
_19_foo_deep_release:
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
    jg _19_foo_release_skip_0
    mov rax, [r12-16] ; load _19_foo_release_field_0 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
_19_foo_release_skip_0:
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _19_foo_deepcopy
_19_foo_deepcopy:
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
    jg _19_foo_deepcopy_skip_0
    mov rcx, [r12-16] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-16], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
_19_foo_deepcopy_skip_0:
    leave
    ret

global foo
foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store x arg in frame
    mov [rbp-16], rsi ; store ok arg in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
    mov rsi, 64 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rbx, rax ; closure env base pointer
    mov rax, [rbp-16] ; load operand
    mov r12, rax ; shadow closure env_end pointer
    push rbx ; save env base pointer
    mov rbx, r12 ; clone source env_end pointer
    mov r13, [rbx+24] ; load env size metadata for clone
    mov r14, [rbx+32] ; load heap size metadata for clone
    mov r12, rbx ; compute env base pointer for clone
    sub r12, r13 ; env base pointer for clone source
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
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
    mov r12, rbx ; cloned env_end pointer
    mov rax, [r12+16] ; load deepcopy helper entry point
    push r12 ; preserve cloned env_end pointer
    mov rdi, r12 ; pass env_end pointer to deepcopy helper
    call rax ; deepcopy reference fields
    pop r12 ; restore cloned env_end pointer
    mov rax, r12 ; copy closure env_end to rax
    pop rbx ; restore env base pointer
    mov [rbx+0], r12 ; capture cloned closure pointer
    mov r12, rbx ; env_end pointer before metadata
    add r12, 16 ; move pointer past env payload
    mov rax, 16 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 64 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [_12_foo_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_12_foo_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_12_foo_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 1 ; store num_remaining
    mov rax, r12 ; copy _17_foo closure env_end to rax
    mov [rbp-24], rax ; store value
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
    mov rsi, 64 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rbx, rax ; closure env base pointer
    mov rax, [rbp-16] ; load operand
    mov r12, rax ; shadow closure env_end pointer
    push rbx ; save env base pointer
    mov rbx, r12 ; clone source env_end pointer
    mov r13, [rbx+24] ; load env size metadata for clone
    mov r14, [rbx+32] ; load heap size metadata for clone
    mov r12, rbx ; compute env base pointer for clone
    sub r12, r13 ; env base pointer for clone source
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
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
    mov r12, rbx ; cloned env_end pointer
    mov rax, [r12+16] ; load deepcopy helper entry point
    push r12 ; preserve cloned env_end pointer
    mov rdi, r12 ; pass env_end pointer to deepcopy helper
    call rax ; deepcopy reference fields
    pop r12 ; restore cloned env_end pointer
    mov rax, r12 ; copy closure env_end to rax
    pop rbx ; restore env base pointer
    mov [rbx+0], r12 ; capture cloned closure pointer
    mov r12, rbx ; env_end pointer before metadata
    add r12, 16 ; move pointer past env payload
    mov rax, 16 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 64 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [_19_foo_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_19_foo_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_19_foo_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 1 ; store num_remaining
    mov rax, r12 ; copy _24_foo closure env_end to rax
    mov [rbp-32], rax ; store value
    mov rbx, [rbp-8] ; load x closure env_end pointer
    mov rax, [rbp-24] ; load operand
    mov [rbx-16], rax ; store env field
    mov rax, [rbp-32] ; load operand
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
    mov rax, [r12-16] ; load x env field
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

global main
main:
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
    mov rax, 42 ; operand literal
    mov [rbx+0], rax ; capture arg into env
    mov r12, rbx ; env_end pointer before metadata
    add r12, 24 ; move pointer past env payload
    mov rax, 24 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 72 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [as_int_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [as_int_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [as_int_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 2 ; store num_remaining
    mov rax, r12 ; copy _26_as_int closure env_end to rax
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
    lea rax, [_28_main_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_28_main_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_28_main_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 0 ; store num_remaining
    mov rax, r12 ; copy _28_main closure env_end to rax
    mov [rbp-16], rax ; store value
    mov rax, [rbp-16] ; load operand
    push rax ; stack arg
    mov rax, [rbp-8] ; load operand
    push rax ; stack arg
    pop rdi ; restore arg into register
    pop rsi ; restore arg into register
    leave ; unwind before named jump
    jmp foo
global main_unwrapper
main_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave ; unwind before named jump
    jmp main
global main_deep_release
main_deep_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global main_deepcopy
main_deepcopy:
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
    jmp main
extern exit
extern sprintf
extern write
section .rodata
_13:
    db "number: %d", 0
_20:
    db "x is str: %s", 10, 0
