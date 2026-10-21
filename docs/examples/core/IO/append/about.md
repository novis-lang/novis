Adds text to the end of a file. If the file does not exist, `Core\IO::append` creates it.

What the file already contains stays as it is, and the new text comes after it. This is the
difference from `Core\IO::write`, which replaces the whole file. `append` adds no line break of its
own, so end each line with `"\n"` yourself.

A program you start from the command line may write any path. On a server, a program may write
only in the folders that `Core\IO::temporaryDir` made for it. Any other path needs the `fs.write`
capability. Without it, `append` throws a `RuntimeError` and creates nothing. When the system cannot write the file, for example because the
folder does not exist, `append` throws an `IOError`.

**Good to know:** the system puts every call at the end of the file at the moment it writes. Two
programs that add lines to the same log file do not write over each other's lines.
