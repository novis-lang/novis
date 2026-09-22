A line comment opened by exactly three slashes is a doc comment, lexed as its own trivia kind. Four or
more slashes is an ordinary comment, as in Rust and for the same reason: a divider line of slashes is not
documentation. A `#` comment is never a doc comment whatever its length — `#[` already opens an attribute,
and a second doc spelling is what `rule:statements/nothing-gets-a-second-name` refuses.

```php
/// The price in cents. Money is `decimal`, never `float`.
/// A negative amount throws; zero is allowed and is a no-op.
public function charge(uint $cents): void { … }

// An ordinary comment. Nothing reads it.
//// ─────────────────────────────  also ordinary
```

The comment documents the declaration it precedes (`rule:tooling/doc-comment-attaches-to-the-next-declaration`),
its body is Markdown plus two tags (`rule:tooling/doc-comment-tags-are-see-and-example`), and it belongs
to *Novis* source only: a `Core` member is Rust and documents itself in the registry
(`rule:core-api/reference-card`), so it never carries one. There is no module-level doc spelling either — a
file's declarations are its surface, and the class is the unit.

**A `/** … */` block directly above a declaration is warned about**, `W1011`: it is the PHPDoc habit, it
documents nothing here, and a program that carries one on every class compiles clean and loses its whole
documentation without a word. The trigger is narrow — a block opening with `/**` and a body, across no blank
line from the declaration or from the `///` run above it — so a divider block, a block above a statement and
`/**/` stay silent, and the help is the one-token fix: *write `///` for a doc comment*. It is a warning and not
a refusal because the program is correct; only its documentation is missing.

A trivium's classification never reaches the checker, so no program output moves; what a compile spends is
one variant test per line comment in the lexer, one `/**` test per block comment, and nothing on any
request path. A doc comment rendered
to HTML crosses the bidi boundary the lexer already checks for every comment span
(`rule:security/bidi-boundaries`), and reuses that check rather than growing a second one.
