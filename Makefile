.PHONY: compile
compile:
	@cargo run -- code/hello.rgo code/hello.asm
	@nasm -felf64 code/hello.asm -o bin/hello.o
	@ld -dynamic-linker /lib64/ld-linux-x86-64.so.2 -lc bin/hello.o -o bin/hello

.PHONY: run
run: compile
	@mkdir -p bin
	@./bin/hello

.PHONY: asm
asm:
	@mkdir -p bin
	@nasm -felf64 code/hello.asm -o bin/hello.o
	@ld -dynamic-linker /lib64/ld-linux-x86-64.so.2 -lc bin/hello.o -o bin/hello
	@./bin/hello

.PHONY: hir
hir:
	@cargo run --bin render_hir -- code/hello.rgo code/hello.hir.rgo

.PHONY: mir
mir:
	@cargo run --bin render_mir -- code/hello.rgo code/hello.mir

.PHONY: test
test:
	@cargo test
