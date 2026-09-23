Opens a file and returns a handle that reads or writes it a part at a time.

`Core\IO::open` takes the path and a `Core\IO\FileMode`. The mode says what the handle can do:

- `Read` reads a file that must already exist.
- `Write` creates the file, or empties it if it is already there.
- `Append` writes at the end of the file and keeps what it has.
- `ReadWrite` reads and writes, creates the file if it is not there, and keeps what it has.

The result is a `Core\IO\File`. Call `close` when you are done. Handles that are still open at the
end of the program are closed for you.

`Read` needs the `fs.read` capability for the path, `Write` and `Append` need `fs.write`, and
`ReadWrite` needs both. Without it, `open` throws a `RuntimeError`. When the file cannot be opened,
for example because it does not exist, `open` throws an `IOError`.

This replaces PHP's `fopen`.
