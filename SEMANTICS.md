# Rgo v1 Semantics

This document describes what Rgo programs mean: which programs are valid,
what source constructs do, and what behavior users can rely on.

## Core Model

Rgo is an identifier-driven, definition-oriented, declaration-before-use,
single-assignment, expression-less, continuation-passing language.

Programs are built from two fundamental actions:

- defining identifiers for values
- transferring control to identifiers that interpret those definitions

There are no expression returns. Computation is a sequence of definitions
followed by a control transfer to another executable value.

At the file root, Rgo accepts definitions. Root-level execution is not valid
source; the compiler appends an invocation of the requested target function
after parsing.

Source-file flow:

- read definitions in order
  - every referenced name and type must already be declared
- choose a compile target by name
  - append an invocation of that target
  - require the target invocation to be complete
- execute by tail-transferring between executable values

Blocks are non-empty. `;` and newlines separate block items.

## Lexical Rules

Identifiers start with an ASCII letter or `_`, followed by ASCII letters,
digits, or `_`.

Line comments start with `//` and continue to the end of the line.

String literals have two forms:

- double-quoted strings process escapes such as `\n`, `\t`, `\\`, `\"`,
  `\0`, and `\u{...}`
- single-quoted strings keep backslashes as ordinary characters

Integer literals are signed machine-sized integer values. Floating literals
exist and have type `f64`.

## Definitions

A definition introduces a name for a value inside the current scope:

```rgo
str: @str
printf: (fmt: str!, args: ..., ok:()) {
    (s: str) = @sprintf(fmt, args)
    @write(s, ok)
}

name: "Bob"
foo: (ok:()){
    printf("hello %s", name, ok)
}
```

Definition forms:

- `name: literal`
  - defines a literal value; string and integer literal aliases have
    compile-time types `str!` and `int!`
- `name: other`
  - aliases an existing identifier
- `name: other(args...)`
  - defines a curried executable value
- `name: (params...)`
  - defines a signature alias
- `name: <T>(params...)`
  - defines a generic signature alias
- `name: (params...){ body }`
  - defines a function
- `name: <T>(params...){ body }`
  - defines a generic function

Function and lambda parameters must have explicit types. Signature aliases may
use unnamed type slots such as `(str)` because they describe shape rather than
binding local parameter names.

Generic parameter lists must contain at least one name and cannot repeat a
name.

Redefining the same label in the same scope is invalid:

```rgo
x: 1
x: 2
```

Nested scopes may shadow outer labels:

```rgo
x: 1
(){
    x: 2
}()
```

Type aliases obey the same declaration-before-use rule as values. Builtin
types and functions are available as `@name` references and can be aliased or
used inside ordinary source definitions, such as `int: @int` or a `@write`.

## Invocation and Currying

Application syntax does not imply a C-style call. The same surface form is
interpreted by position.

Application flow:

- parse an application form, such as `foo(x)`
  - if it appears as a definition value or as an argument
    - bind the supplied arguments
    - produce an executable value
    - do not transfer control yet
  - if it appears as a standalone block item
    - bind the supplied arguments
    - transfer control to the executable value
    - do not return to the current block item

When application appears on the right-hand side of a definition or as an
argument, it applies arguments to an executable value without running it. The
result is another executable value that can be run later:

```rgo
end: exit(0)
foo(end)
```

In `end: exit(0)`, `exit(0)` is a curried executable value. It does not execute
at definition time.

When application appears as a standalone block item, it transfers control to
that executable value and does not return to the current location:

```rgo
foo(end)
```

Block item identifiers and lambdas are invocations even when they have no
explicit argument list:

```rgo
write: @write

mywrite: write("hello", (){})
mywrite
```

The final `mywrite` invokes the executable value and writes `hello`.

Likewise, a lambda block item executes:

```rgo
write: @write

(){
    write("hello", (){})
}
```

A standalone invocation must supply enough arguments to run. A partial
application can be stored or passed, but it cannot be the final action of a
block item by itself.

Literal values cannot be standalone invocations.

Chained application supplies more arguments to the same executable value:

```rgo
foo(a)(b)
```

is equivalent to currying `foo` with `a`, then currying that result with `b`.

## Argument Matching

Arguments are matched by position unless named arguments are used.

Argument matching flow:

- inspect the invocation target signature
  - if all arguments are positional
    - bind them from left to right
  - if any argument is named
    - bind positional arguments to the first still-unassigned parameters
    - bind named arguments to parameters with the same names
    - reject duplicate or unknown argument names
  - reject too many arguments unless a variadic parameter accepts them
  - type-check every non-variadic argument against its matched parameter

