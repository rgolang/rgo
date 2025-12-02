# Testing it

## Makefile utility functions

```sh
make compile # compile ./code/hello.rgo
make run # compile and run ./code/hello.rgo
make test # run compiler tests
```

## Docker (skip installation)
```sh
git clone https://github.com/rgolang/rgo.git

# Build the image
docker build -t rgo-compiler .

# Compile a program (Replace $PWD with your code directory)
docker run --rm \
    -v "$PWD":/work \
    -w /work \
    --platform=linux/amd64 \
    rgo-compiler "path-to-your-program.rgo"
```
The resulting executable appears in your local bin/ directory on your host machine.

This is what happens inside the container (or on your linux machine)
```sh
apt-get install -y nasm gcc make
cargo run -- code/hello.rgo hello.asm
nasm -felf64 hello.asm -o bin/hello.o
ld -dynamic-linker /lib64/ld-linux-x86-64.so.2 -lc bin/hello.o -o bin/hello
./bin/hello
```

## Installation

- Install Rust https://rust-lang.org/tools/install/ or using asdf https://github.com/asdf-community/asdf-rust
- Install a NASM compiler https://www.nasm.us/

```sh
apt-get update
apt-get install -y nasm binutils make
```

## Compiling and running code (On Linux/Debian x86_64 arch)
```sh
cargo run -- code/hello.rgo hello.asm
nasm -felf64 hello.asm -o bin/hello.o
ld -dynamic-linker /lib64/ld-linux-x86-64.so.2 -lc bin/hello.o -o bin/hello
./bin/hello
```
