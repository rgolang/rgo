---
Author: Timo Huovinen
Date: 2022 Dec
---

# rgo
(pre-alpha version)  
`main.rgo`

Rgo is a self describing declarative logical language.  

An ergonomic programming language/data interchange format made with human readability in mind.

>Easy to start, hard to master.

See [parsing.md](parsing.md) for how it works.

The core ideas:
* Is consistent
* Separates mathematical notation from grammatical using `{}` and `()` to make it more familiar to all people that can read English.

    ```
    {grammatical code (mathematical code {grammatical code} mathematical code) grammatical code}
    ```
* Is keyword free and extendable
* Is memory safe and concurrent

This unlocks powerful grammar and allows the language to exist without keywords.  
For more detailed examples, see: [examples_syntax.md](examples_syntax.md)

## Hello World

```rust
{print}: @ "github.com/rgolib/rgo/os"
{f format}: @ "github.com/rgolib/rgo/text"
x: "World"
print f"Hello {x}"
```

For more examples see: [examples.md](examples.md)

## Syntax

In grammatical context `{}`:
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
* `~` approximate/omit
* `=` macro equals
* `''` macro placeholder
* `{}` dictionary
* `[]` list
* `()` math

In mathematical context `()`:
* `a` keyword `a`
* `1` number `1`
* ` ` currying
* `<` less than
* `>` greater than
* `<=` less than or equals to
* `>=` greater than or equals to
* `==` equals
* `~` similar to
* `;` nest a list
* `,` make a list
* `=` is equal to
* `%` percentage
* `/` divide
* `_` visual separator
* `-` minus
* `+` plus
* `*` multiply
* `{}` a set
* `[]` a list
* `()` a group

The meaning above is approximate, for a more accurate and detailed explanation of the syntax, see: [syntax.md](syntax.md)

## License

[Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)


— Timo Huovinen


