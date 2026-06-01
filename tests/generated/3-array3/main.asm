bits 64
default rel
section .text
global _40_main
_40_main:
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
global _40_main_unwrapper
_40_main_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave ; unwind before named jump
    jmp _40_main
global _40_main_deep_release
_40_main_deep_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _40_main_deepcopy
_40_main_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    leave
    ret

global _37_main
_37_main:
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
    lea rax, [_40_main_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_40_main_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_40_main_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 0 ; store num_remaining
    mov rax, r12 ; copy _40_main closure env_end to rax
    mov [rbp-16], rax ; store value
    mov rax, [rbp-8] ; load operand
    push rax ; stack arg
    lea rax, [rel _38] ; point to string literal
    push rax ; stack arg
    pop rdi ; restore arg into register
    pop rsi ; restore arg into register
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
    mov r12, [rbp-16] ; load continuation env_end pointer
    mov rax, [r12+0] ; load continuation entry point
    mov rdi, r12 ; pass env_end pointer to continuation
    leave ; unwind before jumping
    jmp rax
global _37_main_unwrapper
_37_main_unwrapper:
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
    jmp _37_main
global _37_main_deep_release
_37_main_deep_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _37_main_deepcopy
_37_main_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    leave
    ret

global _48_main
_48_main:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    ; load exit code
    mov rdi, 0 ; exit code
    call exit ; call libc exit to flush buffers
global _48_main_unwrapper
_48_main_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave ; unwind before named jump
    jmp _48_main
global _48_main_deep_release
_48_main_deep_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _48_main_deepcopy
_48_main_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    leave
    ret

global _45_main
_45_main:
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
    lea rax, [_48_main_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_48_main_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_48_main_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 0 ; store num_remaining
    mov rax, r12 ; copy _48_main closure env_end to rax
    mov [rbp-8], rax ; store value
    lea rax, [rel _46] ; point to string literal
    push rax ; stack arg
    pop rdi ; restore arg into register
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
    mov r12, [rbp-8] ; load continuation env_end pointer
    mov rax, [r12+0] ; load continuation entry point
    mov rdi, r12 ; pass env_end pointer to continuation
    leave ; unwind before jumping
    jmp rax
global _45_main_unwrapper
_45_main_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave ; unwind before named jump
    jmp _45_main
global _45_main_deep_release
_45_main_deep_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _45_main_deepcopy
_45_main_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    leave
    ret

global _34_main
_34_main:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store nth arg in frame
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
    lea rax, [_37_main_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_37_main_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_37_main_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 1 ; store num_remaining
    mov rax, r12 ; copy _37_main closure env_end to rax
    mov [rbp-16], rax ; store value
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
    lea rax, [_45_main_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_45_main_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_45_main_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 0 ; store num_remaining
    mov rax, r12 ; copy _45_main closure env_end to rax
    mov [rbp-24], rax ; store value
    mov rbx, [rbp-8] ; load nth closure env_end pointer
    mov rax, 1 ; operand literal
    mov [rbx-24], rax ; store env field
    mov rax, [rbp-16] ; load operand
    mov [rbx-16], rax ; store env field
    mov rax, [rbp-24] ; load operand
    mov [rbx-8], rax ; store env field
    mov rdi, rbx ; pass env_end pointer to closure
    mov rax, [rdi+0] ; load closure unwrapper entry point
    leave ; unwind before jumping
    jmp rax ; tail call into closure
global _34_main_unwrapper
_34_main_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-8] ; load nth env field
    mov [rbp-16], rax ; store value
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    mov rax, [rbp-16] ; load operand
    push rax ; stack arg
    pop rdi ; restore arg into register
    leave ; unwind before named jump
    jmp _34_main
global _34_main_deep_release
_34_main_deep_release:
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
    jg _34_main_release_skip_0
    mov rax, [r12-8] ; load _34_main_release_field_0 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
_34_main_release_skip_0:
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
global _34_main_deepcopy
_34_main_deepcopy:
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
    jg _34_main_deepcopy_skip_0
    mov rcx, [r12-8] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-8], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
_34_main_deepcopy_skip_0:
    leave
    ret

global _31_main
_31_main:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store len arg in frame
    mov [rbp-16], rsi ; store nth arg in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
    mov rsi, 56 ; length for allocation
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
    add r12, 8 ; move pointer past env payload
    mov rax, 8 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 56 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [_34_main_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_34_main_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_34_main_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 0 ; store num_remaining
    mov rax, r12 ; copy _52_main closure env_end to rax
    mov [rbp-24], rax ; store value
    mov rax, [rbp-8] ; load operand
    push rax ; stack arg
    lea rax, [rel _32] ; point to string literal
    push rax ; stack arg
    pop rdi ; restore arg into register
    pop rsi ; restore arg into register
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
global _31_main_unwrapper
_31_main_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-16] ; load len env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-8] ; load nth env field
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
    jmp _31_main
global _31_main_deep_release
_31_main_deep_release:
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
    jg _31_main_release_skip_1
    mov rax, [r12-8] ; load _31_main_release_field_1 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
_31_main_release_skip_1:
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _31_main_deepcopy
_31_main_deepcopy:
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
    jg _31_main_deepcopy_skip_1
    mov rcx, [r12-8] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-8], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
