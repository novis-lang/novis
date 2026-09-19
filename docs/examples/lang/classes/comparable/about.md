Compares two objects of your own class with `<`, `>`, `<=`, `>=` and `<=>`.

Those operators work on objects when the class implements `Comparable`. That interface has one
method, `compareTo`, which takes another value of the same class and returns a number: negative when
this value comes first, zero when neither comes first, positive when the other one does. The four
ordering operators read only the sign of that number, and `<=>` returns the number itself. You write
the order once, in one method, and all five operators follow it.

**Good to know:** a class that does not implement `Comparable` cannot be ordered. The comparison
does not compile, and the message names `Comparable` as the fix. `==` is a separate question and is
not affected: two objects are equal only when they are the same object. To test whether two values
have the same contents, check that `compareTo` returns zero.
