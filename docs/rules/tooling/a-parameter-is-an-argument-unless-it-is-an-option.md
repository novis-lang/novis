**A parameter is a positional argument unless it carries `#[Option]`.** One sentence, with no
inference from defaults or types. A `bool` `#[Option]` is a flag; a trailing `array<string>` parameter
is variadic. It is the last parameter without `#[Option]`, takes every positional word left once the
ones before it are filled, and is an empty array when none is left. An `array<string>` anywhere else is
`E0746`, and an array has no constant default (`E0451`). An argument the command line leaves out takes the parameter's declared default, so
`#[Option] uint $retries = 3` is optional and a parameter declaring none is required — the flag being
the one exception, since being *written* is how it is given, and an unwritten `bool $loud = true` is
still `false`. Nothing new is admitted: a default is already a literal of the parameter's own type, so
the table carries the folded constant as the text a command line would have written and the matcher
converts it exactly as it converts a written word.

**A matched value's type comes from the parameter.** `uint $retries` is converted during matching
and, because a checked conversion launders (`rule:security/route-capture-is-laundered-by-its-type`),
arrives **unqualified**; `string $target` arrives `tainted`. An enum converts by its case name, a
set of allowed values by its word (`rule:routing/a-capture-narrows-to-a-closed-set`), and a non-numeric
`--retries` is a usage error rather than a crash. This is the route capture's rule applied unchanged
— no new laundering rule exists for the command line. An `#[Option]` written outside a `#[Command]`
method supplies nothing, because there is no table for it to be a row of.