_31_main_deepcopy_skip_1:
    leave
    ret

global _8_array3
_8_array3:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store one arg in frame
    mov [rbp-16], rsi ; store a arg in frame
    mov rbx, [rbp-8] ; load one closure env_end pointer
    mov rax, [rbp-16] ; load operand
    mov [rbx-8], rax ; store env field
    mov rdi, rbx ; pass env_end pointer to closure
    mov rax, [rdi+0] ; load closure unwrapper entry point
    leave ; unwind before jumping
    jmp rax ; tail call into closure
global _8_array3_unwrapper
_8_array3_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-16] ; load one env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-8] ; load a env field
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
    jmp _8_array3
global _8_array3_deep_release
_8_array3_deep_release:
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
    jg _8_array3_release_skip_0
    mov rax, [r12-16] ; load _8_array3_release_field_0 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
_8_array3_release_skip_0:
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _8_array3_deepcopy
_8_array3_deepcopy:
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
    jg _8_array3_deepcopy_skip_0
    mov rcx, [r12-16] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-16], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
_8_array3_deepcopy_skip_0:
    leave
    ret

global _14_array3
_14_array3:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store one arg in frame
    mov [rbp-16], rsi ; store b arg in frame
    mov rbx, [rbp-8] ; load one closure env_end pointer
    mov rax, [rbp-16] ; load operand
    mov [rbx-8], rax ; store env field
    mov rdi, rbx ; pass env_end pointer to closure
    mov rax, [rdi+0] ; load closure unwrapper entry point
    leave ; unwind before jumping
    jmp rax ; tail call into closure
global _14_array3_unwrapper
_14_array3_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-16] ; load one env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-8] ; load b env field
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
    jmp _14_array3
global _14_array3_deep_release
_14_array3_deep_release:
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
    jg _14_array3_release_skip_0
    mov rax, [r12-16] ; load _14_array3_release_field_0 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
_14_array3_release_skip_0:
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _14_array3_deepcopy
_14_array3_deepcopy:
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
    jg _14_array3_deepcopy_skip_0
    mov rcx, [r12-16] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-16], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
_14_array3_deepcopy_skip_0:
    leave
    ret

global _20_array3
_20_array3:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-8], rdi ; store one arg in frame
    mov [rbp-16], rsi ; store c arg in frame
    mov rbx, [rbp-8] ; load one closure env_end pointer
    mov rax, [rbp-16] ; load operand
    mov [rbx-8], rax ; store env field
    mov rdi, rbx ; pass env_end pointer to closure
    mov rax, [rdi+0] ; load closure unwrapper entry point
    leave ; unwind before jumping
    jmp rax ; tail call into closure
global _20_array3_unwrapper
_20_array3_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-16] ; load one env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-8] ; load c env field
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
    jmp _20_array3
global _20_array3_deep_release
_20_array3_deep_release:
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
    jg _20_array3_release_skip_0
    mov rax, [r12-16] ; load _20_array3_release_field_0 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
_20_array3_release_skip_0:
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _20_array3_deepcopy
_20_array3_deepcopy:
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
    jg _20_array3_deepcopy_skip_0
    mov rcx, [r12-16] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-16], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
_20_array3_deepcopy_skip_0:
    leave
    ret

