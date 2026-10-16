Checks whether the path names a file.

`Core\IO::isFile` returns `true` when there is a file at the path. It returns `false` for a folder,
and `false` when there is nothing at the path. It does not throw an error for a missing file.

A symbolic link (a name that points to another file) is followed. So a link to a file returns
`true`.

The program needs the `fs.read` capability for the path. Without it, `isFile` throws a
`RuntimeError`.

**Good to know:** `isFile` checks what is at the path, not the name. A folder named `report.pdf`
is still a folder.
