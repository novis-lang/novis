Returns the files attached to a received message. The images shown inside its HTML body are
attachments too, so they are in the result.

Each file is a `Core\Mail\Attachment`, in the order of the message. `name()`, `type()` and `bytes()`
read its file name, its media type and its content. A message is made of parts. A part is one piece
of a message, such as the text, the HTML or a file. The plain text and HTML bodies are not in the
array. A message with no files gives an empty array.

The sender chose every file name and every byte, so all of them are tainted. Tainted means that
the value comes from outside your program, so you must check or escape it before you use it. Escape
a name with `Core\Html::escape` before you put it on a page. A tainted name cannot be part of a file
path, so your program chooses its own file name when it saves a file.

A name with a folder in it, such as `../../etc/passwd`, is shortened to its last part, `passwd`.
When nothing is left, `name()` returns `null`.

**Good to know:** you can pass a received attachment to `Core\Mail::send`. An attachment with no
name throws an error there.

**The examples below** show how to list the files, save a file, and forward a file.
