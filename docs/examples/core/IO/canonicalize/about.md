Gives the full, absolute path of a file or folder that exists.

`Core\IO::canonicalize` removes every `.` and `..` from the path and follows every symbolic link.
The result is the one path the system uses for that file. Two different ways to write the path of
one file give the same result, so you can compare the results to find out whether two paths name
the same file.

Every part of the path must exist. For a path with nothing at it, `canonicalize` throws an
`IOError`. The program needs the `fs.read` capability for the path, or `canonicalize` throws a
`RuntimeError`.

**Good to know:** `canonicalize` does not check that a path stays inside a folder. To check a path
that a user sent, use `Core\IO::within`.
