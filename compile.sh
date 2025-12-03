#!/bin/sh
set -e

# If no args or "-" → read RGO source from stdin
if [ -z "$1" ] || [ "$1" = "-" ]; then
    file="stdin_input.rgo"
    cat > "$file"
else
    file="$1"
fi

base="${file%.rgo}"

# Compile source → assembly
/usr/local/cargo/bin/compiler "$file" "$base.asm"

# Assemble
nasm -felf64 "$base.asm" -o "$base.o"

# Link
ld -dynamic-linker /lib64/ld-linux-x86-64.so.2 -lc "$base.o" -o "$base"

chmod +x "$base"
./"$base"

exit $?
