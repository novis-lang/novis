Returns the full path of the source file that contains the call.

The compiler writes the path in place of the call, so the call does no work when the program runs.
The path is the same from every folder you start the program in. A call in a file that you load
with `require` returns the path of that file, not the path of the first file. Use
`Core\Path::thisDir` for the folder of the file.

Replaces PHP's `__FILE__`.

**Good to know:** the full path is different on every computer. Print `Core\Path::basename` of
it when you only need the file name.

**The examples below** print the name of the file, compare its folder with `Core\Path::thisDir`,
and put the name of a tool into its usage message.
