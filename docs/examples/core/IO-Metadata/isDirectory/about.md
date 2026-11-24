Checks if a path was a folder.

`$info->isDirectory()` works on the `Core\IO\Metadata` value that `Core\IO::metadata` returns. It
returns `true` when the path was a folder, and `false` when it was a file or anything else.

`isFile` checks for a regular file. A folder returns `true` for `isDirectory` and `false` for
`isFile`. Some rare kinds of path, such as a device or a named pipe, return `false` for both.

The answer is from the moment `metadata` was called. If the folder is later replaced by a file with
the same name, this value still returns `true`. Call `metadata` again to check the path again.
