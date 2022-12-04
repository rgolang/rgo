# Completeness

Defining a keyword does not have to provide the full definition, partial definitions are also accepted

```rust
email: {
    address: $0
    subject: $1
    body: $2
}
email_bob: message "bob@example.com"
email_bob "hello {email_bob.address}" "how are you?" // {address: "bob@example.com", "subject":"hello bob@example.com", "body":"how are you?"}
```

* `$0` is a placeholder for the next value that is passed

The placeholder can be anywhere:

```rust
dict: {
    $0: $1
}
x: dict a "bob"
x.a // bob
```

* Notice how the `a` in `dict a "bob"` is undefined and it's ok because `dict` defines it.

TODO:
* Figure out the correct term for this.
* Link from somewhere to this file.

