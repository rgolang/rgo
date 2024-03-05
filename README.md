---
Author: Timo Huovinen
Date: 2024 Mar
---

# Rgo programming language
(pre-alpha version)

A novel approach to programming that combines the elegance of lambda calculus with the pragmatism of statement-based programming. Tailored for flow-based programming, it emphasizes functional programming, immutability, and higher-order functions, all while automating memory and resource management without relying on a garbage collector.

Rgo is like a memory safe version of C and Lisp combined, it is most similar to assembly (but readable and safe) and looks similar to javascript/typescript.

Like brainteasers and math? See how rgo transforms and re-interprets the syntax of lambda calculus [LAMBDA.md](LAMBDA.md)

It currently uses the [LLVM](https://en.wikipedia.org/wiki/LLVM) backend and is compiled using golang, but in the future performance can be improved by compiling directly to the target architecture assembly since LLVM lacks direct stack and heap access that rgo requires.

## Features
- **Compiled**
- **Statically typed**
- **Memory safe**: Automatic compile time memory management through syntax and no garbage collector.
- **A simple syntax**: Syntax is declared exclusively through special characters, built-in functionality is prefixed with `@` making it easy to recognize what keyword is custom and what is part of the syntax, 
- **Familiar syntax**: The syntax attempts to be familiar to developers by making it look more like the most popular programming languages.
- **Functional programming**: built from the ground up to support and encourage functional programming paradigms.
- **Currying**: Functions support currying, encouraging for more flexible code reuse, function composition and to get out of "callback hell".
- **Higher-Order Functions**: Treats functions as first-class citizens, enabling functions that take other functions as arguments or return them as results.
- **Error handling**: Through callback functions

## Anti-features
These purposefully don't exist to prevent user errors and simplify the language and compiler
- **No `return`**: Having returns would make the language a hybrid between expression and statement based, by not having returns the language becomes significantly simpler to compile and many problems related to memory and resource management disappear, it also makes the language very _async_ friendly.
- **No variables**: There are no traditional variables, compile time values can be labelled, runtime values are created by built-in functions and are immutable.

## Documentation

For documentation of [syntax, built-in functions and types](DOCUMENTATION.md)

## Installation/Compiling

To run the code, see [COMPILE.md](COMPILE.md)

## Examples

```c
foo: (){} // a function with no parameters
foo() // a function call

bar: {} // a function with no parameters
bar // a function call
```

### Hello World

```c
x: "World"
@printf("Hello %s\n", x)
```

### Hello user

```c
@printf("What is your name?\n")
@prompt(10, (name: @str){
    @printf("Hello %s\n", name)
})
```

### Arithmetic

```c
@add(3, 5, (res: @int){
    @printf("%d\n", res) // prints 8
})
```

## TODO:
[To-Do](TODO.md)

## License

[Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)

