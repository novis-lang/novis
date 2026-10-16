Returns the size, the time of the last change and the kind of a file or folder, with one call.

`Core\IO::stat` takes a path and returns a `Core\IO\Metadata`. This object has four methods:
`size()` returns the size in bytes, `modifiedAt()` returns the time of the last change,
`isFile()` returns `true` for a file, and `isDir()` returns `true` for a folder.

`Core\IO::size`, `Core\IO::modifiedAt`, `Core\IO::isFile` and `Core\IO::isDir` each return one
of these values. When a program needs more than one of them for the same path, `stat` is faster,
because it asks the operating system only once.

The `Core\IO\Metadata` object does not change. If the file changes after the call, the object
still has the old values. Call `stat` again to get the new ones.

`stat` throws an `IOError` when there is nothing at the path. The program needs the `fs.read`
capability for the path. Without it, `stat` throws a `RuntimeError`.
