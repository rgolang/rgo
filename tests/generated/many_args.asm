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
    sub rsp, 400 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov [rbp-32], rsi ; store scalar arg in frame
    mov [rbp-48], rdx ; store scalar arg in frame
    mov [rbp-64], rcx ; store scalar arg in frame
    mov [rbp-80], r8 ; store scalar arg in frame
    mov [rbp-96], r9 ; store scalar arg in frame
    mov [rbp-112], r10 ; store scalar arg in frame
    mov [rbp-128], r11 ; store scalar arg in frame
    mov [rbp-144], r12 ; store scalar arg in frame
    mov [rbp-160], r13 ; store scalar arg in frame
    mov [rbp-176], r14 ; store scalar arg in frame
    mov [rbp-192], r15 ; store scalar arg in frame
    mov rax, [rbp+8] ; load spilled scalar arg
    mov [rbp-208], rax ; store spilled scalar arg
    mov rax, [rbp+16] ; load spilled scalar arg
    mov [rbp-224], rax ; store spilled scalar arg
    mov rax, [rbp+24] ; load spilled scalar arg
    mov [rbp-240], rax ; store spilled scalar arg
    mov rax, [rbp+32] ; load spilled scalar arg
    mov [rbp-256], rax ; store spilled scalar arg
    mov rax, [rbp+40] ; load spilled scalar arg
    mov [rbp-272], rax ; store spilled scalar arg
    mov rax, [rbp+48] ; load spilled scalar arg
    mov [rbp-288], rax ; store spilled scalar arg
    mov rax, [rbp+56] ; load spilled scalar arg
    mov [rbp-304], rax ; store spilled scalar arg
    mov rax, [rbp+64] ; load spilled scalar arg
    mov [rbp-320], rax ; store spilled scalar arg
    mov rax, [rbp+72] ; load spilled scalar arg
    mov [rbp-336], rax ; store spilled scalar arg
    mov rax, [rbp+80] ; load spilled scalar arg
    mov [rbp-352], rax ; store spilled scalar arg
    mov rax, [rbp+88] ; load spilled closure code
    mov rdx, [rbp+96] ; load spilled closure env
    mov [rbp-368], rdx ; save spilled closure env_end pointer
    lea rax, [rel _0] ; point to string literal
    mov [rbp-384], rax ; save evaluated scalar in frame
    mov rax, [rbp-384] ; load scalar from frame
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
    mov [rbp-400], rax ; save evaluated scalar in frame
    mov rdx, [rbp-368] ; load closure env_end for exec
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
    sub rsp, 384 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-184] ; load scalar env field
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-176] ; load scalar env field
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-168] ; load scalar env field
    mov [rbp-64], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-160] ; load scalar env field
    mov [rbp-80], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-152] ; load scalar env field
    mov [rbp-96], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-144] ; load scalar env field
    mov [rbp-112], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-136] ; load scalar env field
    mov [rbp-128], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-128] ; load scalar env field
    mov [rbp-144], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-120] ; load scalar env field
    mov [rbp-160], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-112] ; load scalar env field
    mov [rbp-176], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-104] ; load scalar env field
    mov [rbp-192], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-96] ; load scalar env field
    mov [rbp-208], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-88] ; load scalar env field
    mov [rbp-224], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-80] ; load scalar env field
    mov [rbp-240], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-72] ; load scalar env field
    mov [rbp-256], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-64] ; load scalar env field
    mov [rbp-272], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-56] ; load scalar env field
    mov [rbp-288], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-48] ; load scalar env field
    mov [rbp-304], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-40] ; load scalar env field
    mov [rbp-320], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-32] ; load scalar env field
    mov [rbp-336], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-24] ; load scalar env field
    mov [rbp-352], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov rax, [rax-16] ; load scalar env field
    mov [rbp-368], rax ; save evaluated scalar in frame
    mov rax, [rbp-16] ; load scalar from frame
    mov r10, rax ; env_end pointer for closure field
    mov rdx, [r10-8] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    mov [rbp-384], rdx ; update closure env_end pointer
    mov rdi, [rbp-16] ; load closure env_end pointer
    call release_heap_ptr ; release closure environment
    mov rdx, [rbp-384] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-368] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-352] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-336] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-320] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-304] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-288] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-272] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-256] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-240] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-224] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-208] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-192] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-176] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-160] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-144] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-128] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-112] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-96] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-80] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-64] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-48] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore scalar arg into register
    pop rdx ; restore scalar arg into register
    pop rcx ; restore scalar arg into register
    pop r8 ; restore scalar arg into register
    pop r9 ; restore scalar arg into register
    pop r10 ; restore scalar arg into register
    pop r11 ; restore scalar arg into register
    pop r12 ; restore scalar arg into register
    pop r13 ; restore scalar arg into register
    pop r14 ; restore scalar arg into register
    pop r15 ; restore scalar arg into register
    sub rsp, 8 ; allocate slot for saved rbp
    mov rax, [rbp] ; capture parent rbp
    mov [rsp], rax ; stash parent rbp for leave
    mov rbp, rsp ; treat slot as current rbp
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

