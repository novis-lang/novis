Checks whether a file or a folder exists at a path.

`Core\IO::exists` returns `true` when the path names a file or a folder. It returns `false` when
there is nothing at the path. A missing file is a normal result here, so `exists` does not throw an
error for it.

A program you start from the command line may read any path. On a server, a program may read only
the files in its application folder, except `nvs.toml` and names that start with `.`. Any other
path needs the `fs.read` capability. Without it, `exists` throws a `RuntimeError`. It does not
return `false`, so `false` always means that nothing is there.

**Good to know:** to find out what kind of thing is at the path, use `Core\IO::isFile` or
`Core\IO::isDir`.
