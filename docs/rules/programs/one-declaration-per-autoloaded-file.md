A file loaded through an autoload root contains exactly one top-level declaration — class, interface,
enum or `type` alias — whose name matches the file's base name. Anything else is
`E_AUTOLOAD_FILE_SHAPE`.

This is not tidiness. Without it, whether `App\Helper` exists in the program depends on whether
something else happened to reference `App\Thing` declared in the same file first, which makes the
program's contents depend on resolution order: a non-reproducible build and an unkeyable cache.

Files reached by `require` are unaffected and may declare anything. The cost is that a helper enum or
`type` alias used by one class needs its own file.
