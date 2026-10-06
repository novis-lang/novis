`Core\Serialize::decode()` builds a value again from the `bytes` that
`Core\Serialize::encode()` returned.

An object in the result is a new object of the class with the same name in your program. Its
constructor does not run. The bytes must come from `encode()`. If they are cut short, have extra bytes
at the end, or name a class your program does not have, `decode()` throws a `ParseError`.

Bytes that came from outside the program, such as a request body or a cookie, are `tainted`. A call
to `decode()` with `tainted` bytes does not compile. This stops a visitor from sending bytes that
build any object they choose.

**The examples below** show an object read back without its constructor, bytes that are cut short,
and a list of jobs saved to a file and loaded again.
