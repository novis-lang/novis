The path is read while the program compiles. If no file is at that path, the program does not compile
(`E0311`). The path may be text in quotes, a class constant, or several of these joined with `.`.

A path built from a variable is not read while compiling. When the program reaches that `require`,
it throws a `RuntimeError`, even if the file exists.

The other file's top-level code is checked on its own. Its variables are new variables, even when a
name is the same as one of yours: after the `require`, your `$x` still has its old value.

`include`, `include_once` and `require_once` do not compile (`E0221`). The message names `require`.
A file of declarations that must be read once is found through `autoload`, which never reads a file
twice.