global _17_array3
_17_array3:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-8], rdi ; store index arg in frame
    mov [rbp-16], rsi ; store one arg in frame
    mov [rbp-24], rdx ; store c arg in frame
    mov [rbp-32], rcx ; store none arg in frame
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
    mov rax, [rbp-24] ; load operand
    mov [rbx+8], rax ; capture arg into env
    mov r12, rbx ; env_end pointer before metadata
    add r12, 16 ; move pointer past env payload
    mov rax, 16 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 64 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [_20_array3_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_20_array3_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_20_array3_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 0 ; store num_remaining
    mov rax, r12 ; copy _21_array3 closure env_end to rax
    mov [rbp-40], rax ; store value
    mov rax, [rbp-8] ; load operand
    mov rbx, 2 ; operand literal
    cmp rax, rbx
    je eq__21_array3_true_0_0
eq_none_false_0_0:
    mov rdi, [rbp-40] ; load _21_array3 closure env_end pointer
    call release_heap_ptr ; release _21_array3 closure environment
    mov rbx, [rbp-32] ; load none closure env_end pointer
    mov rdi, rbx ; pass env_end pointer to closure
    mov rax, [rdi+0] ; load closure unwrapper entry point
    leave ; unwind before jumping
    jmp rax ; tail call into closure
eq__21_array3_true_0_0:
    mov rdi, [rbp-32] ; load none closure env_end pointer
    call release_heap_ptr ; release none closure environment
    mov rbx, [rbp-40] ; load _21_array3 closure env_end pointer
    mov rdi, rbx ; pass env_end pointer to closure
    mov rax, [rdi+0] ; load closure unwrapper entry point
    leave ; unwind before jumping
    jmp rax ; tail call into closure
global _17_array3_unwrapper
_17_array3_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-32] ; load index env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-24] ; load one env field
    mov [rbp-24], rax ; store value
    mov rax, [r12-16] ; load c env field
    mov [rbp-32], rax ; store value
    mov rax, [r12-8] ; load none env field
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
    jmp _17_array3
global _17_array3_deep_release
_17_array3_deep_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12+40] ; load __num_remaining env field
    mov [rbp-16], rax ; store value
    mov rax, [rbp-16] ; load operand
    mov rbx, 2 ; operand literal
    cmp rax, rbx
    jg _17_array3_release_skip_1
    mov rax, [r12-24] ; load _17_array3_release_field_1 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
_17_array3_release_skip_1:
    mov rax, [rbp-16] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    jg _17_array3_release_skip_3
    mov rax, [r12-8] ; load _17_array3_release_field_3 env field
    mov [rbp-32], rax ; store value
    mov rdi, [rbp-32] ; load operand
    call release_heap_ptr ; release heap pointer
_17_array3_release_skip_3:
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _17_array3_deepcopy
_17_array3_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12+40] ; load num_remaining env field
    mov [rbp-16], rax ; store value
    mov rax, [rbp-16] ; load operand
    mov rbx, 2 ; operand literal
    cmp rax, rbx
    jg _17_array3_deepcopy_skip_1
    mov rcx, [r12-24] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-24], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
_17_array3_deepcopy_skip_1:
    mov rax, [rbp-16] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    jg _17_array3_deepcopy_skip_3
    mov rcx, [r12-8] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-8], rax ; store duplicated pointer
    mov [rbp-32], rax ; store value
_17_array3_deepcopy_skip_3:
    leave
    ret

