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
global foo
foo:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov [rbp-32], rsi ; store scalar arg in frame
    mov [rbp-48], rcx ; save closure env_end pointer
    mov rdx, [rbp-48] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-16] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore scalar arg into register
    pop rdx ; restore closure code into register
    pop rcx ; restore closure env_end into register
    leave ; unwind before named jump
    jmp add ; jump to fully applied function
global foo_unwrapper
foo_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-24] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-16] ; load scalar env field
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rdx, [r10-8] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rbp-64], rdx ; update closure env_end pointer
    mov rdi, [rbp-16] ; load closure env_end pointer
    call release_heap_ptr ; release closure environment
    mov rdx, [rbp-64] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-48] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore scalar arg into register
    pop rdx ; restore closure code into register
    pop rcx ; restore closure env_end into register
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

global add
add:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov [rbp-32], rsi ; store scalar arg in frame
    mov [rbp-48], rcx ; save closure env_end pointer
    mov rax, rdi ; load first integer
    add rax, rsi ; add second integer
    lea rbx, [rcx-8] ; reserve slot for result before metadata
    mov [rbx], rax ; store sum
    mov rax, rdx ; continuation entry point
    mov rdi, rcx ; pass env_end pointer unchanged
    jmp rax ; jump into continuation

global add_unwrapper
add_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-24] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-16] ; load scalar env field
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rdx, [r10-8] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rbp-64], rdx ; update closure env_end pointer
    mov rdi, [rbp-16] ; load closure env_end pointer
    call release_heap_ptr ; release closure environment
    mov rdx, [rbp-64] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-48] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore scalar arg into register
    pop rdx ; restore closure code into register
    pop rcx ; restore closure env_end into register
    leave ; unwind before named jump
    jmp add ; jump to fully applied function
global _4_run_k
_4_run_k:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    mov [rbp-16], rsi ; save closure env_end pointer
    mov [rbp-32], rdx ; store scalar arg in frame
    lea rax, [rel _5] ; point to string literal
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-48] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore scalar arg into register
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
    mov [rbp-64], rax ; save evaluated scalar in frame
    mov rdx, [rbp-16] ; load closure env_end for exec
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
global _4_run_k_unwrapper
_4_run_k_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rdx, [r10-16] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rbp-32], rdx ; update closure env_end pointer
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-8] ; load scalar env field
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rdi, [rbp-16] ; load closure env_end pointer
    call release_heap_ptr ; release closure environment
    mov rax, [rbp-48] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rdx, [rbp-32] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    pop rdi ; restore closure code into register
    pop rsi ; restore closure env_end into register
    pop rdx ; restore scalar arg into register
    leave ; unwind before named jump
    jmp _4_run_k ; jump to fully applied function
global _4_run_k_release
_4_run_k_release:
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
    jl _4_run_k_release_if_num_curried_lt_1
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-16] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    jmp _4_run_k_release_done
_4_run_k_release_if_num_curried_lt_1:
_4_run_k_release_done:
    leave
    ret

global _4_run_k_deepcopy
_4_run_k_deepcopy:
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
    jl _4_run_k_deepcopy_if_num_curried_lt_1
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-16] ; load field pointer
    test rcx, rcx ; skip null field
    je _4_run_k_deepcopy_copy_field_null_0
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-16], rax ; store duplicated pointer
    jmp _4_run_k_deepcopy_copy_field_done_1
_4_run_k_deepcopy_copy_field_null_0:
    xor rax, rax ; null copy result
_4_run_k_deepcopy_copy_field_done_1:
    mov [rbp-48], rax ; save evaluated scalar in frame
    jmp _4_run_k_deepcopy_done
_4_run_k_deepcopy_if_num_curried_lt_1:
_4_run_k_deepcopy_done:
    leave
    ret

