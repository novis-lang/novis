Every path in an `autoload` declaration is text in quotes, relative to the folder of the file that
contains the declaration. A path in a variable does not compile (`E0123`). An `autoload` declaration
is written at the top level of a file, and inside an `if` or a method it does not compile (`E0233`).
It covers the whole file, so it may come after the code that uses its classes.

Every part of a prefix starts with a capital letter. A prefix such as `'app'` or `'App\*'` does not
compile (`E0327`). One part may be `{.}`, `{..}` or `{../..}`. That part is replaced by the name of
the folder the path points to from the declaring file.

A file found through `autoload` declares one class, interface, enum or type alias. Its full name,
namespace included, must be the name the file was found by. Otherwise the program does not compile
(`E0317`). Folder and file names are case-sensitive, so `App\text\Greeter` does not find
`Text/Greeter.nvs`.

A class that no folder has is `E0303`, the same error as any name that is not declared. A folder that
does not exist is allowed. `nvs check --autoload-map` prints every prefix with its folders, and lists
a folder that does not exist under `missing`.

The map comes from every `autoload` in every file the program reads. `Core\Program::implementing<I>()`
uses it too: it finds each class that implements the interface `I`, even when no code names the class.
