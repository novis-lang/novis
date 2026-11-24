Deletes one empty folder.

`Core\IO::removeDirectory` takes the path of a folder and deletes it. The folder must be empty.
After the call, the folder does not exist and `Core\IO::isDirectory` returns `false` for that path.

There is no way to delete a folder and everything in it with one call. With such a call, one wrong
path could delete a whole tree of files. To delete a full folder, list it first. Delete each file
with `Core\IO::remove` and each inner folder the same way. Then delete the folder itself.

`removeDirectory` throws an `IOError` when the folder still has files or folders in it. It throws
the same error when nothing is at the path, or when the path is a file. It also throws an `IOError` when the operating system
does not allow the deletion. In each case, nothing is deleted.

A program you start from the command line may delete any folder. On a server, a program may delete
folders only inside the folders that `Core\IO::createTemporaryDirectory` made for it. Any other path
needs the `fs.write` capability. Without it, `removeDirectory` throws a `RuntimeError`.