global _9_run
_9_run:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-16], rsi ; save closure env_end pointer
    mov [rbp-32], rcx ; save closure env_end pointer
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 64 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 16 ; bump pointer past env header
    mov qword [rdx+24], 16 ; env size metadata
    mov qword [rdx+32], 64 ; heap size metadata
    mov rax, _4_run_k_unwrapper ; load unwrapper entry point
    mov qword [rdx+0], rax ; store unwrapper entry in metadata
    mov rax, _4_run_k_release ; load release helper entry point
    mov qword [rdx+8], rax ; store release pointer in metadata
    mov rax, _4_run_k_deepcopy ; load deep copy helper entry point
    mov qword [rdx+16], rax ; store deep copy pointer in metadata
    xor rax, rax ; zero num_curried metadata
    mov qword [rdx+40], rax ; store num_curried
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rbx, [rsp+8] ; original closure env_end pointer
    mov r13, [rbx+24] ; load env size metadata for clone
    mov r14, [rbx+32] ; load heap size metadata for clone
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
    mov rdx, [rbp-32] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rsp+16], rax ; stash closure code pointer for clone
    mov rbx, rdx ; original closure env_end pointer
    mov r13, [rbx+24] ; load env size metadata for clone
    mov r14, [rbx+32] ; load heap size metadata for clone
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
    mov [rbx], rdx ; store closure env_end for arg
    mov rbx, [rsp+8] ; env_end pointer for num_curried update
    mov rax, [rbx+40] ; load current num_curried
    add rax, 1 ; increment num_curried
    mov [rbx+40], rax ; store updated num_curried
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov [rbp-48], rdx ; update closure env_end pointer
    mov rdx, [rbp-16] ; load closure env_end for exec
    mov rax, [rdx+0] ; load closure unwrapper entry point
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rdx, [rbp-48] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 8 ; compute slot for next argument
    mov [rbx], rdx ; store closure env_end for arg
    mov rbx, [rsp+8] ; env_end pointer for num_curried update
    mov rax, [rbx+40] ; load current num_curried
    add rax, 1 ; increment num_curried
    mov [rbx+40], rax ; store updated num_curried
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov rdi, rdx ; pass env_end pointer as parameter
    leave ; unwind before calling closure
    jmp rax ; jump into fully applied closure
global _9_run_unwrapper
_9_run_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rdx, [r10-16] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rbp-32], rdx ; update closure env_end pointer
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rdx, [r10-8] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rbp-48], rdx ; update closure env_end pointer
    mov rdi, [rbp-16] ; load closure env_end pointer
    call release_heap_ptr ; release closure environment
    mov rdx, [rbp-48] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rdx, [rbp-32] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    pop rdi ; restore closure code into register
    pop rsi ; restore closure env_end into register
    pop rdx ; restore closure code into register
    pop rcx ; restore closure env_end into register
    leave ; unwind before named jump
    jmp _9_run ; jump to fully applied function
global _9_run_release
_9_run_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax+40] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load operand
    mov rbx, 2 ; operand literal
    cmp rax, rbx
    jl _9_run_release_if_num_curried_lt_2
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-16] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-8] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    jmp _9_run_release_done
_9_run_release_if_num_curried_lt_2:
    mov rax, [rbp-32] ; load operand
    mov rbx, 1 ; operand literal
    cmp rax, rbx
    jl _9_run_release_if_num_curried_lt_1
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-16] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    jmp _9_run_release_done
_9_run_release_if_num_curried_lt_1:
_9_run_release_done:
    leave
    ret

global _9_run_deepcopy
_9_run_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 80 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax+40] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load operand
    mov rbx, 2 ; operand literal
    cmp rax, rbx
    jl _9_run_deepcopy_if_num_curried_lt_2
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-16] ; load field pointer
    test rcx, rcx ; skip null field
    je _9_run_deepcopy_copy_field_null_0
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-16], rax ; store duplicated pointer
    jmp _9_run_deepcopy_copy_field_done_1
_9_run_deepcopy_copy_field_null_0:
    xor rax, rax ; null copy result
_9_run_deepcopy_copy_field_done_1:
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-8] ; load field pointer
    test rcx, rcx ; skip null field
    je _9_run_deepcopy_copy_field_null_2
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-8], rax ; store duplicated pointer
    jmp _9_run_deepcopy_copy_field_done_3
_9_run_deepcopy_copy_field_null_2:
    xor rax, rax ; null copy result
_9_run_deepcopy_copy_field_done_3:
    mov [rbp-64], rax ; save evaluated scalar in frame
    jmp _9_run_deepcopy_done
_9_run_deepcopy_if_num_curried_lt_2:
    mov rax, [rbp-32] ; load operand
    mov rbx, 1 ; operand literal
    cmp rax, rbx
    jl _9_run_deepcopy_if_num_curried_lt_1
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-16] ; load field pointer
    test rcx, rcx ; skip null field
    je _9_run_deepcopy_copy_field_null_4
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-16], rax ; store duplicated pointer
    jmp _9_run_deepcopy_copy_field_done_5
