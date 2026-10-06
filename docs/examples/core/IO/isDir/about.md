Checks whether the path names a folder.

`Core\IO::isDir` returns `true` when there is a folder (a directory) at the path. It returns `false`
for a file, and `false` when there is nothing at the path. It does not throw an error for a missing
folder.

A symbolic link (a name that points to another folder) is followed. So a link to a folder returns
`true`.

The program needs the `fs.read` capability for the path. Without it, `isDir` throws a
`RuntimeError`.

**Good to know:** use `isDir` before `Core\IO::list`, which throws an error for a file.
