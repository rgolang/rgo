Maybe wrap macro in <> or ''
'not true' = 'false'

Every = macro is added to compiler dictionary and then the compiler starts a new thread for this new definition that starts reading code from this point along with other routines doing the negex matching
matches for the same code are sent to a channel that stores possible negex free replacements for the same code in the right order.
Phase 2 simply does that search and replace, replacing longer strings first.

TODO:  
= Macro defines at build time  
: Curry defines at runtime  

$0.$1 = Todo:  

Type check?  Send message/email $0 = Todo:  

  
rgo is a declarative logical language // stream first, can mutate so not functional



`.`  Terminal symbols TODO:  
`:;,?!` Non-terminal symbols  TODO:  

TODO: `()` toggles math context? `{}` toggles grammar context?


---

Explain why ('bool' = 'true|false') uses `|` instead of `/`, because it's way more common in functional programming `haskell` (TODO: Verify); `curry` (TODO: Verify) and `typescript`,`php` and `go` and to use `|` for union. `/` might also be confused with `per` and `or`, also by having `|` be build time, `/` can be for runtime `or`.

TODO:
```rust
bool: true/false

// gets run as possible options:

x bool: true?
x bool: false?

// similar to rust match
x match: 
    true(x) => // run code if true
    false(x) => // run code if false
```
* A file to explain how uncertainty is handled
  
---


TODO: Implement negex
TODO: Support all of https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Right_shift_assignment for rgo_js



* `$0.x` can do shadowing and mean `$0.x₁`?
* check what shadowing actually means
* Assembly is a signal processor
* https://en.wikipedia.org/wiki/Parsing_expression_grammar (is this of any use?)
* Doc: that `@` autoimprorts all keywords by default, unless captured `rgo:@"rgo.io/lang/rgo"` explain WHY?


---

are `a` and `an` shadowing?

---

Figure out the `?` functionality
```rust
x < ch?! "fatal: {x}"!
x < ch? // x is end of stream 
x < ch? "this is a default for x", print x. // can use ,; depending on context x < ch? default for x. // Return
```
* `?` and `!` are non terminating? Does this mean that need to use `?.` and `!.` to `return`?

How much was x? It was 55. What was x? It was a bird. (After the fact resolution) What is an x? It's a bird. (Before the fact resolution)

```js
what was x? it was {x: "a bird"}
what is x? it is {x: "a bird"}
x? it is {x: "a bird"}. // TODO: how to type check?
```

Build time? Runtime?

---

`*` bullet point not necessary, anything on new line is a bullet point anyway.


---

How to autocomplete?


* https://en.wikipedia.org/wiki/Stack-oriented_programming


---

lit is more stream friendly than math  
x: 1,2,3 // in math x = [1,2,3]  
x = 1,2,3 // [x=1, 2, 3] // error: 2, 3 are not global functions 



Use the word `array` instead of `list` and declare this in the readme at the very start and remove the declaration from examples_syntax.


---

{postgres}: @"example.com/postgres"
or
{postgres}: @"postgres"
the latter can specify the version and the domain in the package file.
though this means that packages with the same name can't be imported.

---

To magic
`print "hello"`
or not to magic
`print < "hello"`
that is the question.

---

#noshadow
#nolint

---

TODO: The colons and semicolons are all messed up in English, maybe the claim that it looks like English is too much?

---

Assembly:
index: a keyword in the OS

x = usually means index

di = destination index
rdi = read from input to destination index
rax = register accumulator extended

---


* Implement with `<` but remove it and see if it all still makes sense
* Both input and output in the same signature because var names don't clash anyway, implement output part of the interface later
* Use `()` for number based interfaces (numeric signatures) `myfunc (1, 2)` same as `myfunc {$0:1, $1:2}` and `myfunc (x, y) {}` same as `myfunc {x:$0, y:$1}` and `myfunc (x int, y str) {}` same as `myfunc {x:int $0, y:str $1}` 
* Figure out if `()` notation can still be used for math even if it's used for numeric signatures
* Maybe have a structure with many exported functions and applying data to the structure makes it available to all functions. TODO: Instances.
* types, structs, functions, loops are all compiler concepts, program just knows binary and goto
* `int` is a function that can be imported from `rgo` that accepts 1 input and throws an error if it's the wrong type, it's implemented as part of the compiler directly in machine code or in the intermediate representation language just like the `{}():=` symbols
* `int`, `i32`, `i64`, `uint`, `u32`, `u64`, `str`, `s32`??, `s64`?? `[128]s` and `[128]i` and `[128]u` (The amount of memory a string takes up by default), maybe use `0iFF`, `0uFF` and `0sFF`? or `0xFF`, `1xFF`, `2xFF`
* `int` can be represented as raw bytes `0xFF` `[0,0xFF]` or whatever because of protobuf
* `str` can be represented as raw bytes `0xFF` `[1,0xFF]` or whatever because of protobuf
* Borrowing does not provide transactions, neither do channels, but both can accept a function that is dynamic and a transaction.
* Curry allows passing only one required var and then the rest later.
* Threads are a group of workers and kanban is a channel of tasks.
* In math notation you must declare a variable with a value and no type. (Does this mean mathland doesn't have strings?) Untyped strings treated as very long numbers (because it's just bits)
* Protobuf approach to strings and ints instead of having a terminator `\0` like `C`


```go 
eat 1 apple
say "hello world"
say "hello" "world" // two calls? no, it's composing.
say "hello", "world" // two calls?
```

TODO:
The language is language agnostic as much as possible, only these symbols are used:

`utf-8` a keyword/variable
` ` 
`,`
`!`
`?`
`;`
`:`
`()`
`[]`
`{}`
`…`
`@`
`#` (as a hashtag)
`math`
`%`

and everything else is based on them.

Hex notation is also language specific.



