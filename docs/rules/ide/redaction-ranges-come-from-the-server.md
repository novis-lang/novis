A `secret` value is concealed in the editor by default. The language server answers a list of ranges
and their kinds; the client draws them and knows nothing about what a secret is, because the
alternative is the client guessing.

**The server can be exact where a client cannot.** A tool that hides secrets in an editor otherwise
guesses, by filename, by pattern or by entropy. `secret` is a type-checker fact, and it propagates
through concatenation and interpolation (`rule:security/secret-propagation`), so a value derived from a
credential is concealed for the same reason the credential is, with no heuristic involved. A guess would
miss every value that became `secret` by propagation, which is most of them.

The threat this closes is an **incidental viewer**: a stream, a screen share, a screenshot, somebody
looking over a shoulder. It is not a defence against the machine's own user and it is not secret
management.

**This is a rendering, and it is not what makes a `secret` safe.** The language's guarantee is that
every sink refuses one (`rule:security/secret-sinks-refuse`), checked by the compiler, with no editor
involved and nothing an editor could weaken. Concealment covers the one exposure a compiler cannot
reach — a value legible on a screen somebody else is looking at — and it covers nothing else. An
editor drawing no decoration at all is not running a less safe language; it is only less discreet,
which is why every rule in this section is the editor's and none of them is the language's.

The test that puts a rule here rather than under `security/` is whether it still holds with no editor
running. Every sink refusing a `secret` does, and so do the AST dump's placeholder
(`rule:security/redaction-reaches-the-tools-own-renderings`) and the test runner's redacted diff
(`rule:security/secret-in-a-test-report`). A blur over a string literal does not. The security chapter
is read as what the language will not let a program get wrong, and a rendering filed there would invite
a reader to count it as part of that guarantee.

It does **not** ride the semantic-token channel, even though the qualifier already travels there. That
channel's contract is *names a theme styles*, and its correct degradation is to fall back to the
underlying token type — which a security default whose failure mode is *the value becomes visible*
cannot inherit. Two mechanisms, two contracts, and neither can silently disable the other.

The fail direction is named, because mid-edit is exactly when the type is unknown. A range whose
expression cannot be typed, but whose **binding's declared type carries `secret`**, is redacted
anyway; the client **holds its last answer** and never clears decorations on an error, a cancellation
or a restart. An empty answer means nothing to redact; a *missing* answer means nothing at all.

Redacting on the declared type when the expression's own type is unknown is an over-approximation, and
it is kept because the other failure, the value flashing visible on every keystroke inside a string being
typed, is the one that loses the credential. Where the server cannot attribute a range at all, it answers
nothing for it and the value is visible. That gap is the honest one: over-redacting a document that does
not parse would conceal arbitrary spans of a file somebody is trying to fix.

The cost is a second decoration mechanism beside the semantic-token one, with two contracts to keep
apart and two ways for one token to be described.

- **Memory:** none on the request path. The server answers a range list per open document, and the
  client keeps the last answer for each.

**Declined:**

- **Detecting the range in the client** with a pattern or a filename rule. It puts language logic in a
  client (`rule:ide/one-server-two-thin-clients`), and it misses every value that is `secret` by
  propagation.
- **A third semantic-token modifier.** That channel degrades to the underlying token type when a theme
  has no opinion, and a security default may not have un-redacting as its degradation.
- **Serving a redacted document** through a content provider or a custom editor. The file stops being an
  editable text document, with no language-server edits, no diff, no version-control integration and no
  format-on-save, which is a far larger loss than the exposure it closes; and it substitutes the buffer,
  which `rule:ide/redaction-covers-bytes-only` rules out.
- **A chapter of its own for security in the editor.** It splits the editor across two homes, and every
  rule in each would need a pointer to the other. The editor's chapter is the one home for the editor,
  and its security rules sit together at its end.
