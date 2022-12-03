# rgo
(pre-alpha version)  
`main.rgo`

An ergonomic programming language/data interchange format made with human readability in mind.

The core idea is that it separates mathematical notation from grammatical to make it more familiar to all people that can read English.

>Easy to start, hard to master.

Mathematical notation is separated from code grammar using single quotes `'`, for example:

```rgo
grammatical code (mathematical code) grammatical code 
```
This unlocks powerful grammar and allows the language to exist without keywords.

## Hello World

```rust
{print} @ "github.com/thlib/rgo/os"
{f fmt} @ "github.com/thlib/rgo/strings"
x: "World"
print f"Hello {x}"
```

For more examples see: [EXAMPLES.md](EXAMPLES.md)

## Syntax

* `x` a word
* `@` import
* `{}` dictionary
* `[]` list
* `()` math
* `<` move

For a more detailed explanation of the syntax, see: [SYNTAX.md](SYNTAX.md)

## License

[Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)


— Timo Huovinen, Dec 2022