global _11_array3
_11_array3:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    mov [rbp-8], rdi ; store index arg in frame
    mov [rbp-16], rsi ; store one arg in frame
    mov [rbp-24], rdx ; store b arg in frame
    mov [rbp-32], rcx ; store c arg in frame
    mov [rbp-40], r8 ; store none arg in frame
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
    mov rax, [rbp-24] ; load operand
    mov [rbx+8], rax ; capture arg into env
    mov r12, rbx ; env_end pointer before metadata
    add r12, 16 ; move pointer past env payload
    mov rax, 16 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 64 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [_14_array3_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_14_array3_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_14_array3_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 0 ; store num_remaining
    mov rax, r12 ; copy _15_array3 closure env_end to rax
    mov [rbp-48], rax ; store value
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
    mov rsi, 80 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rbx, rax ; closure env base pointer
    mov rax, [rbp-8] ; load operand
    mov [rbx+0], rax ; capture arg into env
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
    mov [rbx+8], r12 ; capture cloned closure pointer
    mov rax, [rbp-32] ; load operand
    mov [rbx+16], rax ; capture arg into env
    mov rax, [rbp-40] ; load operand
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
    mov [rbx+24], r12 ; capture cloned closure pointer
    mov r12, rbx ; env_end pointer before metadata
    add r12, 32 ; move pointer past env payload
    mov rax, 32 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 80 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [_17_array3_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_17_array3_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_17_array3_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 0 ; store num_remaining
    mov rax, r12 ; copy _22_array3 closure env_end to rax
    mov [rbp-56], rax ; store value
    mov rax, [rbp-8] ; load operand
    mov rbx, 1 ; operand literal
    cmp rax, rbx
    je eq__15_array3_true_0_0
eq__22_array3_false_0_0:
    mov rdi, [rbp-48] ; load _15_array3 closure env_end pointer
    call release_heap_ptr ; release _15_array3 closure environment
    mov rbx, [rbp-56] ; load _22_array3 closure env_end pointer
    mov rdi, rbx ; pass env_end pointer to closure
    mov rax, [rdi+0] ; load closure unwrapper entry point
    leave ; unwind before jumping
    jmp rax ; tail call into closure
eq__15_array3_true_0_0:
    mov rdi, [rbp-56] ; load _22_array3 closure env_end pointer
    call release_heap_ptr ; release _22_array3 closure environment
    mov rbx, [rbp-48] ; load _15_array3 closure env_end pointer
    mov rdi, rbx ; pass env_end pointer to closure
    mov rax, [rdi+0] ; load closure unwrapper entry point
    leave ; unwind before jumping
    jmp rax ; tail call into closure
global _11_array3_unwrapper
_11_array3_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-40] ; load index env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-32] ; load one env field
    mov [rbp-24], rax ; store value
    mov rax, [r12-24] ; load b env field
    mov [rbp-32], rax ; store value
    mov rax, [r12-16] ; load c env field
    mov [rbp-40], rax ; store value
    mov rax, [r12-8] ; load none env field
    mov [rbp-48], rax ; store value
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
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
    leave ; unwind before named jump
    jmp _11_array3
global _11_array3_deep_release
_11_array3_deep_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12+40] ; load __num_remaining env field
    mov [rbp-16], rax ; store value
    mov rax, [rbp-16] ; load operand
    mov rbx, 3 ; operand literal
    cmp rax, rbx
    jg _11_array3_release_skip_1
    mov rax, [r12-32] ; load _11_array3_release_field_1 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
_11_array3_release_skip_1:
    mov rax, [rbp-16] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    jg _11_array3_release_skip_4
    mov rax, [r12-8] ; load _11_array3_release_field_4 env field
    mov [rbp-32], rax ; store value
    mov rdi, [rbp-32] ; load operand
    call release_heap_ptr ; release heap pointer
_11_array3_release_skip_4:
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _11_array3_deepcopy
_11_array3_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12+40] ; load num_remaining env field
    mov [rbp-16], rax ; store value
    mov rax, [rbp-16] ; load operand
    mov rbx, 3 ; operand literal
    cmp rax, rbx
    jg _11_array3_deepcopy_skip_1
    mov rcx, [r12-32] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-32], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
_11_array3_deepcopy_skip_1:
    mov rax, [rbp-16] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    jg _11_array3_deepcopy_skip_4
    mov rcx, [r12-8] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-8], rax ; store duplicated pointer
    mov [rbp-32], rax ; store value
_11_array3_deepcopy_skip_4:
    leave
    ret

