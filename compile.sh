#!/bin/bash
set -e

file="$1"

if [ -z "$file" ]; then
    echo "Usage: compile.sh <file.rgo>"
    exit 1
fi

base="${file%.rgo}"

# Source -> assembly
/rgo/target/release/rgo "$file" "$base.asm"

# NASM: assembly -> object file
nasm -felf64 "$base.asm" -o "$base.o"

# Object -> native binary
ld -dynamic-linker /lib64/ld-linux-x86-64.so.2 -lc "$base.o" -o "$base"

echo "Built $base"
