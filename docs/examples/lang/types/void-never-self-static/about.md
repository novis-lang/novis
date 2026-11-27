Four return types say something an ordinary type cannot.

`void` means the method returns no value. You call it as a statement of its own, and you cannot use
the result.

`never` means the method does not return to its caller. It always throws an error or ends the
program.

`self` and `static` both name a class. `self` is the class the method is written in. `static` is the
class the call names. So a method in a base class can return an instance of the child class it was
called on, and the caller gets that child class. `$o is static` checks whether `$o` is an object of
that class.

The examples show a method called only for what it does, a chain of calls on one object, and a base
class that creates an instance of the child class the call names. Example 04 shows that a `void`
result cannot be printed. Example 05 shows that a `never` call ends the caller's path. Example 06
shows a parameter of type `callable(static)`. Example 07 shows a finder that returns only objects of
the class the call names.
