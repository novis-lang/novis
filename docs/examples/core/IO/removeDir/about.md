Deletes one empty folder.

`Core\IO::removeDir` takes the path of a folder and deletes it. The folder must be empty. After the
call, the folder does not exist and `Core\IO::isDir` returns `false` for that path.

There is no way to delete a folder and everything in it with one call. With such a call, one wrong
path could delete a whole tree of files. A program that wants to delete a full folder lists it,
deletes each file with `Core\IO::remove` and each inner folder the same way, and then deletes the
folder itself.

`removeDir` throws an `IOError` when the folder still has files or folders in it, when nothing is
at the path, when the path is a file, or when the operating system does not allow the deletion. In
each case, nothing is deleted.

Deleting a folder changes the disk, so the program needs the `fs.write` capability for the path.
Without it, `removeDir` throws a `RuntimeError`.
