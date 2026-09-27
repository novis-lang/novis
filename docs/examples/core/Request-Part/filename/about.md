Returns the file name that the client sent with an uploaded file, such as `invoice.pdf`. This is the
name the file had on the client's computer. Replaces the `name` field of PHP's `$_FILES`.

The result is a `tainted string`, which means it came from outside your program. The client can send
any name: an empty one, a very long one, or one like `../../config.toml` that points to another
folder. You cannot use it as a path directly. `Core\IO::within` checks that the name stays inside a
folder you choose. A safer way is to save the file under a name your program makes, and to use the
client's name only for display.

**Good to know:** when a person submits a form without choosing a file, the browser still sends the
field. Its file name is then empty.

The examples show the name of each file, how to skip a file field that was left empty, and how to
save each file under a new name.