global _5_array3
_5_array3:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    mov [rbp-8], rdi ; store a arg in frame
    mov [rbp-16], rsi ; store b arg in frame
    mov [rbp-24], rdx ; store c arg in frame
    mov [rbp-32], rcx ; store index arg in frame
    mov [rbp-40], r8 ; store one arg in frame
    mov [rbp-48], r9 ; store none arg in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
    mov rsi, 64 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rbx, rax ; closure env base pointer
    mov rax, [rbp-40] ; load operand
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
    mov rax, [rbp-8] ; load operand
    mov [rbx+8], rax ; capture arg into env
    mov r12, rbx ; env_end pointer before metadata
    add r12, 16 ; move pointer past env payload
    mov rax, 16 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 64 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [_8_array3_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_8_array3_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_8_array3_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 0 ; store num_remaining
    mov rax, r12 ; copy _9_array3 closure env_end to rax
    mov [rbp-56], rax ; store value
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
    mov rsi, 88 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rbx, rax ; closure env base pointer
    mov rax, [rbp-32] ; load operand
    mov [rbx+0], rax ; capture arg into env
    mov rax, [rbp-40] ; load operand
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
    mov [rbx+8], r12 ; capture cloned closure pointer
    mov rax, [rbp-16] ; load operand
    mov [rbx+16], rax ; capture arg into env
    mov rax, [rbp-24] ; load operand
    mov [rbx+24], rax ; capture arg into env
    mov rax, [rbp-48] ; load operand
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
    mov [rbx+32], r12 ; capture cloned closure pointer
    mov r12, rbx ; env_end pointer before metadata
    add r12, 40 ; move pointer past env payload
    mov rax, 40 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 88 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [_11_array3_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_11_array3_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_11_array3_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 0 ; store num_remaining
    mov rax, r12 ; copy _23_array3 closure env_end to rax
    mov [rbp-64], rax ; store value
    mov rax, [rbp-32] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    je eq__9_array3_true_0_0
eq__23_array3_false_0_0:
    mov rdi, [rbp-56] ; load _9_array3 closure env_end pointer
    call release_heap_ptr ; release _9_array3 closure environment
    mov rbx, [rbp-64] ; load _23_array3 closure env_end pointer
    mov rdi, rbx ; pass env_end pointer to closure
    mov rax, [rdi+0] ; load closure unwrapper entry point
    leave ; unwind before jumping
    jmp rax ; tail call into closure
eq__9_array3_true_0_0:
    mov rdi, [rbp-64] ; load _23_array3 closure env_end pointer
    call release_heap_ptr ; release _23_array3 closure environment
    mov rbx, [rbp-56] ; load _9_array3 closure env_end pointer
    mov rdi, rbx ; pass env_end pointer to closure
    mov rax, [rdi+0] ; load closure unwrapper entry point
    leave ; unwind before jumping
    jmp rax ; tail call into closure
global _5_array3_unwrapper
_5_array3_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-48] ; load a env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-40] ; load b env field
    mov [rbp-24], rax ; store value
    mov rax, [r12-32] ; load c env field
    mov [rbp-32], rax ; store value
    mov rax, [r12-24] ; load index env field
    mov [rbp-40], rax ; store value
    mov rax, [r12-16] ; load one env field
    mov [rbp-48], rax ; store value
    mov rax, [r12-8] ; load none env field
    mov [rbp-56], rax ; store value
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
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
    leave ; unwind before named jump
    jmp _5_array3
global _5_array3_deep_release
_5_array3_deep_release:
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
    jg _5_array3_release_skip_4
    mov rax, [r12-16] ; load _5_array3_release_field_4 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
_5_array3_release_skip_4:
    mov rax, [rbp-16] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    jg _5_array3_release_skip_5
    mov rax, [r12-8] ; load _5_array3_release_field_5 env field
    mov [rbp-32], rax ; store value
    mov rdi, [rbp-32] ; load operand
    call release_heap_ptr ; release heap pointer
_5_array3_release_skip_5:
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global _5_array3_deepcopy
_5_array3_deepcopy:
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
    jg _5_array3_deepcopy_skip_4
    mov rcx, [r12-16] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-16], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
_5_array3_deepcopy_skip_4:
    mov rax, [rbp-16] ; load operand
    mov rbx, 0 ; operand literal
    cmp rax, rbx
    jg _5_array3_deepcopy_skip_5
    mov rcx, [r12-8] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-8], rax ; store duplicated pointer
    mov [rbp-32], rax ; store value
_5_array3_deepcopy_skip_5:
    leave
    ret

