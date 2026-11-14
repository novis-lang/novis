`nvs fmt` writes every doc comment in one layout. The prose comes first, then one empty `///` line, then
the tags with no empty line between them. A comment with no prose starts with its first tag. The tags go
out in this order: `@param` in the order the signature lists the parameters, then `@return`, `@throws`,
`@see` and `@example`. `@throws`, `@see` and `@example` keep the order their author wrote within their
kind, because that order is read back: `nvs meta --json` lists `see` and `example` in written order. An
`@param` whose name is no parameter goes after the known ones, in written order, and the checker already
reports it (`rule:tooling/doc-comment-tags-are-closed-and-checked`).

A tag moves with its continuation lines. `nvs fmt` never changes the text of a line, only the order of the
lines and the empty lines between them. A line after the first tag that is neither a tag, a continuation
nor empty belongs to no tag, and that comment is left as written.

The signature is read from the syntax tree, from the parameter list of the method below the comment. No
type or name is resolved, so the formatter stays syntactic. Order is never a diagnostic
(`rule:tooling/fmt-is-never-a-diagnostic`): a file out of order is what `nvs fmt --check` reports, as it
reports any other layout. A tag's position is not observable, as an import's is not, so this is one of
the two reorderings the formatter performs (`rule:tooling/fmt-sorts-the-use-block`). Every reader of the
tags shows signature order anyway, so an unformatted file reads the same in `nvs doc`, hover and signature
help.

**A doc tag error never stops `nvs fmt`.** The formatter refuses a file with a parse error, but every doc
tag error the parse raises (`E0128`, `E0138`, `E0139`) leaves the tree whole, so the file is formatted.
Otherwise format-on-save would refuse the file right after the quick fix inserts an `@param` line with no
text yet. The tag errors the checker raises never reach the formatter, which only parses.

What it costs: nothing at run time. A file with a comment out of order costs `nvs fmt` one more copy of
its text and one more parse.

**Declined:**
- **A diagnostic for tags out of order.** Layout is `nvs fmt`'s job and never a diagnostic.
- **Moving prose written between tags.** A line after a tag is that tag's text, so there is no prose
  there to move.
- **Leaving the order free.** Every reader already shows signature order, but two authors writing the
  same tags in two orders make every review of a doc comment a diff of its order.
