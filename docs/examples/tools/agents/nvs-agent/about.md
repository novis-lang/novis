`nvs agent` is how a coding agent (an AI assistant that writes code) looks up the Novis language
and its library.

`nvs agent index` prints one line for each `Core` method, enum, exception and attribute, and for
each chapter and heading of the reference. `nvs agent find <query>` prints the index lines whose
name matches the query. `nvs agent show <symbol>` prints the full entry: for a method, the
signature, the description, the parameters and the errors.

The commands read from the `nvs` binary, so a result always describes the installed version. The
usual order is `find`, then `show`, then write the program, then `nvs check`.

With `--json`, `index`, `find` and `show` print one JSON document for a tool to read.

**Good to know:** when no name matches, `find` prints nothing and the exit status is `0`. When
`show` does not find a symbol, it exits with an error and prints the nearest names.