_9_run_deepcopy_copy_field_null_4:
    xor rax, rax ; null copy result
_9_run_deepcopy_copy_field_done_5:
    mov [rbp-80], rax ; save evaluated scalar in frame
    jmp _9_run_deepcopy_done
_9_run_deepcopy_if_num_curried_lt_1:
_9_run_deepcopy_done:
    leave
    ret

global _13_run
_13_run:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-16], rsi ; save closure env_end pointer
    mov [rbp-32], rcx ; save closure env_end pointer
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 64 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 16 ; bump pointer past env header
    mov qword [rdx+24], 16 ; env size metadata
    mov qword [rdx+32], 64 ; heap size metadata
    mov rax, _4_run_k_unwrapper ; load unwrapper entry point
    mov qword [rdx+0], rax ; store unwrapper entry in metadata
    mov rax, _4_run_k_release ; load release helper entry point
    mov qword [rdx+8], rax ; store release pointer in metadata
    mov rax, _4_run_k_deepcopy ; load deep copy helper entry point
    mov qword [rdx+16], rax ; store deep copy pointer in metadata
    xor rax, rax ; zero num_curried metadata
    mov qword [rdx+40], rax ; store num_curried
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rbx, [rsp+8] ; original closure env_end pointer
    mov r13, [rbx+24] ; load env size metadata for clone
    mov r14, [rbx+32] ; load heap size metadata for clone
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
    mov rdx, [rbp-32] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rsp+16], rax ; stash closure code pointer for clone
    mov rbx, rdx ; original closure env_end pointer
    mov r13, [rbx+24] ; load env size metadata for clone
    mov r14, [rbx+32] ; load heap size metadata for clone
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
    mov [rbx], rdx ; store closure env_end for arg
    mov rbx, [rsp+8] ; env_end pointer for num_curried update
    mov rax, [rbx+40] ; load current num_curried
    add rax, 1 ; increment num_curried
    mov [rbx+40], rax ; store updated num_curried
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov [rbp-48], rdx ; update closure env_end pointer
    mov rdx, [rbp-16] ; load closure env_end for exec
    mov rax, [rdx+0] ; load closure unwrapper entry point
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rdx, [rbp-48] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 8 ; compute slot for next argument
    mov [rbx], rdx ; store closure env_end for arg
    mov rbx, [rsp+8] ; env_end pointer for num_curried update
    mov rax, [rbx+40] ; load current num_curried
    add rax, 1 ; increment num_curried
    mov [rbx+40], rax ; store updated num_curried
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov rdi, rdx ; pass env_end pointer as parameter
    leave ; unwind before calling closure
    jmp rax ; jump into fully applied closure
global _13_run_unwrapper
_13_run_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 48 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rdx, [r10-16] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rbp-32], rdx ; update closure env_end pointer
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rdx, [r10-8] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rbp-48], rdx ; update closure env_end pointer
    mov rdi, [rbp-16] ; load closure env_end pointer
    call release_heap_ptr ; release closure environment
    mov rdx, [rbp-48] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rdx, [rbp-32] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    pop rdi ; restore closure code into register
    pop rsi ; restore closure env_end into register
    pop rdx ; restore closure code into register
    pop rcx ; restore closure env_end into register
    leave ; unwind before named jump
    jmp _13_run ; jump to fully applied function
global _13_run_release
_13_run_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax+40] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load operand
    mov rbx, 2 ; operand literal
    cmp rax, rbx
    jl _13_run_release_if_num_curried_lt_2
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-16] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-8] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    jmp _13_run_release_done
_13_run_release_if_num_curried_lt_2:
    mov rax, [rbp-32] ; load operand
    mov rbx, 1 ; operand literal
    cmp rax, rbx
    jl _13_run_release_if_num_curried_lt_1
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-16] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    jmp _13_run_release_done
_13_run_release_if_num_curried_lt_1:
_13_run_release_done:
    leave
    ret

global _13_run_deepcopy
_13_run_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 80 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax+40] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load operand
    mov rbx, 2 ; operand literal
    cmp rax, rbx
    jl _13_run_deepcopy_if_num_curried_lt_2
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-16] ; load field pointer
    test rcx, rcx ; skip null field
    je _13_run_deepcopy_copy_field_null_0
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-16], rax ; store duplicated pointer
    jmp _13_run_deepcopy_copy_field_done_1