global _24_lambda
_24_lambda:
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
global _24_lambda_unwrapper
_24_lambda_unwrapper:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    mov rdi, [rbp-16] ; load closure env_end pointer
    call release_heap_ptr ; release closure environment
    leave ; unwind before named jump
    jmp _24_lambda ; jump to fully applied function
global _24_lambda_release
_24_lambda_release:
    push rbp ; save executor frame pointer
    mov rbp, rsp ; establish new frame base
    sub rsp, 16 ; reserve stack space for locals
    mov [rbp-16], rdi ; store scalar arg in frame
    leave
    ret

global _24_lambda_deepcopy
_24_lambda_deepcopy:
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
    sub rsp, 368 ; reserve stack space for locals
    mov rax, 1 ; load literal integer
    mov [rbp-16], rax ; save evaluated scalar in frame
    mov rax, 2 ; load literal integer
    mov [rbp-32], rax ; save evaluated scalar in frame
    mov rax, 3 ; load literal integer
    mov [rbp-48], rax ; save evaluated scalar in frame
    mov rax, 4 ; load literal integer
    mov [rbp-64], rax ; save evaluated scalar in frame
    mov rax, 5 ; load literal integer
    mov [rbp-80], rax ; save evaluated scalar in frame
    mov rax, 6 ; load literal integer
    mov [rbp-96], rax ; save evaluated scalar in frame
    mov rax, 7 ; load literal integer
    mov [rbp-112], rax ; save evaluated scalar in frame
    mov rax, 8 ; load literal integer
    mov [rbp-128], rax ; save evaluated scalar in frame
    mov rax, 9 ; load literal integer
    mov [rbp-144], rax ; save evaluated scalar in frame
    mov rax, 10 ; load literal integer
    mov [rbp-160], rax ; save evaluated scalar in frame
    mov rax, 11 ; load literal integer
    mov [rbp-176], rax ; save evaluated scalar in frame
    mov rax, 12 ; load literal integer
    mov [rbp-192], rax ; save evaluated scalar in frame
    mov rax, 13 ; load literal integer
    mov [rbp-208], rax ; save evaluated scalar in frame
    mov rax, 14 ; load literal integer
    mov [rbp-224], rax ; save evaluated scalar in frame
    mov rax, 15 ; load literal integer
    mov [rbp-240], rax ; save evaluated scalar in frame
    mov rax, 16 ; load literal integer
    mov [rbp-256], rax ; save evaluated scalar in frame
    mov rax, 17 ; load literal integer
    mov [rbp-272], rax ; save evaluated scalar in frame
    mov rax, 18 ; load literal integer
    mov [rbp-288], rax ; save evaluated scalar in frame
    mov rax, 19 ; load literal integer
    mov [rbp-304], rax ; save evaluated scalar in frame
    mov rax, 20 ; load literal integer
    mov [rbp-320], rax ; save evaluated scalar in frame
    mov rax, 21 ; load literal integer
    mov [rbp-336], rax ; save evaluated scalar in frame
    mov rax, 22 ; load literal integer
    mov [rbp-352], rax ; save evaluated scalar in frame
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
    mov rax, _24_lambda_unwrapper ; load unwrapper entry point
    mov qword [rdx+0], rax ; store unwrapper entry in metadata
    mov rax, _24_lambda_release ; load release helper entry point
    mov qword [rdx+8], rax ; store release pointer in metadata
    mov rax, _24_lambda_deepcopy ; load deep copy helper entry point
    mov qword [rdx+16], rax ; store deep copy pointer in metadata
    xor rax, rax ; zero num_curried metadata
    mov qword [rdx+40], rax ; store num_curried
    mov [rbp-368], rdx ; update closure env_end pointer
    mov rdx, [rbp-368] ; load closure env_end pointer
    mov rax, [rdx+0] ; load closure unwrapper entry point
    push rdx ; stack arg: closure env_end
    push rax ; stack arg: closure code
    mov rax, [rbp-352] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-336] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-320] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-304] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-288] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-272] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-256] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-240] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-224] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-208] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-192] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-176] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-160] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-144] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-128] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-112] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-96] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-80] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-64] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-48] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-32] ; load scalar from frame
    push rax ; stack arg: scalar
    mov rax, [rbp-16] ; load scalar from frame
    push rax ; stack arg: scalar
    pop rdi ; restore scalar arg into register
    pop rsi ; restore scalar arg into register
    pop rdx ; restore scalar arg into register
    pop rcx ; restore scalar arg into register
    pop r8 ; restore scalar arg into register
    pop r9 ; restore scalar arg into register
    pop r10 ; restore scalar arg into register
    pop r11 ; restore scalar arg into register
    pop r12 ; restore scalar arg into register
    pop r13 ; restore scalar arg into register
    pop r14 ; restore scalar arg into register
    pop r15 ; restore scalar arg into register
    sub rsp, 8 ; allocate slot for saved rbp
    mov rax, [rbp] ; capture parent rbp
    mov [rsp], rax ; stash parent rbp for leave
    mov rbp, rsp ; treat slot as current rbp
    leave ; unwind before named jump
    jmp foo ; jump to fully applied function
extern deepcopy_heap_ptr
extern release_heap_ptr
extern write
section .rodata
_0:
    db "All arguments received successfully.", 10, 0
