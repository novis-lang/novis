`require` runs another file right where you write it, as part of the program that is already running.

The other file's own lines run in order at that point, every time your program reaches the `require`.
There is no once-only version. A `require` is also a value: whatever the other file hands back with
`return` at its top level is what you get, and you convert it with `as` to the type you expect. A
file that returns nothing hands back `1`. The path is written relative to the folder holding the file
you wrote the `require` in.

**Good to know:** one kind of name crosses between the two files and one does not. Every class,
interface, enum and type alias the other file declares is yours to use, as if you had typed it in.
Variables are not shared, so the other file's `$name` is its own and yours stays yours. To hand
something across, put it in a class.
