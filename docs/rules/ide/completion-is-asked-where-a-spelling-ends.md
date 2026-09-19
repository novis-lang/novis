The completion trigger characters are `>`, `:`, `\`, `$` and `?` — the last character of `->`, of `::`, of
a namespace separator, of a variable's `$` and of a half-written `<?` — and a request one of them raised is
answered only where the text before the cursor ends in that whole spelling. Each of the five is also an
operator's character, and an editor asks on the keystroke: answering `$a >` or `Core\Str:` with whatever
the position offers opens a list nobody asked for. `-` is not a trigger, because it finishes nothing. A
request the developer raised by hand, or by typing a name, is not held to this.

Three spellings narrow what is offered whoever asked. **After `$`, only variables**: the ones the innermost
body declared, each replacing the `$` already typed, because a lone `$` is no word to a client and one left
to choose its own range would write `$$name`. A cursor past the end of every body is in the file's own
script frame, which is where a developer types in a file with no trailing newline. **After a single `:`
that follows a bare name, nothing**: `Name:` is half of `Name::` and nothing may be written between the
colons. **After `<?` in a run of markup, the two open tags and nothing else**
(`rule:statements/nvs-is-the-only-open-tag`), each replacing the bytes already typed. `<?` is not yet a
tag, so the lexer still reads it as markup, and it is the one place in markup where the developer is
writing Novis.

The markup half has a second side, on `nvs/regions`
(`rule:ide/a-template-region-gets-the-editors-services-and-formatter`): a half-written open tag and the one
character after it are cut out of the HTML region they sit in, so the client forwards nothing there and the
HTML service does not answer beside the two tags. The extra character is what keeps a cursor at the end of
the hole out of the region that follows, since a client reads a region as half-open. `<?xml` and every
other processing instruction stay markup's own.
