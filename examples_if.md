# If
Handling uncertainty

* In English a `?` mark is used to mark uncertainty.
* Programs are imperative.
* Imperative sentences can't end with a `?`.

## Example

Pass the uncertainty along and hope it gets handled down the line:
```js
{print, input}: @"github.com/rgolib/rgo/os"
(
    even = $0 % 2 == 0
    odd = $0 % 2 == 1
    x = double odd/even input
    print x // error: unhandled "odd/even"
)
```

Handle the uncertainty:
```js
(
    input = 3
    x = double odd/even input (
        odd? $0 + 1
        even? $0
    )
    x // 8
)
// or when there are only 2 options
(
    input = 3
    x = double odd/even input (odd? $0 + 1)
    x // 8
)
```

Handle the error with a default:
```js
(
    input = error!
    x = double input (error? 2)
    x // 4
)
(
    input = 3
    x = double input (error? 2)
    x // 6
)
```

Re-throw the error:
```js
(
    input = error!
    x = double input (error? "input error: {$0}"!)
    x // 4
)
```

## Alternative example

```js
{print, input}: @"github.com/rgolib/rgo/os"
{f: format}:    @"github.com/rgolib/rgo/strings"
(
    even = $0 % 2 == 0
    odd = $0 % 2 == 1
    input = input (error? f"input error: {$0}"!) ( // TODO: use {input} instead of {$0}?
        odd? $0 + 1
        even? $0
    )
    x = odd/even input
    print double x // 4 for input 2, 8 for input 3 
)
```


