Checks whether the program may read what is at the path.

`Core\IO::isReadable` returns `true` when the operating system allows this program to read the file
or folder at the path. It returns `false` when it does not. It also returns `false` when there is
nothing at the path, and it does not throw an error for that.

The answer is true at the moment of the check. Another program can delete or change the file right
after it. So a `Core\IO::read` that follows can still throw an `IOError`, and your program should
still catch it.

The program needs the `fs.read` capability for the path. Without it, `isReadable` throws a
`RuntimeError`. It does not return `false`.

**Good to know:** use `isReadable` when a missing file is a normal case, such as an optional
settings file.