_13_run_deepcopy_copy_field_null_0:
    xor rax, rax ; null copy result
_13_run_deepcopy_copy_field_done_1:
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-8] ; load field pointer
    test rcx, rcx ; skip null field
    je _13_run_deepcopy_copy_field_null_2
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-8], rax ; store duplicated pointer
    jmp _13_run_deepcopy_copy_field_done_3
_13_run_deepcopy_copy_field_null_2:
    xor rax, rax ; null copy result
_13_run_deepcopy_copy_field_done_3:
    mov [rbp-64], rax ; save evaluated scalar in frame
    jmp _13_run_deepcopy_done
_13_run_deepcopy_if_num_curried_lt_2:
    mov rax, [rbp-32] ; load operand
    mov rbx, 1 ; operand literal
    cmp rax, rbx
    jl _13_run_deepcopy_if_num_curried_lt_1
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-16] ; load field pointer
    test rcx, rcx ; skip null field
    je _13_run_deepcopy_copy_field_null_4
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-16], rax ; store duplicated pointer
    jmp _13_run_deepcopy_copy_field_done_5
_13_run_deepcopy_copy_field_null_4:
    xor rax, rax ; null copy result
_13_run_deepcopy_copy_field_done_5:
    mov [rbp-80], rax ; save evaluated scalar in frame
    jmp _13_run_deepcopy_done
_13_run_deepcopy_if_num_curried_lt_1:
_13_run_deepcopy_done:
    leave
    ret

