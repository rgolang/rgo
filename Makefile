.PHONY: compile
compile:
	@cargo run -- hello.rgo hello.asm
	@nasm -felf64 hello.asm -o bin/hello.o
	@ld -dynamic-linker /lib64/ld-linux-x86-64.so.2 -lc bin/hello.o -o bin/hello

.PHONY: run
run: compile
	@mkdir -p bin
	@./bin/hello

.PHONY: asm
asm:
	@mkdir -p bin
	@nasm -felf64 hello.asm -o bin/hello.o
	@ld -dynamic-linker /lib64/ld-linux-x86-64.so.2 -lc bin/hello.o -o bin/hello
	@./bin/hello

.PHONY: test
test:
	@cargo test
