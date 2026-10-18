`object` is a type that contains an object of any class. It names no class itself. The compiler only
checks that the value is an object. Use it where a value may come from several unrelated classes,
such as a plugin a program loaded.

A property read through an `object` compiles. The name is looked up while the program runs. The read
throws an error when the object has no property with that name.

A method call through an `object` does not compile. Name the class first. Inside `if ($value is
Tag)`, you can call the methods of `Tag`. `$value as Tag` gives you a `Tag` to use anywhere.

**The examples below** show a value of unknown class, a check before calling a method on it, a list
of mixed objects sorted by class, and a method call that does not compile.