global run
run:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 160 ; reserve stack space for locals
    mov [rbp-16], rsi ; save closure env_end pointer
    mov rax, 1 ; load literal integer
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, 2 ; load literal integer
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, 3 ; load literal integer
    mov [rbp-64], rax ; save evaluated scalar in frame
    mov rax, 1 ; load literal integer
    mov [rbp-80], rax ; save evaluated scalar in frame
    mov rax, 1 ; load literal integer
    mov [rbp-96], rax ; save evaluated scalar in frame
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 72 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 24 ; bump pointer past env header
    mov qword [rdx+24], 24 ; env size metadata
    mov qword [rdx+32], 72 ; heap size metadata
    mov rax, foo_unwrapper ; load unwrapper entry point
    mov qword [rdx+0], rax ; store unwrapper entry in metadata
    mov rax, foo_release ; load release helper entry point
    mov qword [rdx+8], rax ; store release pointer in metadata
    mov rax, foo_deepcopy ; load deep copy helper entry point
    mov qword [rdx+16], rax ; store deep copy pointer in metadata
    xor rax, rax ; zero num_curried metadata
    mov qword [rdx+40], rax ; store num_curried
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rbx, [rsp+8] ; original closure env_end pointer
    mov r13, [rbx+24] ; load env size metadata for clone
    mov r14, [rbx+32] ; load heap size metadata for clone
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
    sub rbx, 24 ; compute slot for next argument
    mov [rbx], rax ; store scalar arg in env
    mov rax, [rbp-48] ; load scalar from frame
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 16 ; compute slot for next argument
    mov [rbx], rax ; store scalar arg in env
    mov rbx, [rsp+8] ; env_end pointer for num_curried update
    mov rax, [rbx+40] ; load current num_curried
    add rax, 2 ; increment num_curried
    mov [rbx+40], rax ; store updated num_curried
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov [rbp-112], rdx ; update closure env_end pointer
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 64 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 16 ; bump pointer past env header
    mov qword [rdx+24], 16 ; env size metadata
    mov qword [rdx+32], 64 ; heap size metadata
    mov rax, _9_run_unwrapper ; load unwrapper entry point
    mov qword [rdx+0], rax ; store unwrapper entry in metadata
    mov rax, _9_run_release ; load release helper entry point
    mov qword [rdx+8], rax ; store release pointer in metadata
    mov rax, _9_run_deepcopy ; load deep copy helper entry point
    mov qword [rdx+16], rax ; store deep copy pointer in metadata
    xor rax, rax ; zero num_curried metadata
    mov qword [rdx+40], rax ; store num_curried
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rdx, [rbp-112] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rsp+16], rax ; stash closure code pointer for clone
    mov rbx, rdx ; original closure env_end pointer
    mov r13, [rbx+24] ; load env size metadata for clone
    mov r14, [rbx+32] ; load heap size metadata for clone
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
    mov [rbx], rdx ; store closure env_end for arg
    mov rdx, [rbp-16] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rsp+16], rax ; stash closure code pointer for clone
    mov rbx, rdx ; original closure env_end pointer
    mov r13, [rbx+24] ; load env size metadata for clone
    mov r14, [rbx+32] ; load heap size metadata for clone
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
    sub rbx, 8 ; compute slot for next argument
    mov [rbx], rdx ; store closure env_end for arg
    mov rbx, [rsp+8] ; env_end pointer for num_curried update
    mov rax, [rbx+40] ; load current num_curried
    add rax, 2 ; increment num_curried
    mov [rbx+40], rax ; store updated num_curried
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov [rbp-128], rdx ; update closure env_end pointer
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 72 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 24 ; bump pointer past env header
    mov qword [rdx+24], 24 ; env size metadata
    mov qword [rdx+32], 72 ; heap size metadata
    mov rax, foo_unwrapper ; load unwrapper entry point
    mov qword [rdx+0], rax ; store unwrapper entry in metadata
    mov rax, foo_release ; load release helper entry point
    mov qword [rdx+8], rax ; store release pointer in metadata
    mov rax, foo_deepcopy ; load deep copy helper entry point
    mov qword [rdx+16], rax ; store deep copy pointer in metadata
    xor rax, rax ; zero num_curried metadata
    mov qword [rdx+40], rax ; store num_curried
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rbx, [rsp+8] ; original closure env_end pointer
    mov r13, [rbx+24] ; load env size metadata for clone
    mov r14, [rbx+32] ; load heap size metadata for clone
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
    sub rbx, 24 ; compute slot for next argument
    mov [rbx], rax ; store scalar arg in env
    mov rax, [rbp-64] ; load scalar from frame
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 16 ; compute slot for next argument
    mov [rbx], rax ; store scalar arg in env
    mov rbx, [rsp+8] ; env_end pointer for num_curried update
    mov rax, [rbx+40] ; load current num_curried
    add rax, 2 ; increment num_curried
    mov [rbx+40], rax ; store updated num_curried
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov [rbp-144], rdx ; update closure env_end pointer
    mov rax, 9 ; mmap syscall
    xor rdi, rdi ; addr = NULL hint
    mov rsi, 64 ; length for allocation
    mov rdx, 3 ; prot = read/write
    mov r10, 34 ; flags: private & anonymous
    mov r8, -1 ; fd = -1
    xor r9, r9 ; offset = 0
    syscall ; allocate env pages
    mov rdx, rax ; store env base pointer
    add rdx, 16 ; bump pointer past env header
    mov qword [rdx+24], 16 ; env size metadata
    mov qword [rdx+32], 64 ; heap size metadata
    mov rax, _13_run_unwrapper ; load unwrapper entry point
    mov qword [rdx+0], rax ; store unwrapper entry in metadata
    mov rax, _13_run_release ; load release helper entry point
    mov qword [rdx+8], rax ; store release pointer in metadata
    mov rax, _13_run_deepcopy ; load deep copy helper entry point
    mov qword [rdx+16], rax ; store deep copy pointer in metadata
    xor rax, rax ; zero num_curried metadata
    mov qword [rdx+40], rax ; store num_curried
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rdx, [rbp-144] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rsp+16], rax ; stash closure code pointer for clone
    mov rbx, rdx ; original closure env_end pointer
    mov r13, [rbx+24] ; load env size metadata for clone
    mov r14, [rbx+32] ; load heap size metadata for clone
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
    mov [rbx], rdx ; store closure env_end for arg
    mov rdx, [rbp-16] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rsp+16], rax ; stash closure code pointer for clone
    mov rbx, rdx ; original closure env_end pointer
    mov r13, [rbx+24] ; load env size metadata for clone
    mov r14, [rbx+32] ; load heap size metadata for clone
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
    sub rbx, 8 ; compute slot for next argument
    mov [rbx], rdx ; store closure env_end for arg
    mov rbx, [rsp+8] ; env_end pointer for num_curried update
    mov rax, [rbx+40] ; load current num_curried
    add rax, 2 ; increment num_curried
    mov [rbx+40], rax ; store updated num_curried
    mov rax, [rsp] ; restore closure code pointer
    mov rdx, [rsp+8] ; restore closure env_end pointer
    add rsp, 24 ; pop temporary closure state
    mov [rbp-160], rdx ; update closure env_end pointer
    mov rdx, [rbp-160] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rdx, [rbp-128] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-96] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-80] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore scalar arg into register
    pop rdx ; restore closure code into register
    pop rcx ; restore closure env_end into register
    pop r8 ; restore closure code into register
    pop r9 ; restore closure env_end into register
    leave ; unwind before named jump
    jmp eqi ; jump to fully applied function
