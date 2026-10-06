A string literal is rewritten to single quotes, with two exceptions that both use double quotes instead:
a literal that interpolates — only a double-quoted string or a heredoc can — and a
literal containing a single quote that single-quoting would force to be escaped.

A literal holding a backslash escape keeps its double quotes, which is the same exception read over
the whole escape grammar rather than over the one character: `\n` is a newline between double quotes
and a backslash and an `n` between single ones, and single quotes have only `\\` and `\'` to offer
back. Respelling one would change the string's value, so the test the formatter actually applies is
that both spellings name the same string — a body with no `'` and no `\` in it.

Heredoc and nowdoc bodies and every comment are left byte-for-byte untouched. Rewriting a heredoc's body
would change the program's own string value, not its layout; and a comment is content the formatter has no
opinion about.

This is the one rewrite, with `rule:tooling/fmt-trailing-commas`, that goes beyond whitespace. A
whitespace-only formatter was rejected because two semantically identical files would still differ
byte-for-byte after formatting, which undercuts the point of having one canonical layout at all.
