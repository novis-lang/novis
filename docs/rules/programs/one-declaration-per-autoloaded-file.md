A file loaded through an autoload root contains exactly one top-level declaration — class, interface,
enum or `type` alias — whose qualified name is the one the map found the file by: the prefix, the
directories under the root, and the file's base name. Anything else, a namespace that differs
included, is `E_AUTOLOAD_FILE_SHAPE`.

This is not tidiness. Without it, whether `App\Helper` exists in the program depends on whether
something else happened to reference `App\Thing` declared in the same file first, which makes the
program's contents depend on resolution order: a non-reproducible build and an unkeyable cache. A file
declaring a namespace other than its map path does the same thing another way: its name would exist
only when something probed or `implementing` scanned that path.

Files reached by `require` are unaffected and may declare anything. The cost is that a helper enum or
`type` alias used by one class needs its own file.
