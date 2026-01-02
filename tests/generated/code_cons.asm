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
global nil
nil:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-16], rsi ; save closure env_end pointer
    mov [rbp-32], rcx ; save closure env_end pointer
    mov rdi, [rbp-16] ; load closure env_end pointer
    call release_heap_ptr ; release closure environment
    mov rdx, [rbp-32] ; load closure env_end for exec
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov rdi, rdx ; pass env_end pointer as parameter
    leave ; unwind before calling closure
    jmp rax ; jump into fully applied closure
global nil_unwrapper
nil_unwrapper:
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
    jmp nil ; jump to fully applied function
global nil_release
nil_release:
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
    jl nil_release_if_num_curried_lt_2
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-16] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-8] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    jmp nil_release_done
nil_release_if_num_curried_lt_2:
    mov rax, [rbp-32] ; load operand
    mov rbx, 1 ; operand literal
    cmp rax, rbx
    jl nil_release_if_num_curried_lt_1
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-16] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    jmp nil_release_done
nil_release_if_num_curried_lt_1:
nil_release_done:
    leave
    ret

global nil_deepcopy
nil_deepcopy:
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
    jl nil_deepcopy_if_num_curried_lt_2
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-16] ; load field pointer
    test rcx, rcx ; skip null field
    je nil_deepcopy_copy_field_null_0
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-16], rax ; store duplicated pointer
    jmp nil_deepcopy_copy_field_done_1
nil_deepcopy_copy_field_null_0:
    xor rax, rax ; null copy result
nil_deepcopy_copy_field_done_1:
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-8] ; load field pointer
    test rcx, rcx ; skip null field
    je nil_deepcopy_copy_field_null_2
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-8], rax ; store duplicated pointer
    jmp nil_deepcopy_copy_field_done_3
nil_deepcopy_copy_field_null_2:
    xor rax, rax ; null copy result
nil_deepcopy_copy_field_done_3:
    mov [rbp-64], rax ; save evaluated scalar in frame
    jmp nil_deepcopy_done
nil_deepcopy_if_num_curried_lt_2:
    mov rax, [rbp-32] ; load operand
    mov rbx, 1 ; operand literal
    cmp rax, rbx
    jl nil_deepcopy_if_num_curried_lt_1
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-16] ; load field pointer
    test rcx, rcx ; skip null field
    je nil_deepcopy_copy_field_null_4
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-16], rax ; store duplicated pointer
    jmp nil_deepcopy_copy_field_done_5
nil_deepcopy_copy_field_null_4:
    xor rax, rax ; null copy result
nil_deepcopy_copy_field_done_5:
    mov [rbp-80], rax ; save evaluated scalar in frame
    jmp nil_deepcopy_done
nil_deepcopy_if_num_curried_lt_1:
nil_deepcopy_done:
    leave
    ret

global cons
cons:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 64 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov [rbp-32], rdx ; save closure env_end pointer
    mov [rbp-48], r8 ; save closure env_end pointer
    mov [rbp-64], r10 ; save closure env_end pointer
    mov rdi, [rbp-64] ; load closure env_end pointer
    call release_heap_ptr ; release closure environment
    mov rdx, [rbp-48] ; load closure env_end for exec
    mov rax, [rdx+0] ; load closure unwrapper entry point
    sub rsp, 24 ; allocate temporary stack for closure state
    mov [rsp], rax ; save closure code pointer temporarily
    mov [rsp+8], rdx ; save closure env_end pointer temporarily
    mov rax, [rbp-16] ; load scalar from frame
    mov rbx, [rsp+8] ; env_end pointer
    sub rbx, 16 ; compute slot for next argument
    mov [rbx], rax ; store scalar arg in env
    mov rdx, [rbp-32] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
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
    mov rdi, rdx ; pass env_end pointer as parameter
    leave ; unwind before calling closure
    jmp rax ; jump into fully applied closure
global cons_unwrapper
cons_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 80 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-32] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rdx, [r10-24] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rbp-48], rdx ; update closure env_end pointer
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
    mov rdx, [rbp-48] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore closure code into register
    pop rdx ; restore closure env_end into register
    pop rcx ; restore closure code into register
    pop r8 ; restore closure env_end into register
    pop r9 ; restore closure code into register
    pop r10 ; restore closure env_end into register
    leave ; unwind before named jump
    jmp cons ; jump to fully applied function
