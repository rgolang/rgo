.PHONY: compile
compile:
	@go run main.go "./code/hello.rgo"
	@llc -filetype=obj -relocation-model=pic -opaque-pointers ./code/hello.ll -o ./bin/hello.o
	@gcc ./bin/hello.o -o ./bin/hello

.PHONY: run
run: compile
	@./bin/hello
	@rm ./bin/hello
	@rm ./bin/hello.o

.PHONY: test
test:
	go test -count=1 ./...

.PHONY: llvm
llvm:
	@llc -O0 -filetype=obj -relocation-model=pic -opaque-pointers ./code/hello.ll -o ./bin/hello.o # -O0 means no optimization
	@gcc ./bin/hello.o -o ./bin/hello
	@rm ./bin/hello.o
	@./bin/hello

.PHONY: debug
debug:
	llc -filetype=obj -relocation-model=pic -opaque-pointers ./code/hello.ll -o ./bin/hello.o
	clang -g ./code/hello.o -o ./code/hello
	lldb ./code/hello
	# breakpoint set --name callback
	# run
	# dt
	# thread backtrace

.PHONY: optimize
optimize:
	@opt -O3 ./code/hello.ll -S -o ./code/hello_optimized.ll # -S means output assembly
	@llc -filetype=obj -relocation-model=pic ./code/hello_optimized.ll -o ./bin/hello_optimized.o
	@gcc ./bin/hello_optimized.o -o ./bin/hello_optimized
	@./bin/hello_optimized
	@rm ./bin/hello_optimized.o

.PHONY: testgen
testgen:
	go run testgen/main.go "./testgen"

.PHONY: bench
bench:
	clang -O0 ./code/hello.ll -o ./bin/hello
	valgrind --tool=massif --massif-out-file=./bin/massif.out ./bin/hello

