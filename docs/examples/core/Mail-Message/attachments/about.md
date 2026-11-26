Returns the files attached to a received message, and the images shown inside its HTML body.

Each file is a `Core\Mail\Attachment`, in the order of the message. `name()`, `type()` and `bytes()`
read its file name, its media type and its content. The plain text and HTML bodies are not in the
array. A message with no files gives an empty array.

The sender chose every file name and every byte, so all of them are tainted. Tainted means that
the value comes from outside your program. A tainted name cannot be part of a path, so your program
chooses its own file name when it saves a file.

A name with a folder in it, such as `../../etc/passwd`, is shortened to its last part, `passwd`.
When nothing is left, `name()` returns `null`.

**Good to know:** you can pass a received attachment to `Core\Mail::send`. An attachment with no
name throws an error there.

**The examples below** show how to list the files, save a file, and forward a file.
