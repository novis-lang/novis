A property, a method or a constant written without `public`, `protected` or `private` does not
compile (`E0122`). A method named `__construct` does not compile either (`E0114`). The error tells you
to name it `constructor`. A class name is PascalCase, so `class point` is the error `E0110`.

Reading a property or calling a method where its visibility does not allow it is the error `E0471`.
That includes a `private` property read in a class that extends the declaring class. A `private`
constructor works the same way. `new A()` outside `A` is `E0471`, and a `static` method of `A` may
still call `new A()`.

A class is declared at the top level of a file. A class or an interface inside a class body is the
error `E0233`. Only an `enum` or a `type` alias may be declared there. A class inside a method or an
`if` block is `E0233` too.

A wrong number of arguments is the error `E0402`. An unknown method or property is `E0405`, for a
read and for a write. A class with no `constructor` takes no arguments, so `new Counter(1)` is
`E0402`. A class that extends another and declares no constructor uses its parent's: with
`class B extends A {}`, `new B(4)` calls `A`'s constructor.

A visibility keyword is allowed only on a parameter of `constructor`. On any other method it is the
error `E0722`. A promoted parameter may also be `readonly`: `public readonly int $x`.

`new A` without parentheses is the same as `new A()`. A constructor may declare the return type
`void`, and it may have no return type at all.
