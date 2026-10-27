`spawn script "file.nvs"` runs another file while your program keeps going. It gives you a handle.
`await $handle` waits for that file and returns its result.

The result has four fields. `ok` is true when the file finished without an error. `value` is what the
file returned, and you convert it with `as`. `output` is what the file printed, when you wrote
`output: "capture"`. `error` has the class name and the message when the file stopped with an error.

A path in quotes starts at the folder of the file that contains it. A path you build at run time
must be a full path, or `spawn script` throws `RuntimeError`. Build one with `Core\Path::join`. You
can also start a static method, written `Class::method`.

A static method and a file inside your application folder need no capability. A file outside that
folder needs the `script.spawn` capability. You can await one handle once. A second `await` of the
same handle throws `LogicError`.
