`nvs agent primer` prints the one document that a coding agent reads before it writes Novis code.

The document has five parts, in this order. The first explains how to look things up with
`nvs agent find` and `nvs agent show`. The second is one complete program with an explanation of
each part. The third explains capabilities (the permissions a program needs, such as reading a
file), with the smallest `nvs.toml` that allows a program to read a file. The fourth explains how
`nvs reload` makes a running server use the code you changed. The last is a list of the chapters
of the reference.

The command prints to standard output and writes no file. Each part is a whole section of the
reference, copied from the `nvs` binary. Because of this, the primer always describes the language
that this binary compiles.
