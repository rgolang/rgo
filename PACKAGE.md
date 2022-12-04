# Packages

When running a file it defaults to the `main` target: `/bin/sh rgo run ./my_app.rgo`

```js
// in ./my_app.rgo
main: {
    // code in grammatical context to run
}
```
equally
```js
// in ./my_app.rgo
main: (
    // code in mathematical context to run
)
```

When the `run` target is a directory/folder: `/bin/sh rgo run ./my_app` it will default to `/bin/sh rgo run ./my_app/main.rgo` if the file exists, or run the below code:

```js
// in ./my_app
{os, parse} @ "rgo.io/lang/rgo"
main: {
    cwd: os.cwd // sets cwd to the path handler of "./my_app"
    ...{ // TODO: Finish loops
        ...$0
    } parse .read | cwd.files "./" // TODO: This syntax is not yet finalized
}
```

This can be customized with a `make.rgo` with `run:` definition TODO: Example link

Which reads and parses all the files in the current folder into a common wrapping `{}` scoped code.

Example:

```js
// ./my_app/file1.rgo
a: 0; b: 1
// ./my_app/file2.rgo
b: 2, c: 3
// ./my_app/file3.rgo
d: 4
e: 5
// ./my_app/file4.rgo
f: 6
g: 7
```

```sh
/bin/sh rgo run ./my_app
```
runs:
```js
[{a:0},{b:1},{c:3},{d:4},{e:5},{f:6},{g:7}]
```

* TODO: Make `main` configurable `rgo run ./ -d default`
* TODO: Mention `make.rgo`

