# Semicolon explained  
  
: Is for dictionaries/objects  
, Is for lists/arrays  
; Is to nest a list  
x:1 // x [1]  
x;1 // [x,1]  
x,1 // [x,1]  
  
x:1:2:3 // {x:{1:{2:3}}}  
x;1;2;3 // [x,1,2,3]  
x,1,2,3 // [x,1,2,3]  
  
x:1,2,3 // {x:[1,2,3]}  
x;1,2,3 // [x,[1,2,3]]  
x;1;2,3 // [x,1,[2,3]]  
x,1;2,3 // [x,[1,2],3]  
x;1,2;3 // [[x,1],[2,3]]  
  
x:1;2,3 // {x:[[1,2],3]}  
  
  