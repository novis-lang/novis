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

**It is a second way a file may begin, and not a second spelling of one.** `<?nvs` separates code
from the template text around it, and a shebang file has no template text: the marker the operating
system already requires selects a file kind, a program rather than a template. Every scripting
engine strips a leading `#!` line, and PHP does too, yet PHP still needs `<?php` on line 2. Python,
Ruby, Perl, Node and `sh` are in code mode from byte 0. The cost is one sentence of language surface,
and there is no runtime, latency or memory consequence.

**Declined:**
- **`<?nvs` still required after a shebang, as PHP requires `<?php`.** One file opening, one rule,
  but it charges every CLI script a disambiguation that cannot arise. PHP pays it for files older
  than its command-line mode, and Novis has none.
- **A second extension, `.nvss`, in code mode from byte 0.** The shebang has to be in the file
  anyway, so an extension is a second thing to get right, and it splits tooling, editor
  associations and `require` resolution for no gain.
- **Every file in code mode by default, with an opt-in for template mode.** The cleanest design
  from the command line, declined on scope: it overturns inline HTML and `<?=` to save one line in
  a CLI script. It is the alternative to reopen if the template half of the language is ever
  reconsidered on its own terms.
