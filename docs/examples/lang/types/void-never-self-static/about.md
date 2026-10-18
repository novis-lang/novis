Four return types say something an ordinary type cannot.

`void` means the method returns no value. You call it as a statement of its own. You cannot print
the result, store it in a variable or pass it to another method.

`never` means the method does not return to its caller. It always throws an error or ends the
program.

`self` and `static` both name a class. `self` is the class the method is written in. `static` is the
class the call names. So a method in a base class can return an instance of the child class it was
called on, and the caller gets that child class.

The examples show a method called only for what it does, a chain of calls on one object, and a base
class that creates an instance of whichever child class the call names. Example 04 shows that a
`void` result cannot be printed. Example 05 shows that a call to a `never` method ends the caller's
path.
