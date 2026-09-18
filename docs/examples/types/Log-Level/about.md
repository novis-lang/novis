How loud a log record is. Every record your program writes carries one of five levels, and whoever
runs the server decides how much of that they want to keep.

The five, quietest first, are `Debug`, `Info`, `Warn`, `Error` and `Critical`. `Debug` is detail for
whoever is watching this one run. `Info` is something the program did that an operator would want in
the record. `Warn` is something recoverable that nobody chose — a retry, a fallback. `Error` means an
operation failed. `Critical` is for a program that can no longer do its job at all.

The level a deployment keeps is a floor rather than a filter: set `[log] level` to `Warn` and
everything louder than `Warn` is kept too. A deployment that has set nothing keeps every record, so a
`Debug` line you left in your code stays in the log until somebody chooses otherwise.

**Good to know:** the level decides how loud a record is, never who reads it. Where records go is a
separate setting, and the same record goes to the same place whatever its level.
