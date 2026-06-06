# Rgo Language Design Rules

This document states Rgo design rules, constraints, and invariants in affirmative terms. 
It does not pin surface syntax. 
Concrete spelling, punctuation choices, examples, and placeholder forms belong in syntax proposals, tests, or reference material.

## Definitions must be ordered

Rgo must be identifier-driven, definition-oriented, and declaration-before-use.
A definition must become available only to later code in the same scope. Earlier code must not depend on later labels.

## Rgo must be CPS

Rgo must be continuation-passing.
Executable control must proceed through continuations, and every executable path must transfer control explicitly.

## Nested scopes must inherit outer labels

A nested scope must be able to use labels from enclosing scopes. 
Referenced outer values must be captured into nested executable values.

## Builtin labels must not share the user label namespace

Builtin labels must not share the user label namespace. 
A builtin can participate in the user label namespace only when user code gives it a user-space label.

## Computation must be application

User-visible computation must be expressed through application. 
Currying, invocation, argument binding, and continuation passing must be cases of the same application model, comparable in role to beta reduction in lambda calculus.

## Semantic categories must come from shape

Rgo must not need reserved category keywords like `func`, `struct`, `type`, or `interface`.
Callable code, data shapes, type aliases, and interface-like contracts must be differentiated by syntax and represented by labels, signatures, types, and application rules.

## Labels must preserve labelled code semantics

Code must be labelable. A label must be one binding mechanism, not a separate declaration category for values, types, functions, signatures, or already-applied executable values.
Labels may also be introduced by function parameters for received values.
A label must bind one meaning in its scope, and a nested label may shadow an outer label without changing the outer label.
When a label labels code, it must have the same kind of value as the code it labels, and later uses of that label must compile as uses of the labelled code.
Replacing a labelled-code use with the labelled code must preserve target selection, argument binding, captures, continuations, type checking, lowering, and executable behavior.

## Function parameters can introduce labels for received values.

Function parameters can introduce labels for received values.

## Rgo must avoid deeply nested continuation flow

Rgo must provide a mechanism for expressing continuation-heavy control flow without forcing deeply nested code structure.

## Control flow must be handled by functions

Control flow must be handled through functions instead of reserved keywords like `if`, `for`, `while`, `break`, `switch`.

## Definitions must not imply control transfer

Defining code must not execute it. Control must transfer only when an executable value is invoked.

## Types must be structural

Rgo must have a small structural type model. 
Type compatibility must not depend on a shared nominal declaration unless the type itself intentionally encodes identity.

## Signature matching must use duck typing

A value must match a signature by having the required callable shape and compatible nested types.

## Values must not become dynamically untyped

Rgo must avoid the pitfall of allowing values to flow through the program without a checked static shape.
Every value that crosses a label, parameter, continuation, or import boundary must have a compile-time type or signature, and mismatches must fail before execution.

## Builtin functions must replace symbolic operators

Builtin functions must replace symbolic operators for arithmetic, comparison, conversion, output, and process completion.

## Builtin signatures must be valid user signatures

A builtin function must expose a signature that would also be valid as a user-defined signature.
The implementation may be internal, hidden, backend-specific, or platform-specific, but the callable surface must obey ordinary Rgo signature rules.

## Core builtins must require backend support

Core builtin functions must not be arbitrary conveniences.
They must enable functionality that user code cannot implement by itself, such as OS interaction, process completion, raw assembly-level operations, ABI/platform calls, or primitive backend instructions.
They must require direct support from each supported backend or standard platform facility and must have explicit CPS signatures.

## Higher-level facilities must be library code

Higher-level common facilities must be implemented as ordinary Rgo code or platform-backed library code and published through official libraries.

## Data and control must be encoded with functions and signatures

Data structures and control forms must be encoded with functions and signatures.

## Loops must be recursive continuation-passing functions

Loops must be recursive continuation-passing functions.

## Closures must carry captured state

Closure values must capture referenced outer values and preserve independent behavior when they have multiple live uses.
