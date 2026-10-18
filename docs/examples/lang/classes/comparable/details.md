Each of the five operators on objects of a class that does not implement `Comparable` is the error
`E0411`. A class that implements `Comparable` and has no `compareTo` method is the error `E0449`.

Both sides must have the same class. Two objects of different classes are the error `E0411`, even
when both classes implement `Comparable`. This includes a subclass and its parent class. An object
compared with a number is `E0411` too. So is a value of a type such as `?Version`, which can be
`null`.

Each operator calls `compareTo` once. If `compareTo` throws an error, the comparison throws that
error.

`==` does not call `compareTo`. `$a == $b` is `true` only when both variables are the same object.