global run_unwrapper
run_unwrapper:
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
    jmp run ; jump to fully applied function
global run_release
run_release:
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
    jl run_release_if_num_curried_lt_1
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-8] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    jmp run_release_done
run_release_if_num_curried_lt_1:
run_release_done:
    leave
    ret

global run_deepcopy
run_deepcopy:
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
    jl run_deepcopy_if_num_curried_lt_1
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-8] ; load field pointer
    test rcx, rcx ; skip null field
    je run_deepcopy_copy_field_null_0
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-8], rax ; store duplicated pointer
    jmp run_deepcopy_copy_field_done_1
run_deepcopy_copy_field_null_0:
    xor rax, rax ; null copy result
run_deepcopy_copy_field_done_1:
    mov [rbp-48], rax ; save evaluated scalar in frame
    jmp run_deepcopy_done
run_deepcopy_if_num_curried_lt_1:
run_deepcopy_done:
    leave
    ret

global eqi
eqi:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 80 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov [rbp-32], rsi ; store scalar arg in frame
    mov [rbp-48], rcx ; save closure env_end pointer
    mov [rbp-64], r9 ; save closure env_end pointer
    cmp rdi, rsi ; load first integer
    jne eqi_false_1 ; compare to second integer
eqi_true_0:
    mov rax, rdx ; continuation entry point
    mov rdi, rcx ; pass env_end pointer to continuation
    leave
    jmp rax
eqi_false_1:
    mov rax, r8 ; continuation entry point
    mov rdi, r9 ; pass env_end pointer to continuation
    leave
    jmp rax

global eqi_unwrapper
eqi_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 80 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-32] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-24] ; load scalar env field
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rdx, [r10-16] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rbp-64], rdx ; update closure env_end pointer
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rdx, [r10-8] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rbp-80], rdx ; update closure env_end pointer
    mov rdi, [rbp-16] ; load closure env_end pointer
    call release_heap_ptr ; release closure environment
    mov rdx, [rbp-80] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rdx, [rbp-64] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-48] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore scalar arg into register
    pop rdx ; restore closure code into register
    pop rcx ; restore closure env_end into register
    pop r8 ; restore closure code into register
    pop r9 ; restore closure env_end into register
    leave ; unwind before named jump
    jmp eqi ; jump to fully applied function
global _17_lambda
_17_lambda:
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
global _17_lambda_unwrapper
_17_lambda_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rdi, [rbp-16] ; load closure env_end pointer
    call release_heap_ptr ; release closure environment
    leave ; unwind before named jump
    jmp _17_lambda ; jump to fully applied function
global _17_lambda_release
_17_lambda_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    leave
    ret

global _17_lambda_deepcopy
_17_lambda_deepcopy:
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
global _start
_start:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
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
    mov rax, _17_lambda_unwrapper ; load unwrapper entry point
    mov qword [rdx+0], rax ; store unwrapper entry in metadata
    mov rax, _17_lambda_release ; load release helper entry point
    mov qword [rdx+8], rax ; store release pointer in metadata
    mov rax, _17_lambda_deepcopy ; load deep copy helper entry point
    mov qword [rdx+16], rax ; store deep copy pointer in metadata
    xor rax, rax ; zero num_curried metadata
    mov qword [rdx+40], rax ; store num_curried
    mov [rbp-16], rdx ; update closure env_end pointer
    mov rdx, [rbp-16] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    pop rdi ; restore closure code into register
    pop rsi ; restore closure env_end into register
    leave ; unwind before named jump
    jmp run ; jump to fully applied function
extern deepcopy_heap_ptr
extern printf
extern release_heap_ptr
section .rodata
_5:
    db "number: %d", 0
