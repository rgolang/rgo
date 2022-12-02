# rgo
(pre-alpha version)  
`main.rgo`

An ergonomic programming language/data interchange format made with human readability in mind.

The core idea is that it separates mathematical notation from grammatical to make it more familiar to all people that can read English.

>Easy to start, hard to master.

Mathematical notation is separated from code grammar using single quotes `'`, for example:

```rgo
gramatical code 'mathematical code' gramatical code 
```
This unlocks powerful grammar and allows the language to exist without keywords.

## Hello World

```rust
{print} @ "github.com/thlib/rgo/os"
print "hello world"
```

## Hello Alice

```rust
rgo @ "github.com/thlib/rgo"
{print, scan} rgo.os
{fmt} rgo.strings
name scan
print fmt"hello {name}"
```

in one line:
```rust
rgo @ "github.com/thlib/rgo"; {print, scan} rgo.os; {fmt} rgo.strings; name scan; print fmt"hello {name}"
```

## Math

## Syntax
* `<` write one item from a list or channel into an undefined word or channel
* `|` stream all items one by one from a list or channel into an undefined word or channel

## Instances
