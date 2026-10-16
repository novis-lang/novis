Returns every file and folder inside a folder, including the ones in its sub-folders.

`Core\IO::walk` returns one string for each file and folder it finds. Each string is a path relative
to the starting folder, such as `images/logo.png`. Use `Core\Path::join` to put the starting folder
in front again. A symbolic link is in the result, but `walk` does not follow it.

The entries of one folder come before the entries of the folders inside it. Inside one folder, the
order is the order the operating system gives. The folders are read when you call `walk`, so a file
written later is not in the result.

`walk` throws an `IOError` when the path does not exist, when it is a file, or when a folder cannot
be read. The program needs the `fs.read` capability for every folder it reads. Without it, `walk`
throws a `RuntimeError`.
