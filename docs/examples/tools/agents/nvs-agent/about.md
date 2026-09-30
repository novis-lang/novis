`nvs agent` is how a coding agent (an AI assistant that writes code) looks up the Novis language
and its library.

`nvs agent index` prints one line for each `Core` method, enum, exception and attribute. It then
prints one line for each chapter and each heading of the reference. `nvs agent find <query>` prints
the index lines whose name matches the query. `nvs agent show <symbol>` prints the full entry. For
a method, that is the signature, the description, the parameters and the errors. For a heading, it
is that section of the reference.

The commands read from the `nvs` binary and write nothing to disk, so a result always describes the
version that is installed. The usual order is `find`, then `show`, then write the program, then
`nvs check`.

**Good to know:** when no name matches, `find` prints nothing and the exit status is `0`. The index
is complete, so no `Core` name and no heading has that word. When `show` does not find a symbol, it
exits with an error and prints the nearest names.
