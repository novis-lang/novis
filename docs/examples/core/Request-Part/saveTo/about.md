Writes one uploaded file to a file on disk. It reads the upload in small pieces and writes each piece
at once, so even a very large file uses little memory. The program needs the `fs.write` capability for
the path.

`{max: …}` sets the largest size in bytes. Without it, there is no limit. A larger file throws a
`RuntimeError`, and the part that was already written is deleted. `saveTo` does not replace a file
that already exists and throws an error instead. `{overwrite: true}` allows it.

The file name from `filename()` is `tainted`, so `saveTo` does not accept it as a path. Pass it
through `Core\IO::within` first. Call `saveTo` while the `Core\Request::files()` loop is on this file.
After the loop moves on, it throws a `LogicError`. The content can be read only once.

The examples show how to save a file, what happens when it is too large, and how to store files
under the names the client sent.
