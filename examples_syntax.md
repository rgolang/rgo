# Syntax explained using examples

> A program is an imperative array of commands.

Note: An "array" is simply an ordered "list".

A list:
* one
* two
* three

An array:
1. one
2. two
3. three

Note: Assume all code in the examples below imports the keywords `print` and `syscall`:

```go
{syscall, print}:@"github.com/rgolib/rgo/os"
```

* `syscall`: a global function that calls the `os` to pay attention, it takes no input and provides no output
* `print`: a global function with variable input and no output, writes to `stdout`
---  

## `//` and `/**/` are comments

```go
// this is a single line comment
/* this is a 
multi-line comment */
```

## `()` mathematical notation
```go
(!2)  // 2 (factorial)
(2+3) // 5 (add)
(2*3) // 6 (multiply)
(2^3) // 8 (power of)
```

## `{}` groups
```go
{syscall, syscall}
```
```go
{
    syscall,
    syscall
}
```
Note:
* The embedded `{}` has a new scope and a new stack frame TODO: link to examples_stack.md

## A comma or semicolon separates

```go
syscall   // execute a global function "syscall" 
[syscall] // execute a global function "syscall" 
{syscall} // execute a global function "syscall"
(syscall) // execute a global function "syscall"
syscall, syscall   // execute two global functions "syscall" 
syscall; syscall   // execute two global functions "syscall"
[syscall, syscall] // execute two global functions "syscall" 
{syscall, syscall} // execute two global functions "syscall" 
(syscall, syscall) // execute two global functions "syscall" 
```

They all look the same? They even act the same! But they behave differently under more complicated scenarios.

For example the above can be written in one line:

```
syscall;[syscall];{syscall};(syscall);syscall,syscall;syscall; syscall;[syscall,syscall];{syscall,syscall};(syscall,syscall)
```
But how does it translate to an array of commands?
* `[]` arrays are unwrapped, `[syscall]` becomes `syscall`
* `{}` and `()` are similar to `[]` arrays and also unwrapped

Let's do it in stages:
1. `[a, b, c]` is a array of `a`, `b` and `c`
2. Treat a series of comma `,` or semicolon `;` separated values as an array
   ```go
   syscall,syscall // [syscall,syscall]
   ```

3. Treat a mix of `,` and `;` as a sub-array in an array
   ```go
   syscall;syscall,syscall;syscall // [syscall,[syscall,syscall],syscall]
   ```

4. Convert grouping parenthesis `{}` and `()` to `[]` arrays
   ```go
   {syscall,syscall};{syscall,syscall} // [[syscall,syscall],[syscall,syscall]]
   ```
5. Unwrap the sub-arrays
   ```go
   {syscall,syscall};{syscall,syscall} // [syscall,syscall,syscall,syscall]
   ```

So this example:

```
syscall;[syscall];{syscall};(syscall);syscall,syscall;syscall; syscall;[syscall,syscall];{syscall,syscall};(syscall,syscall)
```

becomes

```
syscall;syscall;syscall;syscall;syscall;syscall;syscall; syscall;syscall;syscall;syscall;syscall;syscall;syscall
```

Note: A space has no special meaning inside lists

```
syscall; syscall;syscall; syscall; syscall;syscall; syscall;  syscall; syscall; syscall; syscall; syscall;syscall;syscall
```

Note: `;` can be replaced with a newline `\n` or carriage return `\r` or both `\r\n`

```
syscall
syscall
syscall
syscall
syscall
syscall
syscall
syscall
syscall
syscall
syscall
syscall
syscall
syscall
```

---  

## A colon declares keywords

A `:` is used to define the preceding value using the value that follows, just like in English.

> An array of items: item1, item2, item3.  
> UAP: unidentified aerial phenomenon.

```go
x:syscall // define keyword `x` as `syscall` in stack 
x;x       // execute syscall twice: syscall;syscall
```

Because of the sub-array unwrapping it can also be written as
```go
x:syscall // define keyword `x` as `syscall` in stack 
x,x       // execute syscall twice: `syscall;syscall`
```

Adding whitespace for readability:

```go
x: syscall // define keyword `x` as `syscall` in stack 
x, x       // execute syscall twice: `syscall;syscall`
```

--- 

## An empty space combines/defines
Space separated values are combined into one using a concept called [currying](https://en.wikipedia.org/wiki/Currying)

```go
x: syscall  
x             // execute `syscall`
print 1       // prints 1 (a `print` accepts unlimited input)
p: print      // declare "p" as "print"
p 1           // prints 1  
p: print 2    // declare "p" as "print 2"
p             // prints 2  
p: print 3 2  // declare "p" as "print 3 2"
p             // prints 32
p 1           // prints 321  
```
---
## `{}` exports keywords
```go
{one: 1} // keyword `one` is exported/unwrapped automatically
print one // print 1  
```

```go
// `x` and `y` are exported to the current scope
{
    x: 2,
    y: 3
}

// same as:
x: 2
y: 3
```

This is useful as it allows us to use less computer memory

```go  
// re-use already allocated memory for `x`
{
    x: 2,
    x: (x + 1)
}
print x // print 3  
```
Note:
* This is memory safe even across multiple threads because the keyword `x` is lazily evaluated TODO: Link to a dedicated page

## `:{}` captures keywords and `.` concatenates/accesses them

```go 
z: {x:2, y:3}  
print z // prints {x:2,y:3}

// same as the above
y.x: 2 // declare z.x as 2  
z.y: 3 // declare z.y as 3 
print z // prints {x:2,y:3}  
```

## `{}` or a series of colons `:` is a tree structure

```go
x: y: z: 1       // {x: {y: {z: 1}}}
{x: {y: {z: 1}}} // x: y: z: 1
``` 

## Lists `:[]` are not trees `:{}`
`{x:2,y:3}` is the same as `[x:2,y:3]`  
but  
```go
z: {x:2, y:3}   
z.x // z.x is 2  
```
is not the same as  
```go
z: [x:2, y:3]   
z.x // z.x is undefined and the compiler will tell you
z.0 // z.0 is `x:2` 
```

This is because:
* `[]` Are arrays first  
* `()`, `{}` Are trees first  

## `.` tree and list accessor

```js
contact: {
    name: "Bob",
    emails: [
        {
            name: "Bob", 
            address: "bob@example.com"
        },
        {
            name: "Robert", 
            address: "bob+2@example.com"
        }
    ]
}
print contact.name             // prints "Bob"
print contact.emails.0.address // prints "bob@example.com"
print contact.emails.name      // prints ["Bob","Robert"]
```

TODO: Experimental patching
```js
contact: {
    name: "Bob",
    emails: [
        {
            name: "Bob", 
            address: "bob@example.com"
        },
        {
            name: "Robert", 
            address: "bob+2@example.com"
        }
    ]
}
print contact.emails.name              // prints ["Bob","Robert"]

contact.emails.name: [, "Bob 2"]
print contact.emails.name              // prints ["Bob","Bob 2"]

contact.emails.name: ["Bob 1", "Bob 2"]
print contact.emails.name              // prints ["Bob 1","Bob 2"]
```
