Creates a folder, and every missing folder above it.

`Core\IO::makeDirectory` makes sure a folder exists at the path. If folders on the way to it are
missing, `makeDirectory` creates them too. For example, `makeDirectory("logs/2026/09")` creates
`logs`, `logs/2026` and `logs/2026/09` when none of them exist.

When the folder is already there, `makeDirectory` does nothing and does not throw an error. So you
can call it before every write, without checking first.

`makeDirectory` throws an `IOError` when a part of the path is a file. It also throws an `IOError`
when the operating system does not allow the program to create a folder there. An empty path throws a `RuntimeError`.

A program you start from the command line may write any path. On a server, a program may write only
in the folders that `Core\IO::createTemporaryDirectory` made for it. Any other path needs the
`fs.write` capability. Without it, `makeDirectory` throws a `RuntimeError` and creates nothing.
