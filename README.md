---
Author: Timo Huovinen
Date: 2022 Dec
---

# rgo
(pre-alpha version)  
`main.rgo`

An ergonomic programming language/data interchange format made with human readability in mind.

The core idea is that it separates mathematical notation from grammatical to make it more familiar to all people that can read English.

>Easy to start, hard to master.

Mathematical notation is separated from code grammar using round brackets `()`, for example:

```rgo
grammatical code (mathematical code) grammatical code 
```
This unlocks powerful grammar and allows the language to exist without keywords.

## Hello World

```rust
{print} @ "github.com/thlib/rgo/os"
{f format} @ "github.com/thlib/rgo/text"
x: "World"
print f"Hello {x}"
```

For more examples see: [EXAMPLES.md](EXAMPLES.md)

## Syntax

* `a` keyword `a` 
* `1` number `1`
* ` ` currying
* `<` move
* `:` define a keyword
* `;` nest a list
* `,` make a list
* `!` error
* `?` if then
* `.` end/return/accessor/concatenation
* `#` hashtag
* `@` import
* `$` placeholder
* `%` probability
* `/` or
* `\` escape
* `_` visual separator
* `-` range
* `=` macro equals
* `''` macro placeholder
* `{}` dictionary
* `[]` list
* `()` math
* `(a)` keyword `a`
* `(1)` number `1`
* `( )` currying
* `(<)` less than
* `(>)` greater than
* `(<=)` less than or equals to
* `(>=)` greater than or equals to
* `(==)` equals
* `(~)` approximate
* `(;)` nest a list
* `(,)` make a list
* `(=)` set the value of
* `(%)` percentage
* `(/)` per
* `(_)` visual separator
* `(-)` minus
* `(+)` plus
* `(*)` multiply
* `({})` dictionary
* `([])` list
* `(())` group

The meaning above is approximate, for a more accurate and detailed explanation of the syntax, see: [SYNTAX.md](SYNTAX.md)

## License

[Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)


— Timo Huovinen


