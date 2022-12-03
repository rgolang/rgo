# Structured data

{x:1} // core type dictionary, aka define operation  
x: 1 // shorthand for define when inside a dictionary  
x 1 // curry operator, when used with an unknown function x performs a define operation: {x:1}  
  
{a:1,b:2,c:3} // a dictionary with 3 words: a, b, c  
a dictionary can be moved to memory  
But it has to be contextual to avoid erasing all other values  
id: rgo.Heap {a:1,b:2,c:3}  
res: rgo.Heap < id {a:1,b:3} // id nests it as {address1:{a:1,b:3}}  

