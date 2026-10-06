Checks if a path was a regular file.

`$info->isFile()` works on the `Core\IO\Metadata` value that `Core\IO::stat` returns. It returns
`true` when the path was a regular file, and `false` when it was a folder or anything else.

`isDir` checks for a folder. A regular file returns `true` for `isFile` and `false` for `isDir`.
Some rare kinds of path, such as a device or a named pipe, return `false` for both.

The answer is from the moment `stat` was called. If the file is later replaced by a folder with
the same name, this value still returns `true`. Call `stat` again to check the path again.