global cons_release
cons_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 32 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax+40] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load operand
    mov rbx, 3 ; operand literal
    cmp rax, rbx
    jl cons_release_if_num_curried_lt_3
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-24] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-16] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-8] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    jmp cons_release_done
cons_release_if_num_curried_lt_3:
    mov rax, [rbp-32] ; load operand
    mov rbx, 2 ; operand literal
    cmp rax, rbx
    jl cons_release_if_num_curried_lt_2
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-24] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-16] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    jmp cons_release_done
cons_release_if_num_curried_lt_2:
    mov rax, [rbp-32] ; load operand
    mov rbx, 1 ; operand literal
    cmp rax, rbx
    jl cons_release_if_num_curried_lt_1
    mov rbx, [rbp-16] ; load env_end pointer for release
    mov rcx, [rbx-24] ; load field pointer
    mov rdi, rcx ; field release pointer
    call release_heap_ptr ; release heap pointer
    jmp cons_release_done
cons_release_if_num_curried_lt_1:
cons_release_done:
    leave
    ret

global cons_deepcopy
cons_deepcopy:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 128 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax+40] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-32] ; load operand
    mov rbx, 3 ; operand literal
    cmp rax, rbx
    jl cons_deepcopy_if_num_curried_lt_3
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-24] ; load field pointer
    test rcx, rcx ; skip null field
    je cons_deepcopy_copy_field_null_0
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-24], rax ; store duplicated pointer
    jmp cons_deepcopy_copy_field_done_1
cons_deepcopy_copy_field_null_0:
    xor rax, rax ; null copy result
cons_deepcopy_copy_field_done_1:
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-16] ; load field pointer
    test rcx, rcx ; skip null field
    je cons_deepcopy_copy_field_null_2
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-16], rax ; store duplicated pointer
    jmp cons_deepcopy_copy_field_done_3
cons_deepcopy_copy_field_null_2:
    xor rax, rax ; null copy result
cons_deepcopy_copy_field_done_3:
    mov [rbp-64], rax ; save evaluated scalar in frame
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-8] ; load field pointer
    test rcx, rcx ; skip null field
    je cons_deepcopy_copy_field_null_4
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-8], rax ; store duplicated pointer
    jmp cons_deepcopy_copy_field_done_5
cons_deepcopy_copy_field_null_4:
    xor rax, rax ; null copy result
cons_deepcopy_copy_field_done_5:
    mov [rbp-80], rax ; save evaluated scalar in frame
    jmp cons_deepcopy_done
cons_deepcopy_if_num_curried_lt_3:
    mov rax, [rbp-32] ; load operand
    mov rbx, 2 ; operand literal
    cmp rax, rbx
    jl cons_deepcopy_if_num_curried_lt_2
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-24] ; load field pointer
    test rcx, rcx ; skip null field
    je cons_deepcopy_copy_field_null_6
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-24], rax ; store duplicated pointer
    jmp cons_deepcopy_copy_field_done_7
cons_deepcopy_copy_field_null_6:
    xor rax, rax ; null copy result
cons_deepcopy_copy_field_done_7:
    mov [rbp-96], rax ; save evaluated scalar in frame
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-16] ; load field pointer
    test rcx, rcx ; skip null field
    je cons_deepcopy_copy_field_null_8
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-16], rax ; store duplicated pointer
    jmp cons_deepcopy_copy_field_done_9
cons_deepcopy_copy_field_null_8:
    xor rax, rax ; null copy result
cons_deepcopy_copy_field_done_9:
    mov [rbp-112], rax ; save evaluated scalar in frame
    jmp cons_deepcopy_done
cons_deepcopy_if_num_curried_lt_2:
    mov rax, [rbp-32] ; load operand
    mov rbx, 1 ; operand literal
    cmp rax, rbx
    jl cons_deepcopy_if_num_curried_lt_1
    mov rbx, [rbp-16] ; load env_end pointer for copy
    mov rcx, [rbx-24] ; load field pointer
    test rcx, rcx ; skip null field
    je cons_deepcopy_copy_field_null_10
    mov rdi, rcx ; copy pointer argument for deepcopy
    call deepcopy_heap_ptr ; duplicate heap pointer
    mov [rbx-24], rax ; store duplicated pointer
    jmp cons_deepcopy_copy_field_done_11
cons_deepcopy_copy_field_null_10:
    xor rax, rax ; null copy result
cons_deepcopy_copy_field_done_11:
    mov [rbp-128], rax ; save evaluated scalar in frame
    jmp cons_deepcopy_done
cons_deepcopy_if_num_curried_lt_1:
cons_deepcopy_done:
    leave
    ret

extern deepcopy_heap_ptr
extern release_heap_ptr