Parameter names matter for named application. Function signature compatibility
is otherwise structural: two function-typed values match by parameter shape and
types, not by parameter names.

Builtins can be passed where a function-typed value is expected. The compiler
treats the builtin as a function value that forwards its parameters to the
builtin operation.

## Scope Capture

Scope capture rewrites the rest of the block into a continuation and passes it
to the captured operation.

Scope-capture flow:

- encounter `(name: type) = operation(args...)`
  - require `operation` to be an executable value that accepts the appended continuation
  - turn the rest of the current block into a continuation
    - the continuation receives `name: type`
    - the continuation body is the original remaining block
  - append that continuation to `operation(args...)`
  - transfer control to `operation`

For example:

```rgo
int: @int
add: @add
str: @str
printf: (fmt: str!, args: ..., ok:()) {
    (s: str) = @sprintf(fmt, args)
    @write(s, ok)
}
exit: @exit

hello: (){
    (sum: int) = add(2, 3)
    printf("sum: %d\n", sum, exit(0))
}
```

This behaves like nested continuations, but keeps source code flat. Each
capture introduces the named value into the remaining block.

Nested functions may reference values from enclosing scopes. Those references
are captured into the nested function value. Root-level functions are ordinary
declared functions and are not captured from other root-level functions.

## Types

Builtin type names can be used directly with `@`:

```rgo
foo: (x: @int, text: @str, ratio: @f64){
}
```

They can also be given local aliases with ordinary definitions:

```rgo
int: @int
str: @str
f64: @f64
```

The primitive type rules are:

- `str` must be a string.
- `str!` must be a compile-time available string. The argument must be a string
  literal or another `str!` value.
- `int` must be an integer.
- `int!` must be a compile-time available integer. The argument must be an
  integer literal or another `int!` value.
- `f64` must be a floating-point value. An integer literal may satisfy an
  `f64` parameter because it is compile-time available.

Function types are written with parameter lists:

```rgo
receiver: (value: str)
predicate: (ok: (), err: ())
mapper: (value: str, ok: (str))
```

Data and control are encoded with functions and signatures rather than
reserved categories. A value that can be either an integer or a string can be
represented by a function that accepts one continuation for each case:

```rgo
int: @int
str: @str

int_or_str: (i: (int), s: (str))

as_int: (x: int, ok: (int), (str)){
    ok(x)
}

as_str: (x: str, (int), ok: (str)){
    ok(x)
}
```

A control form can be represented the same way:

```rgo
bool: (yes: (), no: ())

true: (yes: (), no: ()){
    yes()
}

false: (yes: (), no: ()){
    no()
}

choose: (cond: bool, yes: (), no: ()){
    cond(yes, no)
}
```

Signature aliases name reusable function types:

```rgo
receiver: (str)
pair: <T>(left: T, right: T)
```

Generic type parameters are placeholders inside a signature or generic
function. Repeated uses of the same generic parameter must resolve to the same
actual type for a given invocation.

Generic matching flow:

- enter a generic function or signature alias
  - register each generic parameter name once
- match actual arguments against expected parameters
  - when a generic parameter is first seen
    - bind it to the actual type
  - when the same generic parameter is seen again
    - require the actual type to match the earlier binding
- substitute the bound types through the remaining signature

The `...` marker is part of a parameter, not part of the type itself. It marks
how the function accepts input, in the same way that `!` marks a compile-time
requirement on the parameter. A user-declared `...` parameter is opaque: source
can forward it to another variadic call, but cannot inspect it as a collection.

## Variadic Parameters

Source programs may declare their own `...` parameters. The parameter has no
source-visible element type or collection API.

The implemented variadic builtin is:

```rgo
sprintf(format: str!, args: ..., ok: (str))
```

Variadic argument flow:

- match fixed prefix parameters
  - `format` must be `str!`
- match fixed suffix parameters
  - `ok` is the final continuation
- route every argument between the prefix and suffix through `args`
  - v1 does not expose `args` as a source-level array value
  - v1 does not assign a declared element type to those middle arguments

Formatted printing is ordinary source code:

```rgo
printf: (fmt: str!, args: ..., ok:()) {
    (s: str) = @sprintf(fmt, args)
    @write(s, ok)
}
```

## Closure Values and Affinity

A closure value is an executable value with captured and/or already supplied
arguments.

Currying a closure produces an executable value with more arguments supplied.
The language must preserve affine behavior: no two live values may observe
incompatible mutations of the same logical closure state.

Rules:

1. If a closure value has exactly one remaining use, currying may reuse the
   same logical closure value.
