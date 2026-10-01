Returns the full path of the folder that contains the source file with the call.

The compiler writes the path in place of the call, so the call does no work when the program runs.
The path is the same from every folder you start the program in. Give `thisDir` a relative path
as a string literal, such as `'data/rates.json'`, and it adds that path to the folder. `.` and `..`
are removed from the result.

Replaces PHP's `__DIR__`.

**Good to know:** the path you add must be written as a string literal. A variable does not
compile. For a part that your program builds, use `Core\Path::join(Core\Path::thisDir(), $part)`.

**The examples below** find a file beside the program, go to the folder above, and join a name
that the program builds.
