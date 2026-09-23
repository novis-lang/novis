Checks whether a file or a folder exists at a path.

`Core\IO::exists` returns `true` when the path names a file or a folder. It returns `false` when
there is nothing at the path. A missing file is a normal result here, so `exists` does not throw an
error for it.

The program needs the `fs.read` capability for the path. Without it, `exists` throws a
`RuntimeError`. It does not return `false`, so `false` always means that nothing is there.

**Good to know:** to find out what kind of thing is at the path, use `Core\IO::isFile` or
`Core\IO::isDir`.

This replaces PHP's `file_exists`.
