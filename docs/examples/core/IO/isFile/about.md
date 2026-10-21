Checks whether the path names a file.

`Core\IO::isFile` returns `true` when there is a file at the path. It returns `false` for a folder,
and `false` when there is nothing at the path. It does not throw an error for a missing file.

A symbolic link (a name that points to another file) is followed. So a link to a file returns
`true`.

A program you start from the command line may read any path. On a server, a program may read only
the files in its application folder, except `nvs.toml` and names that start with `.`. Any other
path needs the `fs.read` capability. Without it, `isFile` throws a `RuntimeError`.

**Good to know:** `isFile` checks what is at the path, not the name. A folder named `report.pdf`
is still a folder.
