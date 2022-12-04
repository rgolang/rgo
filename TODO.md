Maybe wrap macro in <> or ''
'not true' = 'false'

Every = macro is added to compiler dictionary and then the compiler starts a new thread for this new definition that starts reading code from this point along with other routines doing the negex matching
matches for the same code are sent to a channel that stores possible negex free replacements for the same code in the right order.
Phase 2 simply does that search and replace, replacing longer strings first.

TODO:  
= Macro defines at build time  
: Curry defines at runtime  

$0.$1 = Todo:  

Type check?  Send message/email $0 = Todo:  

  
rgo is a declarative logical language // stream first, can mutate so not functional



`.`  Terminal symbols TODO:  
`:;,?!` Non-terminal symbols  TODO:  

TODO: `()` toggles math context? `{}` toggles grammar context?


---

Explain why ('bool' = 'true|false') uses `|` instead of `/`, because it's way more common in functional programming `haskell` (TODO: Verify); `curry` (TODO: Verify) and `typescript`,`php` and `go` and to use `|` for union. `/` might also be confused with `per` and `or`, also by having `|` be build time, `/` can be for runtime `or`.

TODO:
```rust
bool: true/false

// gets run as possible options:

x bool: true?
x bool: false?

// similar to rust match
x match: 
    true(x) => // run code if true
    false(x) => // run code if false
```

---


TODO: Implement negex
TODO: Support all of https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Right_shift_assignment for rgo_js



* `$0.x` can do shadowing and mean `$0.x₁`?
* check what shadowing actually means
* Assembly is a signal processor
* https://en.wikipedia.org/wiki/Parsing_expression_grammar (is this of any use?)






