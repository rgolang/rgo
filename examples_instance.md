# Instance

```rust

Server: {
    h: Heap 
    h < $0
}
Server.add: {
    Server.h < int ($0+)  
}
Server.get: {
    < h
}

instance: Server {
    init: 10
}
instance.Add
```

---

```rust
h {
    x: Heap $0
    set: {
        x < $0
    }
}
h.add: {
    h.x < (+$0)
}
h.get: {
    < h.x
}
x: h 1
x.add < 2
y: x.get
y // 3
x.set < 5
z: x.get
z // 5
```

