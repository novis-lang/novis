The converter needs an AST for **PHP 7.4 through 8.6**, extendable. 8.0 is the hard requirement and
7.4 came free with the parser chosen; a dialect below 7.4 is **refused, not guessed at** — the file
becomes `rule:tooling/convert-drops-nothing`'s commented-out file, and the report names the version
it was tried under. Extending the range is a row: a new dialect value plus a branch on any rule whose
behaviour it changed.

The front end is **`php-rs-parser`** (with `php-ast`, `php-lexer` and `phpdoc-parser`), BSD-3-Clause
and pure Rust, **pinned to an exact version** and upgraded deliberately. It was chosen over
`mago-syntax` by measurement: it names the constructs PHP 8.0 removed, rejects a type PHP itself
rejects, carries a version knob, states a semantic-rejection contract — at least one diagnostic iff
`php -l` would reject the input at the configured version — and returns an owned tree with comments
in source order and doc-blocks attached to their declaration.

Three bounds hold the dependency in place. **The passes see only `nvs_convert::php`** — our own
facade over node kinds, spans and comments, written before any pass and the only module allowed to
name the parser crate — so replacing it is one module, not a rewrite. **The parser is behind a Cargo
feature and is never linked into the server binary**: a PHP front end has no business on a machine
serving requests. **No PHP binary is required to convert.** PHP is the differential oracle that
proves an E rule (`rule:tooling/convert-equivalent-is-proven`), a development-side dependency; an
installed PHP at convert time would make output depend on which build the user has, breaking
`rule:tooling/convert-is-deterministic`. The parser is not vendored: owning it would mean owning
every future PHP release.
