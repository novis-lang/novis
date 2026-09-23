Deletes one file.

`Core\IO::remove` takes the path of a file and deletes it. After the call, the file does not exist
and `Core\IO::exists` returns `false` for that path.

`remove` deletes files only. To delete an empty folder, use `Core\IO::removeDir`. There is no way to
delete a folder and everything in it with one call. A program that wants this lists the folder and
deletes each file first.

`remove` throws an `IOError` when there is no file at the path, when the path is a folder, or when
the operating system does not allow the deletion. So a program always knows if a file was really
deleted.

Deleting a file changes it, so the program needs the `fs.write` capability for the path. Without
it, `remove` throws a `RuntimeError` and the file stays.

This replaces PHP's `unlink`.
