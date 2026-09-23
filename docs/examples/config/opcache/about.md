Controls how a running server notices that a source file changed.

Novis compiles your code and keeps the result. The server checks the files behind that result in the
background, so a changed file reaches the next request without a restart. `validate` is how it checks:
`mtime` compares the time each file was last changed, and `hash` reads each whole file. `mtime` is
the default. `revalidate_freq` is how often it checks. `settle` is how long the files must stay
unchanged before the server compiles them again.

The default of `settle` depends on the run mode: `1s` in production and `100ms` in development. A
value written in the file is used in both modes.

**Good to know:** a program cannot change any of these settings. Only the person who runs the server
may change them.
