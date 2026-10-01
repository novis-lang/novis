`spawn script "file.nvs"` runs another file while your program keeps going. It gives you a handle.
`await $handle` waits for that file and returns its result.

The result has four fields. `ok` is true when the file finished without an error. `value` is what the
file returned, which you convert with `as`. `output` is what the file printed, when you asked to keep it
with `output: "capture"`. `error` has the class name and the message when the file stopped with an
error.

A path written as a string literal starts at the folder of the file that contains it. A path you
build while the program runs must be a full path, or `spawn script` throws `RuntimeError`. Build one
with `Core\Path::join`. You can also start a static method, written `Class::method(...)`. You need the `script.spawn` capability to start either
one. One handle can be awaited once. A second `await` of the same handle throws `LogicError`.

**The examples below** show one file started and collected, two files running at the same time, and a
page written while a report is built next to it.
