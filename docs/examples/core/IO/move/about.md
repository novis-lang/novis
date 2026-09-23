Moves a file to a new path, or gives it a new name.

`Core\IO::move` takes two paths. The file at the first path moves to the second path. After the
call, the first path is empty. If a file is already at the second path, `move` replaces it.

The move happens in one step. Another program that reads the second path sees either the old
file or the whole new file, and never half of one. Because of this, a program often writes new
content to a temporary file first, and then moves it over the real file.

`move` does not move a file to a different disk. It throws an `IOError` instead. Use
`Core\IO::copy` and then `Core\IO::remove` for that. `move` also throws an `IOError` when there is
no file at the first path, or when the folder of the second path does not exist.

The program needs the `fs.write` capability for both paths. Without it, `move` throws a
`RuntimeError`.

This replaces PHP's `rename`.
