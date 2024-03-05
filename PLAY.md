## Installation

- Install Golang https://go.dev/doc/install or using asdf https://github.com/asdf-community/asdf-golang
- Install LLVM https://releases.llvm.org/download.html

## Compiling and running code
```sh
go mod tidy
go run main.go "./code/hello.rgo"
mkdir -p ./bin
llc -filetype=obj -relocation-model=pic -opaque-pointers ./code/hello.ll -o ./bin/hello.o
gcc ./bin/hello.o -o ./bin/hello
```

## Makefile utility functions

```sh
make compile # compile ./code/hello.rgo
make run # compile and run ./code/hello.rgo
make test # run compiler go tests
make llvm # compile and run ./code/hello.ll
make debug # run and debug ./code/hello.ll with break points
make optimize # optimize ./code/hello.ll to ./code/hello_optimized.ll
make testgen # generate go tests based on the files in ./testgen
make bench # benchmark ./code/hello.ll using valgrind
```
