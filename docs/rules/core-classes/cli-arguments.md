`Core\Cli::arguments()` answers the words the program was started with, as an
`array<tainted string>`. It replaces `$argv` and `$argc` in one place: a count is the array's length,
so there is nothing for two spellings to disagree about.

The elements are `tainted`, because a word typed at a shell is user-derived data like any other, and
a sink refuses one. A word that looks like syntax comes back as the value it is, unsplit and
uninterpreted.

Within a request tree the answer is the same at every depth: it reflects how the **process** was
invoked, which is a process-wide fact rather than a per-isolate one, so a spawned isolate neither
fakes nor suppresses it the way it does for request state. A program reading arguments a *request*
supplied wants `rule:core-classes/script-args` instead.

The record this rule comes from specified `args()`/`argc()` and a throw when called while serving
HTTP. The shipped member is `arguments()`, and inside a request it answers empty rather than throwing
— the launcher writes the command line and only the CLI entry point writes one.
