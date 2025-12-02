# Compiler

This is a small experimental programming language focused on simplicity, predictability, and explicit semantics. The compiler is written in Rust and lowers Rgo programs directly to NASM AMD64 assembly, producing ELF binaries that run on any AMD64 Linux system with hooks to standard libc (no LLVM, no JIT, and no garbage collector).

The grammar file lives in: [grammar.peg](./grammar.peg)

## Highlights

- **Continuation-Passing Style (CPS)**: Every label ends with a tail transfer to its continuation, enabling predictable control flow, no stack frames.
- **Deterministic memory model**: Closure environments and other allocations use mmap/munmap. The compiler manages lifetimes, so no tracing GC or manual free is required.
- **Strictly typed**: All interfaces, closure shapes, and continuation types are explicit and checked at compile time.
- **Punctuation-driven syntax**: A minimal surface language that stays readable while keeping the parser and backend fast.
- **No keywords**: There are no built-ins like `let`, `fn`, `if`, or `struct`, every semantic construct arises from punctuation and continuation form.
- **First-class functions**: Every value is passed explicitly, closures are automatically curried and lowered to environment structures.
- **Unicode-aware frontend**: UTF-8 literals and identifiers work as expected while the grammar stays ASCII-friendly.
- **Direct compile-to-assembly backend**: Deterministic performance, tiny runtime footprint, full control over calling conventions and memory layout.

# Example
```
@printf
@exit

name: "Alice"
printf("hello %s", name, exit(0))
```

## Execution Model & Core Semantics

It is a **identifier-driven, definition-oriented, continuation-passing language**.  
Programs are built from two fundamental actions:

- **defining identifiers for values**, and  
- **transferring control to identifier** that interpret those definitions.

There are no expressions, operators or return values, all computation is a sequence of definitions followed by a transfer of control to another function.

### Definition

A definition introduces a name for a value inside the current scope:
```
@printf

name: "Bob"
foo: (ok:()){
   printf("hello %s", name, ok)
}
```

### Execution

```
@exit
@printf

name: "Bob"
foo: (ok:()){
   printf("hello %s", name, ok)
}
end: exit(0)
foo(end)
```

This syntax `foo(end)` does **not** imply a C-style function call.  
Instead, the parser interprets the form in one of two ways:

1. **Argument application ([currying](https://en.wikipedia.org/wiki/Currying))**  
When application appears on the right-hand side of a definition `end: exit(0)` or as a direct argument `foo(exit(0))`
it is treated as applying arguments to a closure. No execution happens at this point, it creates an applied closure that can be executed later.

2. **Tail jump (control transfer)**
When application appears as a standalone action in a block .e.g `foo(end)`
it is compiled as a tail jump to `foo`. Control transfers directly to `foo`
and never returns to the current location.

## Avoiding Deeply Nested Control Flow
Languages that rely on embedding functions inside functions often produce deeply
nested control structures, sometimes referred to as "callback hell." A typical
nested style (shown here in pseudocode) looks like:

```
read("a.txt", (a:str) {
    read("b.txt", (b:str) {
        process(a, b, (result:str) {
            write("out.txt", result, (code:int) {
                exit(code)
            })
        })
    })
})
```
Each operation encloses the next, causing the structure to collapse inward.

Rgo avoids such nesting through **scope capture** using `=`.
The operator does not assign or mutate, it transforms the remainder of the block into a continuation that receives the named value.

The same logical flow becomes:
```
(a:str) = read("a.txt")
(b:str) = read("b.txt")
(result:str) = process(a, b)
(code:int) = write("out.txt", result)
exit(code)
```

Each line shapes the continuation: `read("a.txt")` continues into the remainder
of the block with `a:str` in scope, then `read("b.txt")` continues with `b:str`,
and so on. The remaining scope is repeatedly captured and threaded forward,
so control flow stays flat and easy to follow instead of nesting deeper with
each dependent step.

This is done purely through syntax sugar.

### Lambda Calculus as an Operational Machine Model

What if we adopt a fully operational view of [lambda calculus](https://en.wikipedia.org/wiki/Lambda_calculus), where every term is an executable computation rather than a value-denoting expression?

Under this interpretation, the lambda calculus effectively becomes:
- a **minimal machine** model much closer to assembly than to high-level mathematics
- a **control-flow graph** where substitution acts as a jump with an extended environment
- a **small-step abstract machine** (CEK, Krivine, etc.) but with memory management without a garbage collector.
- a **rewriting interpreter** whose only instruction is β-reduction (providing arguments to functions).

The idea was to make this operational structure explicit and statically checked, while presenting it using a familiar C-family surface syntax (inspired by JavaScript, TypeScript, Go, Rust).

Programs are then lowered directly into tail-jump CPS and compiled straight to assembly.

## Quick start (Using Docker)

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

## Building and testing

- Rebuild the compiler or run the golden snapshot suite with `cargo test`. This also executes `tests/golden_test.rs`, which regenerates snapshots under `tests/generated/`:
  - `*.asm` contains the final NASM output.
  - `*.asm.txt` records the pseudo-assembly that feeds the final backend.
  - `*.hir.rgo` is the normalized high-level IR after parsing.
  - `*.hir.debug.txt` shows the HIR structure.
  - `*.txt` captures the parser AST dump.
- Whenever you change the compiler or templates that affect these snapshots, re-run `cargo test` and check the updated files into source control if they reflect expected behavior.

## Project structure

- `src/`: Rust implementation of the lexer, parser, HIR, and back-end code generator.
- `code/`: sample Rgo programs (`hello.rgo` is a friendly starting point with Makefile shortcuts).
- `tests/`: integration and golden snapshot tests; `golden_test.rs` is the automated snapshot generator.
- [SEMANTICS.md](SEMANTICS.md) describes runtime expectations and language rules.

## Notes for contributors

- The compiler is intentionally small: prefer clarity over clever macros.
- Document any new language surface you add so users can follow the same mental model.

## Current Limitations & Roadmap Notes

This language is still in an early experimental phase, and several subsystems are intentionally minimal or entirely missing. The following areas are not yet implemented:

- No optimizations  
The backend currently emits straightforward CPS-lowered NASM without peephole passes, register allocation strategies, inlining, or dead-code elimination. Output is correct but not optimized.
- No floating-point support  
The type system and backend only handle integers and pointers today. Floating-point literals, arithmetic, and ABI conventions remain unimplemented.
- No math library  
Functions such as sin, cos, sqrt, and friends are not yet exposed. Interfacing to libm and defining a typed surface for it are planned but currently absent.
- No arrays or slices  
Aggregate data structures are not yet supported. There is no syntax or type-level encoding for contiguous memory layouts, indexing, or bounds semantics.
- Minimal runtime surface  
At present, the only “standard library” consists of printf, sprintf, and arbitrary native NASM instructions. Everything else must be built manually.

Despite that, functionality is slowly expanding, and the compiler architecture is structured so these features can be added piece by piece while keeping the language’s core goals (simplicity, explicitness, and predictability) intact.

## License

[Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)
