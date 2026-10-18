An object of a class that does not implement `Stringable` is the error `E0412` in each of the four
places. A method named `toString` is not enough. The class must also say `implements Stringable`.
A class that implements `Stringable` and has no `toString` method is the error `E0449`.

The method name `__toString` is the error `E0111`, because a method name is written in camelCase.

An object passed to a `string` parameter is the error `E0401`. An object compared with a string
using `==` is the error `E0466`.

`Stringable` is also a type. A parameter or a variable of type `Stringable` accepts an object of any
class that implements it. You can use that value in the same four places.

A variable of a type such as `?Money` can be `null`. A `null` value gives the empty string in each of
the four places.

If `toString` throws an error, the `echo`, the `.` or the conversion throws that error.
