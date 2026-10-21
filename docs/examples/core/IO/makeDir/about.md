Creates a folder, and every missing folder above it.

`Core\IO::makeDir` makes sure a folder exists at the path. If folders on the way to it are
missing, `makeDir` creates them too. For example, `makeDir("logs/2026/09")` creates `logs`,
`logs/2026` and `logs/2026/09` when none of them exist.

When the folder is already there, `makeDir` does nothing and does not throw an error. So you can
call it before every write, without checking first.

`makeDir` throws an `IOError` when the path is empty, when a part of the path is a file, or when
the operating system does not allow the program to create a folder there.

A program you start from the command line may write any path. On a server, a program may write
only in the folders that `Core\IO::temporaryDir` made for it. Any other path needs the `fs.write`
capability. Without it, `makeDir` throws a `RuntimeError` and creates nothing.
