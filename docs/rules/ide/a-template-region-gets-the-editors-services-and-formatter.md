The extension forwards requests inside an inline-HTML region to VS Code's built-in HTML, CSS and
JavaScript language services, so the half of a `.nvs` file that is markup gets what those services
answer as providers: completion, hover, the colour picker and the linked ranges that rename a tag.
Since `rule:programs/first-party-framework` makes inline HTML the template engine, that region is where
a web application's markup is written, not an edge case.

**Emmet expansion and HTML validation are not among them.** Neither is a provider a request can be
forwarded to: Emmet expands from the language of the document the cursor is in, and the HTML service
publishes validation only for documents opened as HTML. Emmet would take `emmet.includeLanguages`
mapping `nvs` to `html`, which turns it on in the Novis half as well, so the extension contributes
neither; a user who wants Emmet in a template sets that mapping themselves.

**An html template's body is a region too.** ``html`…` `` (`rule:core-classes/html-template`) is markup
written in expression position rather than at file scope, so it gets the same services on the same
terms — the holes are Novis and the segments are HTML, which is the boundary the lexer already knows.

**The region list comes from the server**, as one request of Novis's own, `nvs/regions`, beside
`nvs/redactions`. The lexer already knows where a mode ends; the client does not re-derive it from a
grammar, for the reason `rule:ide/redaction-ranges-come-from-the-server` gives for redaction
ranges — a client that guesses is a second implementation of the lexer. Forwarding a request to a service
the extension did not write is not language logic in the client. The request carries an optional `text`,
and then answers for that text rather than the open buffer, because a format pass needs the regions of
`nvs fmt`'s output, which no buffer holds yet. The answer's shape stays a range and a language. Its cost is protocol surface: it is a second
request of Novis's own, and one more a client other than VS Code implements to reach the same depth.

**Formatting is `nvs fmt` first, then the editor's own HTML formatter over the markup, starting where the
Novis code is.** A format request runs `nvs fmt` over the whole file, asks `nvs/regions` for the regions
of the result, and hands the editor's HTML formatter one chunk at a time: the markup between a `?>` that
ends its line and the `<?nvs` that reopens code, `<?= … ?>` holes included. A chunk's lines start at the
indentation of its `?>` line, which `rule:tooling/fmt-novis-constructs` puts at the depth of its block,
so markup nests from the Novis code around it and the file does not jump between the two; the line
holding the closing `<?nvs` starts there too. Nesting inside a chunk is the HTML formatter's, in
`nvs fmt`'s four-space unit whatever the editor's own `tabSize` is, because a chunk and the code around
it have to agree on what one level is. The markup before a file's first open tag is a chunk based at
column zero. A hole's bytes are Novis's and are never edited: the formatter is shown a stand-in run of the
same length, and a chunk whose formatting would change a hole is left as written, the safe direction
because an unformatted chunk is a layout complaint and an edited hole is a changed program. A file
`nvs fmt` refuses is left as written and nothing after it runs, and a regions request nothing answers
leaves every chunk as `nvs fmt` wrote it.

**`nvs fmt` stays the only formatter of Novis and never touches markup**, so `nvs fmt --check` passes over
a file this pass formatted. What is given up is a canonical layout for markup: it reads the user's
`html.format.*` settings, and it re-indents bytes a program prints — which a browser ignores and a
program printing text through inline HTML does not.

Each half is turned off by a setting of its own, both of them `true` by default: `nvs.template.services`
off stops the forwarding, `nvs.template.format` off stops the markup pass. A user with their own HTML
tooling, or with markup whose whitespace is output, has to be able to get out of the way of ours. With
the second off, format-on-save is `nvs fmt` and nothing else. The markup pass is on by default because
markup is where a web application is written, and a formatter that skips it is one developers replace
with nothing.

- **Memory:** nothing on the request path and nothing held by the server. Each format request costs one
  `nvs fmt` process, one regions round trip and one HTML format per chunk, all in the editor's process.

**Declined:**
- **The markup pass off by default.** It keeps every file's bytes canonical, and costs the layout of the
  half of a template a web application is mostly written in; the setting keeps that answer one click away.
- **Markup formatting inside `nvs fmt`.** It would be canonical, checkable in CI and identical in every
  editor, but it is an HTML formatter to build and own, every change to it is a diff across every
  template, and a Novis formatter that re-indents markup changes what a program prints.
- **One HTML format over the whole joined document**, the virtual document the services are shown. It
  nests markup by HTML depth alone, so a chunk inside a `foreach` sits at its siblings' column, which is
  the jump between the two halves the chunked pass removes.
- **The client lexing `nvs fmt`'s output, or `nvs fmt` printing the regions.** The first is the second
  lexer the server's region list prevents; the second is a flag that is not an I/O mode
  (`rule:tooling/fmt-is-one-canonical-style`).
