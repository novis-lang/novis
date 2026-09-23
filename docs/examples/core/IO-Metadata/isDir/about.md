Checks if a path was a folder.

`$info->isDir()` works on the `Core\IO\Metadata` value that `Core\IO::stat` returns. It returns
`true` when the path was a folder, and `false` when it was a file or anything else.

`isFile` checks for a regular file. A folder returns `true` for `isDir` and `false` for
`isFile`. Some rare kinds of path, such as a device or a named pipe, return `false` for both.

The answer is from the moment `stat` was called. If the folder is later replaced by a file with
the same name, this value still returns `true`. Call `stat` again to check the path again.

This replaces PHP's `is_dir`.
