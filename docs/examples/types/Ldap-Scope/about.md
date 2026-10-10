Says how far below its `base` a search looks.

A directory is a tree of entries, and a search starts at one entry, its `base`. The `scope` option
of a search is one of three cases. `Scope::Base` returns only the `base` entry itself, which is a
quick way to read one entry with a filter. `Scope::OneLevel` returns the entries directly below
`base`. `Scope::Subtree` returns `base` and every entry below it, at any depth.

A search without `scope` uses `Scope::Subtree`.
