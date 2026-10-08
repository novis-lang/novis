The directory where Novis stores compiled code.

Compiling a program takes time, so Novis writes the compiled code to disk and loads it again at the
next start. `[opcache] file_cache_dir` is that directory. It is separate from `[cache]`, which is
the data cache that your application uses.

A program cannot change this key, and it cannot turn the cache off. The server runs the compiled
code that it finds in this directory. A program that could change the directory could choose which
code runs.

**Good to know:** a running server applies a new directory when you save the file, and the next
compile writes there. Programs that are already compiled keep running. If other accounts can write
to the new directory, the server keeps the old directory. The server then writes to its log that the
key was not applied.

The example prints the cache settings and tries to change the directory. `Core\Config::set` returns
`false`.
