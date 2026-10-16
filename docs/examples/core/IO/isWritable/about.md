Checks whether the program may write to what is at the path.

`Core\IO::isWritable` returns `true` when the operating system allows this program to change the
file, or to create files in the folder, at the path. It returns `false` when it does not. It also
returns `false` when there is nothing at the path.

This means a file you are about to create is not writable yet. To check that you can create a new
file, pass the folder it goes into.

The answer is true at the moment of the check. A `Core\IO::write` that follows can still throw an
`IOError`.

The program needs the `fs.write` capability for the path. Without it, `isWritable` throws a
`RuntimeError`, even when the program may read the path.
