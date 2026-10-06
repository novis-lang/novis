```
#!/usr/bin/env nvs
Core\Cli::write("hello\n");
```

**The trigger is exact:** the bytes `#!` at offset 0. Line 1, up to and including its first `\n`, is
**trivia** — not a token, not emitted, preserved by the formatter and seen by an editor as a comment.
**The file then continues in code mode**, exactly as if `<?nvs` stood there. Nothing else changes:
`?>` still switches to text mode and writes literal bytes to standard output, and a later `<?nvs`
reopens code mode (`rule:statements/nvs-is-the-only-open-tag`).

**`#!` anywhere but offset 0 is ordinary text**, in either mode, with no lookahead and no special
case. A byte-order mark before it therefore defeats the shebang and the file has none — left as-is
rather than repaired, because inventing one rule for one
marker is how a parser acquires the heuristics `rule:errors/ambiguous-input-refused` forbids.

**An `<?nvs` in a shebang file, before any `?>`, is `E0009`** — "this file opens with `#!` and is
already in code mode; remove the `<?nvs`" — rather than a lex error naming something the author did
not write. The reverse, a shebang file that never wanted code mode, is not a shape anyone writes and
gets no rule.

The line is trivia on every platform, so one file runs as `./app` on Unix and as `nvs app.nvs` on
Windows with no edit; Windows gains no kernel shebang support, and its distribution answer is the
single-file executable. This is one lexer branch at offset 0: no parser rule, HIR shape or runtime
behaviour changes.
