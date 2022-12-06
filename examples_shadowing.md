# Variable shadowing

Useful to re-use allocated space and make the program very memory efficient, but has the disadvantage of being confusing to read.
```rust
a: {
    b: 1,
    c: b,
    b: 3,
    d: c
}
print a // {b: 3, c: 1, d: 1}
```

Use `#noshadow` to disable variable shadowing.

TODO: 
* Link to this file
* Implement hashtags