global array3
array3:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-8], rdi ; store a arg in frame
    mov [rbp-16], rsi ; store b arg in frame
    mov [rbp-24], rdx ; store c arg in frame
    mov [rbp-32], rcx ; store ok arg in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
    mov rsi, 96 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rbx, rax ; closure env base pointer
    mov rax, [rbp-8] ; load operand
    mov [rbx+0], rax ; capture arg into env
    mov rax, [rbp-16] ; load operand
    mov [rbx+8], rax ; capture arg into env
    mov rax, [rbp-24] ; load operand
    mov [rbx+16], rax ; capture arg into env
    mov r12, rbx ; env_end pointer before metadata
    add r12, 48 ; move pointer past env payload
    mov rax, 48 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 96 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [_5_array3_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_5_array3_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_5_array3_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 3 ; store num_remaining
    mov rax, r12 ; copy _24_array3 closure env_end to rax
    mov [rbp-40], rax ; store value
    mov rbx, [rbp-32] ; load ok closure env_end pointer
    mov rax, 3 ; operand literal
    mov [rbx-16], rax ; store env field
    mov rax, [rbp-40] ; load operand
    mov [rbx-8], rax ; store env field
    mov rdi, rbx ; pass env_end pointer to closure
    mov rax, [rdi+0] ; load closure unwrapper entry point
    leave ; unwind before jumping
    jmp rax ; tail call into closure
global array3_unwrapper
array3_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-8], rdi ; store env_end arg in frame
    mov r12, [rbp-8] ; load operand
    mov rax, [r12-32] ; load a env field
    mov [rbp-16], rax ; store value
    mov rax, [r12-24] ; load b env field
    mov [rbp-24], rax ; store value
    mov rax, [r12-16] ; load c env field
    mov [rbp-32], rax ; store value
    mov rax, [r12-8] ; load ok env field
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
    jmp array3
global array3_deep_release
array3_deep_release:
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
    jg array3_release_skip_3
    mov rax, [r12-8] ; load array3_release_field_3 env field
    mov [rbp-24], rax ; store value
    mov rdi, [rbp-24] ; load operand
    call release_heap_ptr ; release heap pointer
array3_release_skip_3:
    mov rdi, r12 ; use pinned __env_end env_end pointer
    call release_heap_ptr ; release __env_end closure environment
    leave
    ret

global array3_deepcopy
array3_deepcopy:
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
    jg array3_deepcopy_skip_3
    mov rcx, [r12-8] ; load field pointer
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [r12-8], rax ; store duplicated pointer
    mov [rbp-24], rax ; store value
array3_deepcopy_skip_3:
    leave
    ret

global main
main:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr hint for kernel base selection
    mov rsi, 64 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rbx, rax ; closure env base pointer
    mov r12, rbx ; env_end pointer before metadata
    add r12, 16 ; move pointer past env payload
    mov rax, 16 ; store env size metadata
    mov qword [r12+24], rax ; env size metadata
    mov rax, 64 ; store heap size metadata
    mov qword [r12+32], rax ; heap size metadata
    lea rax, [_31_main_unwrapper] ; load unwrapper entry point
    mov qword [r12+0], rax ; store unwrapper entry in metadata
    lea rax, [_31_main_deep_release] ; load release helper entry point
    mov qword [r12+8], rax ; store release pointer in metadata
    lea rax, [_31_main_deepcopy] ; load deep copy helper entry point
    mov qword [r12+16], rax ; store deep copy pointer in metadata
    mov qword [r12+40], 2 ; store num_remaining
    mov rax, r12 ; copy _31_main closure env_end to rax
    mov [rbp-8], rax ; store value
    mov rax, [rbp-8] ; load operand
    push rax ; stack arg
    lea rax, [rel _27] ; point to string literal
    push rax ; stack arg
    lea rax, [rel _26] ; point to string literal
    push rax ; stack arg
    lea rax, [rel _25] ; point to string literal
    push rax ; stack arg
    pop rdi ; restore arg into register
    pop rsi ; restore arg into register
    pop rdx ; restore arg into register
    pop rcx ; restore arg into register
    leave ; unwind before named jump
    jmp array3
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
extern printf
section .rodata
_38:
    db "element at index 1 is %s", 10, 0
_46:
    db "index out of bounds", 10, 0
_32:
    db "array has %d elements", 10, 0
_25:
    db "alice", 0
_26:
    db "bob", 0
_27:
    db "charlie", 0
