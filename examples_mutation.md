# Mutating data

Mutating the stack is possible but done through the stack keyword, it's dangerous?
See how rust solves this Todo:

```rust
c rgo.Stack 0
c < (+1)
c < (+1)
c // 2
```

```rust
c rgo.Heap 0
c < (+1)
c < (+1)
c // 2
```


