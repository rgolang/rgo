# rgo
`main.rgo`

An ergonomic programming language/data interchange format made with human readability in mind.

The core idea is that it separates mathematical notation from grammatical to make it more familiar to all people that can read English.

>Easy to start, hard to master.

Mathematical notation is separated from code grammar using single quotes `'`, for example:

```rgo
gramatical code 'mathematical code' gramatical code 
```

## Hello World

```rust
{print} @rgo.os
print "hello world"
```

## Hello Alice

```rust
{print, scan} @rgo.os
{fmt} @rgo.strings
name scan
print fmt"hello {name}"
```

one line formatting is supported:
```rust
{print, scan} @rgo.os; {fmt} @rgo.strings; name scan; print fmt"hello {name}"
```
