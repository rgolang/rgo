# Macros

Macros are code "drop-in" replacements using templates that are run during compilation (build-time)

For example:
```js
{print}: @ "rgo.io/os"
('true' = '1')
('false' = '0')
('bool' = 'true|false') 
('not true' = 'false')
('not false' = 'true')
print not true // prints 0
```

## Placeholders

Placeholders are supported:

```js
{print}: @ "rgo.io/os"
('double $0' = '$0 + $0')
print double 5 // prints 10
```

* `$0` equals the following negex: `$[a-zA-Z0-9_]+`
* It does not check if the definition for `run` exists, that happens in the next compilation stage.

See [examples_negex.md](examples_negex.md) for examples.

TODO:
* These can be read in reverse to try and de-duplicate code
* Checkout loops in rust
* Make the negex example above proper
