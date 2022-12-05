# Parsing

An imperative computer program is a series/array (ordered list) of operations, it is often denoted as a list of items: `[item0, item1, item2]`.   

In English literature this would be an imperative sentence and would look like:
```
Take this little bag of dreams; Unloose the cord.
<-----------------------------> <-------------->
           clause                    clause         
<---------------------------------------------->
                   sentence
```

* `;` creates a list and marks the preceding clause as part of the same list.
* `.` marks the end of the imperative sentence and is often implied by ending the list.
* Information flows from right to left while it is read left to right
    ```
    Take this little bag of dreams; Unloose the cord.
    reading direction ------------------------------>
    <----------------------------------- information.
    ```

This is parsed to:
```
["Take this little bag of dreams", "Unloose the cord"]
 <------------------------------>  <---------------->
              item                        item
<---------------------------------------------------->
                       array
```

This could also be written as:

```
Advice: Take this little bag of dreams; Unloose the cord.
Advice: Take this little bag of dreams, Unloose the cord.
<---->  <---------------------------->  <-------------->
word                clause                   clause
        <---------------------------------------------->
                     definition of "Advice"
```

* `,` and `;` can be used as the list separator, whichever comes first will be the primary list and whatever comes second will be a sub-list.


Which would be parsed into a dictionary of definitions (Unless it's already wrapped in a dictionary `{}`):
```
{"Advice": ["Take this little bag of dreams", "Unloose the cord"]}
 <------>   <------------------------------>  <---------------->
   term                   item                       item
           <---------------------------------------------------->
                          definition of "Advice"
```

An example of how it would look like:

```
term: operation 1; operation 1a, operation 1b; operation 2
term: operation 1, operation 1a; operation 1b, operation 2
<-->  <--------->  <---------->  <---------->  <--------->
term  item         item          item          item
      <-------------------------------------------------->
      list
                   <------------------------>
                   sub-list
```

This is parsed into:
```
{"term": ["operation 1", ["operation 1a", "operation 1b"], "operation 2"]}
 <---->   <--------->     <---------->    <---------->     <--------->
term      item            item            item             item
         <-------------------------------------------------------------->
         list
                         <------------------------------>
                         sub-list
```

---
`:` colon creates recursive nested tree structures if it's not terminated with a `.` period

```
word 1: word 2: word 3: word 4
```

Is parsed into:

```
{"word 1": {"word 2": {"word 3": "word 4"}}}
```

The reason human languages do lists and trees this way is so that you would be able to detect the structure as you're reading without having to read until the end, it's a stream of information.  


This is confusing so here are some more examples to help clarify:

`:` start definition
`.` end definition
`,` list
`;` nest list

``` js
x: 1. // {x:1}
x; 1. // [x,1]  
x, 1. // [x,1]  
```
```
x: 1. y: 2. // {x:1,y:2}
```

```js
x: 1: 2: 3. // {x:{1:{2:3}}}  

x; 1; 2; 3. // [x,1,2,3]  
x, 1, 2, 3. // [x,1,2,3]  
  
x: 1, 2, 3. // {x:[1,2,3]}  
x; 1, 2, 3. // [x,[1,2,3]]  

x; 1; 2, 3. // [x,1,[2,3]]
x: 1; 2: 3. // {x:[1,{2:3}]}  
x, 1; 2, 3. // [x,[1,2],3]  
x; 1, 2; 3. // [[x,1],[2,3]]  
  
x: 1; 2, 3.  // {x:[[1,2],3]}  
```

TODO: Generate examples of all possible permutations with `:`, `;`, `,` and `.`

---

a new line is also a separator, for example:

```js
{a: 1; b: 2}
// is
{a: 1, b: 2}
// is
{
    a: 1
    b: 2
}
```

This means that

```js
{
    a:1; b:2
    c:3, d:4
    e:5
    f:6
}
```
is parsed as
```js
{a:1; b:2; [c:3, d:4, e:5, f:6]}
```
but since anonymous lists like that don't make sense as they are auto exported // TODO: Maybe make sub-lists a sub-thread?
```js
{a:1; b:2; c:3; d:4; e:5; f:6}
```
Which is normalized into
```js
{a:1, b:2, c:3, d:4, e:5, f:6}
```

## `{}` meaning

Is a scoped stored procedure in the program grammar code context:

```js
{a: 1, b: 2, c: {d: 1, e: 2}} // [a:1, b:2, c.d:1, c.e:2]
```

## Default `{}`:

By default all files except `main.rgo` are actually treated as already inside a shared code scope `{}`, hence typing:
```js
a: 1; b: 2 // produces {a: 1, b: 2}
```

For more examples see: [examples_package.md](examples_package.md)
