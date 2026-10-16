Returns a function for every class that implements an interface, and each function creates an object of its class.

`Core\Program::implementing` creates the objects itself, so each class needs a constructor without
arguments. This member gives you the functions instead, and you pass the arguments when you call
one. You write two types between `<` and `>`: the interface or class, and the type of the function,
for example `Core\Program::constructors<Handler, callable(Settings): Handler>()`. Each row has two
fields: `class` is the full class name, and `make` is the function. The rows are sorted by class
name, in the same order as `implementing`.

**Good to know:** no object is created until you call `make`. A class whose constructor does not
accept these arguments does not compile.

**The examples below** create one handler by its class name, create only the plugins that are
switched on, and print a list of commands without creating any of them.
