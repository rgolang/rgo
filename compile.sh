#!/bin/sh
set -e

usage() {
    echo "usage: rgo-compile <input.rgo|-> <target>" >&2
    exit 64
}

if [ "$#" -ne 2 ]; then
    usage
fi

target="$2"

# If input is "-" read RGO source from stdin.
if [ "$1" = "-" ]; then
    file="stdin_input.rgo"
    cat > "$file"
else
    file="$1"
fi

base="${file%.rgo}"

# Compile source → assembly
/usr/local/cargo/bin/compiler "$file" "$target" "$base.asm"

# Assemble
nasm -felf64 "$base.asm" -o "$base.o"

# Link
ld -dynamic-linker /lib64/ld-linux-x86-64.so.2 -lc "$base.o" -o "$base"

chmod +x "$base"
./"$base"

exit $?
