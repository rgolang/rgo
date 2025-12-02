
# Lambda-calculus

https://en.wikipedia.org/wiki/Lambda_calculus

>Lambda calculus (also written as λ-calculus) is a formal system in mathematical logic for expressing computation based on function abstraction and application using variable binding and substitution. It is a universal model of computation that can be used to simulate any Turing machine. It was introduced by the mathematician Alonzo Church in the 1930s as part of his research into the foundations of mathematics.

It supports only one input and output, the function parameter and body is separated using `.`, units can be grouped using parenthesis `()`, some of its fundamental features are https://en.wikipedia.org/wiki/Currying and https://en.wikipedia.org/wiki/First-class_function

## Intro

Starting with pure lambda calculus with a predefined `@add` function and literal integer support `1`,`2`,`3`:  
```js
(λa.λb.@add a b) 5 3
```
Showing how it evaluates one line at a time:
```js
(λa.λb.@add a b) 5 3
(λb.@add 5 b) 3
@add 5 3
8
```

A more complicated example with functions as arguments and a predefined `@mul` function. (each line shows how it's evaluated)
```js
(λf.λv.(λa.λb.f (@add a b)) (@add 2 3) 3) (λz.@mul 2 z) 0
(λv.(λa.λb.(λz.@mul 2 z) (@add a b)) (@add 2 3) 3) 0
(λa.λb.(λz.@mul 2 z) (@add a b)) (@add 2 3) 3
(λb.(λz.@mul 2 z) (@add (@add 2 3) b)) 3
(λz.@mul 2 z) (@add (@add 2 3) 3)
@mul 2 (@add (@add 2 3) 3) 
@mul 2 (@add 5 3)
@mul 2 8
16 
```

## Step by step transformation

**Use ^**: Replace `λ` with `^` to make it easier to type
```c
(^f.^v.(^a.^b.f (@add a b)) (@add 2 3) 3) (^z.@mul 2 z) 0
```

**No return**: Functions can't and don't return, no implicit return, no expressions.  
The functions now execute zero or more state changing statements.  
Let's use `@printint` as a built in function to change the state of the OS that is running this lambda calculus, `@add` and `@mul` functions now accept a callback function to obtain the result of their calculation.  

```js
(λa.λb.@add a b @printint) 5 3
(λb.@add 5 b @printint) 3
@add 5 3 @printint // prints 8
```

The original complicated example becomes:
```js
(^f.^v.@add 2 3 (^a.@add a 3 f)) (^z.@mul 2 z @printint) 0
(^v.@add 2 3 (^a.@add a 3 (^z.@mul 2 z @printint))) 0
@add 2 3 (^a.@add a 3 (^z.@mul 2 z @printint))
@add 5 3 (^z.@mul 2 z @printint)
@mul 2 8 @printint // prints 16
```

**Shadowing**: Callback functions can shadow variables.
`a` is shadowed multiple times in this example:
```js
(^f.^a.@add 2 3 (^a.@add a 3 f)) (^a.@mul 2 z @printint) 0
(^a.@add 2 3 (^a.@add a 3 (^a.@mul 2 a @printint))) 0
@add 2 3 (^a.@add a 3 (^a.@mul 2 a @printint))
@add 5 3 (^a.@mul 2 a @printint)
@mul 2 8 @printint // prints 16
```

**Remove the dot**: Use explicit curly brackets for the body instead
```js
(^f{^v{@add 2 3 (^a{@add a 3 f})}}) (^z{@mul 2 z @printint}) 0
```

**Params**: the parameter must be inside parenthesis `()`, can use comma `,` instead of traditional `.` to group a sequence of function returns. 
```js
((^f,^v){@add 2 3 ((^a){@add a 3 f})}) ((^z){@mul 2 z @printint}) 0
```

**Args in parenthesis**: Args must now also be in parenthesis `()`, separated by `,`
```js
(^f,^v){@add(2,3,(^a){@add(a,3,f)})}((^z){@mul(2,z,@printint)},0)
```

**Whitespace doesn't matter**: Any whitespace doesn't matter.
```js
(^f, ^v){@add(2, 3, (^a){
    @add(a, 3, f)
})}((^z){
    @mul(2, z, @printint)
}, 0)
```

**Constants**: Inside the function body you can declare constants (functions and literals), as long as they don't clash with a parameter name.
```js
printdouble: (^z){
    @mul(2, z, @printint)
}
calculate: (^f, ^v){
    @add(2, 3, (^a){
        @add(a, 3, f)
    })
}
calculate(printdouble, 0)
```

**Types**: Introduce types, for example `@int` and `@uint`, `(@int)` is a function signature for a function that accepts an `@int`, they are required for params, remove `^` as it's not needed anymore.
```c
printdouble: (z: @int){
    @mul(2, z, @printint)
}
calculate: (f: (@int), v: @int){
    @add(2, 3, (a: @int){
        @add(a, 3, f)
    })
}
calculate(printdouble, 0)
```

**Import builtins**: Explicitly import `add` and `mul`
```js
@int
@add
@mul

printdouble: (z: int){
    mul(2, z, @printint)
}
calculate: (f: (int), v: int){
    add(2, 3, (a: int){
        add(a, 3, f)
    })
}
calculate(printdouble, 0)
```

**Function type**: 
`(int)` is a function signature for a function that accepts an `int`     
`((int))` is a function signature for a function that accepts a function that accepts an `int`  
`()` is a function signature for a function that has no params.  
`(())` is a function signature for a function that accepts a function that has no params.


**Currying**: Can curry functions to create new functions.
```c
@int
@mul
@add
@printint

printdouble: (z: int){
    mul(2, z, printint)
}
calculate: (f: (int), v: int){
    add(2, 3, (a: int){
        add3: add(a, 3)
        add3(f)
    })
}
run: calculate(printdouble)
run(0)
```
