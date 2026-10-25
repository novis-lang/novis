The message must be a `string`. `new RuntimeError(5)` is the error `E0401`. A `tainted string` is the
error `E0401` too, so check text from outside the program before you put it in a message.

`previous` takes a `Throwable` or `null`. Another type is the error `E0401`. Any other option, such as
`{ cause: $e }`, is the error `E0454`, and its help lists `previous`.

With names, the arguments may come in any order:
`new RuntimeError(options: { previous: $e }, message: "…")`. A subclass that writes no constructor
accepts the names too.

A subclass with its own constructor must call `parent::constructor(...)` on every path. A path without
the call is the error `E0410`. `parent::constructor()` with no message is the error `E0402`.

A subclass may add properties with a default value, such as `public int $status = 500;`, without a
constructor of its own.
