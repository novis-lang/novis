A directive holding several values is a TOML array, and a directive that is a repeated record with
more than one field is an array of tables:

```toml
[debug]
mode = ["coverage", "branch"]                 # [] is off

[capabilities]
script.spawn = ["/srv/www/jobs", "C:\\srv\\jobs"]

[[extension]]
path   = "image.nvsx"
sha256 = "…"
```

Nothing is a comma-joined or `PATH`-joined string. The `:`-joined root list could not spell a Windows
absolute path at all, since every one contains the separator, and the extension hash pin gets a shape
instead of a convention — which matters because that pin is what accepting a precompiled binary rests
on.

A capability's name is dotted, and a dotted TOML key *is* table nesting: `script.spawn` under
`[capabilities]` and a `[capabilities.script]` block with a `spawn` key are the same thing, and the
registry names the directive by its full dotted path either way. Neither spelling needs quoting.

Across files the two shapes behave differently, which is
`rule:config/a-value-array-replaces-and-a-table-appends`.
