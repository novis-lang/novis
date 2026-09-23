Returns the names of the files and folders inside a folder.

`Core\IO::list` reads one folder and returns an array with one string for each file and folder
directly inside it. Each string is a bare name, such as `report.pdf`, without the folder path in
front. Use `Core\Path::join` to build the full path again. The special names `.` and `..` are
never in the result. `list` does not look inside the folders it finds.

The order of the names is the order the operating system gives, and it can differ between
systems. Call `Core\Arr::sort` on the result when you need alphabetical order. An empty folder
returns an empty array.

`list` throws an `IOError` when the path does not exist, when it is a file, or when the operating
system does not allow the program to read the folder.

The program needs the `fs.read` capability for the path. Without it, `list` throws a
`RuntimeError`.

This replaces PHP's `scandir`.
