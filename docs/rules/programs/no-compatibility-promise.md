The PHP-shaped syntax is an on-ramp. What may be said about it is bounded.

**Permitted:** that Novis is familiar to a PHP developer, that `<?nvs` and inline HTML work the way
they expect, and that a PHP developer can port an application's own code by hand.

**Forbidden in any document, error message or landing page:** "PHP compatible", "drop-in", "runs your
PHP", "migrate your Laravel app", or any phrasing a reader could reasonably take as a promise that
existing packages, frameworks or code run. Existing PHP does not run unconverted, and this is a rule
about how the project speaks, not only a fact it knows.

**Naming `.php` as an input a tool accepts is one of those phrasings.** The front end reads a file's
content and never its extension, so a `.php` in a `--help` line or a manual's syntax block means only
"this ignores the name" and is read as "this runs PHP" — and a `<?php` tag is refused with `E0229`
whatever the file is called. `rule:packaging/help-text-speaks-to-its-reader` is the same rule at the
one surface every user meets first.

There is no mechanical migration from PHP, shipped or planned, and no table that maps a PHP function
or construct to a Novis one. A program is ported by rethinking it — by hand, or with an AI agent and
`nvs agent` — because a facade, a trait or an active-record model has no Novis form a rewriter could
pick for its author.

**Where PHP is named at all is bounded too: origins stay honest, not prominent.** One line on the home
page and one in `README.md` say Novis grew out of PHP's ideas. Elsewhere PHP is named only on the one
concept page, `docs/reference/tools/30-php-differences.md`, with its examples and attacks; on
`why-novis` and the claims pages that compare the languages; in the PHP benchmarks; and in the
diagnostics for a PHP habit — `E0229` for `<?php`, `W1004`, and the superglobals and PHP-constant
reference cards — because each of those is help for one error a person meets, not a table to
translate from. The `php-differences` id and paths, the `E_PHP_*` constants and a conformance case
whose file name mentions PHP keep their names.
