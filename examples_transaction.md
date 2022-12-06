# Transactions

```rust
pg: @"example.com/postgres"
conn: pg.connect "postgres://localhost:5432"
out: conn.tx {out1: op1 in1} {out2: op2 in2} {
    out3: log "transaction complete"? "log failed, transaction aborted"!
}
print out.out1
print out.out2
print out.out3
```

In go it could look like: TODO: examples in other languages
```go
var out3 error
conn.tx(op1(in1, out1), op2(in2, out2), fn () error {
    // if the code here fails, neither op1 or op2 will apply their changes
    err = log("transaction complete")
    if err != nil {
        return fmt.Errorf("log failed, transaction aborted: %w", err)    
    }
})
fmt.print(out1)
fmt.print(out2)
fmt.print(err)
```

TODO: Link to this file.

