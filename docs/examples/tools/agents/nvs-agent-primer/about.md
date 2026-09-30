`nvs agent primer` prints the one document that a coding agent reads before it writes Novis code.

The document has these parts, in this order. The first explains how to look things up with `nvs agent find` and
`nvs agent show`. The second is one complete program with an explanation of each part. The third
explains capabilities, with the smallest `nvs.toml` that allows a program to read a file. The
fourth is a set of tables of PHP syntax that Novis does not have, each with its error code. The
last is a list of the chapters of the reference.

The command prints to standard output and writes no file. Each part is a section of the reference
that is built into the `nvs` binary, copied whole. The primer therefore describes the language that
this binary compiles.
