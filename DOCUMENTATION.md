## Syntax

The list of special characters is very small `@:(){}!`

### `:`
For labelling functions, literals (string/int/float) and parameters can be labelled like so:
```
label: declaration
```
Labelling is purposefully only available for values that are present at compile time, this includes function parameters because their type is declared at compile time.

### `(){}`

Functions look like `(){}` or `{}`, where the parameters go inside `()` and the body (inside `{}`) contains a list of newline or comma separated statements.  
A statement can be a function call or a label operation, but not both.  
When a statement is a function call, the function must have been provided all of its arguments.

### `@`
For importing built-in functions.  
Anything starting with `@` is built into the language.

### `!` 
Compile time flag for types, this enforces the function argument to be available at compile time

## Built-in types
### `@int`
### `@str`

## Built-in functions
### `@printf`
Accepts a compile time 
```c
(fmt: @str!, ..., ok: (@str))
```

The first parameter is a string literal that affects the function signature, it uses the libc `printf` specifier notation.

Example:
```c
num: 10
str: "hi"
@printf("%d %s\n", num, str)
```

### `@prompt`
Reads a certain number of characters into the stack and provides it to a callback function

```c
(limit: @int!, ok: (@str))
```

Example:
```c
@prompt(10, (input: @str){
    @printf("%s\n", input)
})
```

### `@ieq`
int equals  
Signature:
```c
(x: @int, y: @int, true: (), false: ())
```
LLVM signature:
```c
define void @.ieq(i32 %x, i32 %y, void ()* %true, void ()* %false)
```
Example:
```c
@ieq(1, 1, (){
    @printf("they are equal\n")
})
```

### `@igt`
int greater than  
Signature:
```c
(x: @int, y: @int, true: (), false: ())
```
LLVM signature:
```c
define void @.igt(i32 %x, i32 %y, void ()* %true, void ()* %false)
```

### `@add`
add two numbers
Signature:
```c
(x: @int, y: @int, ok: (@int))
```
LLVM signature:
```c
define void @.add(i32 %x, i32 %y, void (i32)* %ok)
```
Example:
```c
@add(1, 1, (res: @int){
    @printf("%d\n", res) // prints 2
})
```

### `@mul`
multiply two numbers
Signature:
```c
(x: @int, y: @int, ok: (@int))
```
LLVM signature:
```c
define void @.mul(i32 %x, i32 %y, void (i32)* %ok)
```
Example:
```c
@add(1, 1, (res: @int){
    @printf("%d\n", res) // prints 1
})
```

## `if` function
Staying true to lambda calculus, `if` is a function and it's implemented like this:

```c
if: (cond: ((),()), ok:()){
    cond(ok, {})
}
if(@igt(4, 3), {
    @printf("More\n")
})
```

Internally the compiler may optimize this to use assembly mechanisms like conditional jumps between blocks

## Currying

### Hello World curried
```c
// Define a greeting function
hello: (s: @str, name: @str){
    @printf("Hello %s %s\n", s, name)
}
// Curry the greeting function
greet: hello("Dear") // Prepare the call
helloBob: greet("Bob")
helloBob() // Execute the call
```

### Arithmetic curried

```c
baz: (x: @int) {
    ok: @add(3, x)
    ok((res: @int){
        @printf("%d\n", res)
    })
}
baz(5) // prints "8"
```

