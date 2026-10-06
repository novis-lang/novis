An object becomes text through the interface `Stringable`, whose one method is `toString`.

A class that implements it can be written out in four places: `echo`, the `.` operator that joins two
texts, a value inside a string like `"{$price}"`, and the cast `as string`. Novis calls `toString` at
each of them and uses the text it returns. An object whose class does not implement the interface is
not allowed in any of the four, so no program prints something like `Object#3`.

Everywhere else an object stays an object. Passing one to a method that wants a `string` does not
compile. Write `as string` there yourself.

**Good to know:** the method is called on the real class of the object, so a subclass that writes its
own `toString` is the one that runs.

**The examples below** show a price with its own text form, the four places that text appears, and a
receipt built out of several such classes.
