Checks if a path was a regular file.

`$info->isFile()` works on the `Core\IO\Metadata` value that `Core\IO::metadata` returns. It returns
`true` when the path was a regular file, and `false` when it was a folder or anything else.

`isDirectory` checks for a folder. A regular file returns `true` for `isFile` and `false` for
`isDirectory`. Some rare kinds of path, such as a device or a named pipe, return `false` for both.

The answer is from the moment `metadata` was called. If the file is later replaced by a folder with
the same name, this value still returns `true`. Call `metadata` again to check the path again.
