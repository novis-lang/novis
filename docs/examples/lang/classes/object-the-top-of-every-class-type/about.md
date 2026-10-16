`object` is a type that holds an object of any class. It names no class itself, so the compiler
knows a value of this type is an object and nothing more. Use it where a value may come from several
unrelated classes, such as a plugin a program loaded or a value that just came out of a container.

A property read through an `object` compiles. The name is looked up while the program runs, and the
read throws an error when the real class has no property with that name.

A method call through an `object` does not compile. Say which class you mean first. `if ($value is
Tag)` opens the call inside that branch. `$value as Tag` gives you a variable of that class to use
anywhere.

**The examples below** show a value of unknown class, a check before calling a method on it, and a
list of mixed objects sorted into the classes a program handles.
