# Syntax

TODO: fill in the missing symbols below, provide `negex` (nested regex), explain why this symbol is used and the pros/cons.

The symbols below are described using [`regex`](https://en.wikipedia.org/wiki/Regular_expression)

To simplify describing the below, these characters will have special meaning:
* `b` a word boundary, `\b` in regex, for example: "hello world" is actually `bhellob bworldb` 
* `s` a spacing character, `\s` in regex, for example: "hello world" is actually `hellosworld`
* `t` a terminating character, for example: `.` period, `\n` newline, `\r` carriage return and sometimes `;` semicolon (depending on context, see [parsing.md][parsing.md]) or the end of the file.

## In grammatical context `{}`

### `a` keyword `a` 
### `1` number `1`  
`b` currying  
* `\b` is a word boundary, it can be created using a space ` `; `(`; `)`; `{` or `}`.
`<` read one to  
* TODO: Move memory operator, assembly `MOV` ?
* TODO: verify if the assembly example matches
`>` quote if the first character of an expression
* TODO: maybe define it as a tree accessor for channels? Breadcrumbs?
`:` define a keyword
* `:` starts a defining sentence for the preceding term and is terminated by `.` or `}` or `)`
`;` make a list, including the preceding clause
`,` make a list, excluding the preceding clause
`!` error  
`?` if  
`.s`/`.t` end/return
* a `.` period followed by any whitespace or newline character is treated as a `return`
* ends an imperative sentence started by `:`
* ends a list
* `return` for code in `{}`
`.` accessor/concatenation
* TODO:
`#` hashtag  
`@` import  
* import all the keywords from a package on the network, for example a domain
`$` unknown input placeholder  
`%` probability  
`/` or  
`\` escape the next character
`|`
* Undefined TODO: `pipe` or `or`?
* TODO: maybe: stream all items one by one from a list or channel into an undefined word or channel
`_` visual separator  
* Does nothing
* TODO: maybe use to define undefined variables? (Maybe ^ would be better as it's used for proof-reading)
`-` range  
`+`   
`*` 
* TODO: maybe: pointer or list item?
`=` macro equals  
`''` macro placeholder  
`()` math
* for example: `(2*3)`
`{}` dictionary  
* dictionary of keywords, for example: `{word_a: "hello", word_b: "world"}`
* is a stored procedure in the program grammar code context
* is a scoped stream of operations for the CPU to process
`[]` list  
* for example: `[0, 1, 2, 3]`
`...` spread/concatenate list
* for example: `a: 0, ...[1, 2], 3` becomes `{a: [0, 1, 2, 3]}` 
* for example: `[...[0, 1, 2], ...[3, 4, 5]]` becomes `[0, 1, 2, 3, 4, 5]` 
'`'
* tilde

## In mathematical context `()`
`` math  
`a` keyword `a`  
`1` number `1`  
` ` currying  
`<` less than  
`>` greater than  
`<=` less than or equals to  
`>=` greater than or equals to  
`==` equals  
`~` approximate  
`:`   
`;` nest a list  
`,` make a list  
`!`   
`?`   
`.`   
`=` set the value of  
`#`   
`@`   
`$`   
`%`   
`/`   
`\`   
`|`   
`_` visual separator  
`-` minus  
`+` plus  
`{}` dictionary  
`[]` list  
`()` group  


TODO: 
* Make the syntax in README.md match this one
* Make the syntax in README.md shorter
* Make the syntax in README.md and here alphabetically ordered.
