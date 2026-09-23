Creates a new, empty folder for the files a program only needs while it runs.

`Core\IO::temporaryDir` creates a folder and returns its full path. The folder is new and empty,
and no other program uses it. Each call creates a different folder. The folders are inside the
folder set by `[io] temp_root`. Without that setting, they are inside a `novis` folder in the
temporary folder of the system.

When the program ends, Novis deletes each folder it created, with every file in it. You do not
need to delete it yourself, but you can. A file that must still exist after the program ends
belongs somewhere else.

The program needs the `fs.write` capability for the new folder. Without it,
`temporaryDir` throws a `RuntimeError`. It throws an `IOError` when the folder cannot be created.

This replaces PHP's `sys_get_temp_dir` and `tempnam`.
