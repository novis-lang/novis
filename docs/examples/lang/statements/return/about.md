`return $value;` leaves the method it is in and gives the value to whoever called the method. The
lines after it do not run, and neither does the rest of any loop it is inside. A method may have as
many `return` statements as you need, and the first one that runs ends the method.

The value has to fit the return type the method declares. A method declared `void` gives nothing
back, so its `return` carries no value and is written `return;` alone. Writing a value there does
not compile.

`return` also works outside any method, at the top level of a file. There it ends the file. In the
file you started, that ends the program.

**The examples below** show a method that returns as soon as it has found what it was looking for,
then a `void` method that ends early, then a program that stops because it has nothing to do. The
last one shows a `void` method with a value in its `return`, which does not compile.