2. If a closure value has more than one remaining use, currying must preserve
   the other uses as if they had independent closure state.
3. If the same closure value is used in multiple places, such as `k(x, x)`,
   all uses must behave as independent values where later currying could
   otherwise interfere.
4. Pure renaming without duplication does not create a new semantic use.

These are semantic rules.

## Builtins and Imports

Bare `@name` references compiler-provided builtin types and functions. Builtins
can be used directly at the use site:

```rgo
hello: (){
    (s: @str) = @sprintf("hello\n")
    @write(s, @exit(0))
}
```

They can also be aliased with ordinary definitions:

```rgo
int: @int
write: @write
```

The builtin name must match a builtin known by the compiler. Current builtin
entries are:

```rgo
@str // owner: backend/runtime ABI; string literal storage and pointer/length passing
@int // owner: backend/ABI; machine preferred integer layout for the target architecture
@f64 // owner: CPU/backend/ABI; floating-point layout and register passing
@add // owner: CPU/backend; primitive integer instruction exposed with a CPS signature
@sub // owner: CPU/backend; primitive integer instruction exposed with a CPS signature
@mul // owner: CPU/backend; primitive integer instruction exposed with a CPS signature
@div // owner: CPU/backend; primitive checked integer division with error and success continuations
@divint // owner: CPU/backend; primitive checked integer division with error and success continuations
@addf64 // owner: CPU/backend; primitive floating-point instruction exposed with a CPS signature
@mulf64 // owner: CPU/backend; primitive floating-point instruction exposed with a CPS signature
@divf64 // owner: CPU/backend; primitive floating-point instruction exposed with a CPS signature
@eq // owner: CPU/backend; primitive equality branch emitted as direct control transfer
@eqi // owner: CPU/backend; primitive integer equality branch emitted as direct control transfer
@eqs // owner: backend/runtime; string equality over the runtime string representation
@lt // owner: CPU/backend; primitive integer comparison branch emitted as direct control transfer
@gt // owner: CPU/backend; primitive integer comparison branch emitted as direct control transfer
@write // owner: OS/filesystem descriptor API; byte-stream output operation
@exit // owner: OS process ABI; process-completion operation
@sprintf // owner: libc variadic ABI and runtime buffer; current v1 builtin exception for formatting to a string
```

The root namespace is flat:

- no duplicates
- no categories
- no groups
- no second paths
- no keywords

Builtin references have no path form:

```rgo
int: @int
foo: (x: int){
}
```

Only `@name` names builtins. In v2, source imports use `@/path` and remain
separate from builtin references.

Builtin operation signatures:

- integer arithmetic: `add`, `sub`, `mul` take `x: int`, `y: int`, and
  `ok: (int)`
- integer division: `div` and `divint` take `x: int`, `y: int`, `err: (int)`,
  and `ok: (int)`
- floating arithmetic: `addf64`, `mulf64`, `divf64` take `x: f64`, `y: f64`,
  and `ok: (f64)`
- equality: `eq`, `eqi`, and `eqs` choose true and false continuations rather
  than returning booleans
- integer comparisons: `lt` and `gt` jump to their final continuation when the
  comparison succeeds
- conversion and output: `write`, `sprintf`, and `exit`
  perform their named effects through continuations where their signatures
  require one

`sprintf` is a current exception to the builtin design rules. It is a
higher-level formatting facility and should become ordinary Rgo library code or
platform-backed library code once the language can express its implementation.

`puts` is not a builtin. It is ordinary source code built from `@write`, and a
libc-style wrapper always appends a newline before continuing:

```rgo
str: @str
write: @write
puts: (s: str, ok:()) {
    write(s, () {
        write("\n", ok)
    })
}
```

Higher-level facilities are ordinary Rgo code or platform-backed library
code rather than core builtins. For example, integer maximum is a library
function built from comparison and continuations:

```rgo
int: @int
lt: @lt

max_int: (x: int, y: int, ok: (int)){
    lt(x, y, (){
        ok(y)
    }, (){
        ok(x)
    })
}
```

Language features that do not depend on external functionality are expressed
with grammar and punctuation rather than English keywords.

## Punctuation Pattern

Rgo uses a repeated punctuation pattern:

- `()` provides or fills value slots.
- `<>` provides or fills type slots.
- `{}` separates or contains choice/body structure.
- `name:` labels code or values.
- `@name` references compiler-provided builtin types and functions.
- `@/path` names source imports in v2.

Types are not allowed as arguments in `()` because type arguments can often be
inferred and would clash with value argument counts. Type arguments use `<>`
instead.
