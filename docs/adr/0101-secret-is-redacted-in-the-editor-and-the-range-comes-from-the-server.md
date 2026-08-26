# ADR 0101 — A `secret` value is concealed in the editor by default, the range comes from the server, and `tainted` gets no default decoration at all

- **Status:** Accepted
- **Date:** 2026-08-26
- **Scope:** what the VS Code extension does with [ADR 0024](0024-taint-tracking-for-injection-sinks.md)'s
  `tainted` and [ADR 0033](0033-secret-qualifier-for-confidential-values.md)'s `secret` beyond colour — the
  `mwl/redactions` request that carries the ranges, which spans are concealed and which are deliberately
  not, how a reveal works and how long it lasts, the extension's own surfaces that must inherit the
  redaction, and the leak surfaces VS Code gives no way to close. It does **not** decide colour or token
  names ([0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 4 owns both, unchanged), the qualifier
  semantics ([0024](0024-taint-tracking-for-injection-sinks.md),
  [0033](0033-secret-qualifier-for-confidential-values.md)), the diagnostic record's own redaction
  ([0092](0092-one-diagnostic-record-three-renderings.md) § 1), or anything for PhpStorm
  ([0016](0016-ide-integration.md) § 3).
- **Amends:** [0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 3 — the M4B request set gains
  `mwl/redactions`, the one MWL-specific request on it. § 6 — the frozen contribution roster gains two
  settings and two commands, added and never renamed, per that section's own rule; and the AST panel of
  § 7 acquires an obligation it did not have. § 4 is **not** amended: its two token modifiers, its
  no-colours rule and its "not MWL's call" sentence are what *Decision § 4* below applies rather than
  changes. [0016](0016-ide-integration.md) § 2 — the VS Code roster gains one bullet naming this ADR, and
  its verification line gains the extension-host case.
- **Relates to:** 0004, 0016, 0024, 0033, 0040, 0092, 0099

> **In short:** a `secret` value's *bytes* are concealed in the editor by default — blurred in place, not
> deleted, not folded — because the threat this closes is an incidental viewer: a stream, a screen share, a
> screenshot, a shoulder. The ranges are computed by `mwl-lsp`, which already knows `is_secret(ty)`, and
> handed over by one new request, `mwl/redactions`; the client draws a `TextEditorDecorationType` and holds
> no language logic, per [ADR 0016](0016-ide-integration.md) § 1. Only a **literal token or an interpolation
> slot** whose static type carries `secret` is concealed — never the identifier that binds it, because
> blurring `$apiKey` conceals nothing and costs all the readability. Reveal is explicit, per range, and does
> **not** persist across a close: the whole model assumes an unattended screen. `tainted` gets the opposite
> answer — **no default decoration whatever.** [ADR 0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md)
> § 4 already gives it a semantic-token modifier a theme may style, and that section's rule that MWL does not
> decide how a construct looks in someone else's editor is binding here; a marker glyph is *added content*,
> so it ships opt-in (`mwl.taint.mark`, default `off`). The asymmetry is the point and is not a preference:
> a credential on a stream is a security incident under this project's priority 1, and a tainted value on a
> screen is not a security event at all. What this ADR cannot do is stated as loudly as what it can —
> concealment is cosmetic, the bytes stay in the buffer, and Search, Quick Open, the diff and SCM views, the
> clipboard and the file on disk all still hold the plaintext.

## Context

- The editor is the one place a `secret` value is displayed in full by design. Every *program* sink
  [ADR 0033](0033-secret-qualifier-for-confidential-values.md) § 4 names is already closed at compile time
  and [ADR 0092](0092-one-diagnostic-record-three-renderings.md) § 1 already makes the redaction a node kind
  that all three diagnostic renderings inherit — so the remaining exposure is not a program behaviour at
  all. It is a person's screen while they are being watched.
- **MWL can be exact here where nothing else can.** Every shipping tool that hides secrets in an editor
  guesses — by filename (`.env`), by regex, by entropy. `mwl-lsp` does not have to guess: `secret` is a
  type-checker fact, computed by `mwl_types::expr::quals::is_secret`, and it propagates through
  concatenation and interpolation under [ADR 0033](0033-secret-qualifier-for-confidential-values.md) § 2. A
  value derived from a credential is concealed for the same reason the credential is, with no heuristic in
  the loop. This is the whole reason the feature is worth building rather than telling users to install one
  of the guessing extensions.
- **Propagation is also what makes the naive version unusable**, and it is why *Decision § 2* is narrow.
  `secret` and `tainted` both poison, and every `Core\Request` accessor returns `tainted` — so in a request
  handler nearly every string-typed expression carries the qualifier. Decorating each one is not a security
  feature, it is a wall of glyphs a user turns off, taking the redaction with it.
- [ADR 0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 4 already decided that MWL ships *names*
  and no colours, that the two custom modifiers map to standard TextMate scopes a theme already styles, and
  that a theme with no opinion must degrade to the underlying token type. Concealment cannot ride that
  channel: a theme that declines to style `secret` would silently un-redact, which is the one failure
  direction a security default may not have.
- The threat model is bounded and saying so is part of the decision. This defends against an **incidental
  viewer** of a screen. It is not a defence against the machine's own user, it is not secret management, and
  it does not make a hardcoded credential safe — that credential is in the working tree and in `git log`
  whatever the editor draws. See *Revisiting*.

## Decision

### 1. The server computes the ranges; the client draws them and knows nothing

`mwl-lsp` gains one request, `mwl/redactions`, joining the M4B set in
[ADR 0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 3. It takes a `TextDocumentIdentifier` and
answers a list of `{range, kind}`, `kind` being `secretLiteral` today and an open string for whatever a
later qualifier needs. It is the **only** MWL-specific request in that set, and it exists because the
alternative is the client deciding what a secret is — which [ADR 0016](0016-ide-integration.md) § 1 forbids
and which is exactly the guessing *Context* rejects.

**It does not ride the semantic-token channel**, even though the `secret` modifier already travels there.
That channel's contract is names a theme styles, and its correct degradation is "fall back to the
underlying token type"; a security default whose failure mode is *the value becomes visible* cannot inherit
that. Two mechanisms, two contracts, and neither can silently disable the other.

**The fail direction is named, because mid-edit is exactly when the type is unknown.** A document being
typed into is a document that does not check, so:

- A range whose expression cannot be typed, but whose **binding's declared type carries `secret`**, is
  answered as redacted anyway. The declaration is stable while its initializer is being written, which is
  the common case and the one that would otherwise flash the value on every keystroke.
- The client **holds its last answer** until a new one arrives and never clears decorations on an error
  response, a cancelled request, or a server restart. An empty answer means "nothing to redact here"; a
  *missing* answer means nothing at all and must not be read as one.
- Where the server genuinely cannot attribute a range it answers nothing for it, and the value is visible.
  That is a real gap and it is the honest one: over-redacting a document that does not parse would conceal
  arbitrary spans of a file someone is trying to fix.

### 2. Only the bytes are concealed — a literal token or an interpolation slot, never an identifier

A range is answered when it is a **string literal, heredoc body, or `bytes` literal token** whose static
type carries `secret`, or an **interpolation slot** inside one whose interpolated expression does. Nothing
else. In particular:

- **Never an identifier.** `$apiKey` and `Config::$token` are names, not secrets; concealing them hides no
  bytes and makes the file unreadable for the developer whose editor it is.
- **Never a type annotation.** `secret string` is the declaration doing its job and must stay legible — it
  is how a reader knows the concealment below it is deliberate rather than a rendering fault.
- **Never a whole line, statement or block.** [ADR 0040](0040-vscode-deep-tooling-and-resilient-parsing.md)'s
  folding ranges are line-granular and structurally cannot express `secret string $k = "…";`, which is the
  shape this exists for; see *Alternatives rejected*.

The concealment is a `TextEditorDecorationType` applied over the range — a blur, with the character cells
kept — so the cursor, selection and every LSP edit still address the real text. It is a rendering, not an
edit and not a document substitution: the buffer VS Code holds is the file on disk, byte for byte.

### 3. Reveal is explicit, per range, and does not survive the editor closing

`mwl.secrets.redact` (default `true`) is the setting; `mwl.revealSecret` reveals the range at the cursor
and `mwl.hideSecrets` re-conceals every revealed range in the window. A revealed range is also offered as a
command link in the decoration's own hover, which is the discoverable path.

**A reveal is window-local, per range, and is dropped when the editor for that document closes.** It is not
written to workspace state and it does not survive a reload. The threat model is an unattended screen, so a
reveal that outlives the moment it was needed is the same as no redaction at all — and a user who reveals
one credential to read it has not consented to reveal every credential in the workspace for the rest of the
week.

There is **no automatic reveal and no automatic re-conceal on a signal**, because there is no signal: no VS
Code API reports that the window is being shared, recorded or projected. Anything that looked like one
would be a guess with a security failure attached, so redaction is unconditional by default and the user is
the only thing that turns it off.

### 4. `tainted` ships no default decoration; the marker is opt-in

[ADR 0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 4 gives `tainted` a semantic-token modifier
mapped to a standard scope, and rules that how a construct looks is the user's theme's to decide. That rule
is applied here, not amended: a marker **glyph is added content**, not a colour, and shipping one on by
default is MWL writing into someone else's editor exactly what that section refuses.

So `mwl.taint.mark` is a setting with three values and `off` is the default:

| Value | What is marked |
|---|---|
| `off` | nothing — the token modifier alone, styled by the theme (default) |
| `declaration` | a glyph after each declaration whose type carries `tainted` |
| `sink` | `declaration`, plus each argument position where a `tainted` value reaches a member classified as a sink under [ADR 0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 1 |

The glyph is a **themed codicon**, not an emoji: `contentText` renders an emoji at the mercy of the host's
installed fonts, and a security-adjacent marker that renders as a replacement box on one platform is worse
than none. Its colour is a `ThemeColor` reference, never a literal — the same reason § 4 gives.

**The asymmetry with `secret` is deliberate and is this section's whole content.** A credential on a shared
screen is a security incident under [AGENTS.md](../../AGENTS.md)'s priority 1, which is what buys the
default. A tainted value on a screen is not an event at all — `tainted` is a *compile-time* guarantee that
is already enforced by refusing the sink, so the editor marking it is teaching, and teaching does not get to
override the user's theme.

### 5. The extension's own surfaces inherit the redaction, and one of them does not today

Two of the extension's own renderings would otherwise print the plaintext the editor just concealed:

- **The diagnostics channel already inherits it.**
  [ADR 0092](0092-one-diagnostic-record-three-renderings.md) § 1 makes the redaction a node kind in the
  diagnostic record, so every rendering — the Problems panel included — carries the placeholder from one
  place. Nothing is owed here; it is stated so a reader does not go looking for a second fix.
- **The AST panel does not, and this ADR gives it the obligation.**
  [ADR 0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 7's `mwl ast --json` schema includes each
  node's own scalar fields, which for a string literal is its text — so the panel would render a secret
  the buffer behind it is blurring. A literal node whose static type carries `secret` emits the same fixed
  placeholder [ADR 0033](0033-secret-qualifier-for-confidential-values.md) § 4 gives a dumped property, in
  the JSON itself rather than in the panel, so the CLI's `--json` and the webview cannot disagree.

### 6. PhpStorm draws nothing at this milestone

`mwl/redactions` is a request on the one server both clients drive, so the PhpStorm plugin can answer it
whenever it is built. It does not at M4B: the plugin is M10 under
[ADR 0016](0016-ide-integration.md) § 3, and an editor-side decoration API is per-editor work with no shared
half. Named here so the gap is a decision rather than something discovered when someone opens a `.mwl` file
in PhpStorm on a call.

### 7. What this cannot close, stated because a redaction trusted past its reach is worse than none

All of these leak the plaintext of a value the editor is concealing, and none has an API that would let the
extension close it:

- **Search results** (`Ctrl+Shift+F`) and **Quick Open**'s peek render matching lines outside any
  `TextEditor`, so no decoration applies.
- **Diff and SCM views** are `TextEditor`s and *can* be decorated, but the server has no type information
  for the "before" side of a diff — so the old value of an edited secret is visible in the diff that reviews
  the edit.
- **The minimap** renders from the buffer, not from decorations.
- **Any other extension's hover, CodeLens or webview** reads the document text directly.
- **The clipboard.** A copy of a concealed range copies the plaintext; nothing may intercept it — see
  *Alternatives rejected*.
- **The file itself.** It is on disk, in the working tree, and in `git log` the moment it is committed. The
  editor concealing a hardcoded credential does not make it less hardcoded, which is *Revisiting*'s first
  entry.

This list is part of the decision, not commentary on it: the extension's own documentation states it, so the
feature is not sold as something it is not.

## Consequences

**Positive**

- The one exposure [ADR 0033](0033-secret-qualifier-for-confidential-values.md) left open — a developer's
  own screen — is closed by default, with an exactness no filename or entropy heuristic can reach, because
  the qualifier is already a checked fact and already propagates.
- The qualifier becomes visible where it is learned. A developer sees concatenation carry `secret` into a
  derived value, in the editor, at the moment they write it — the same argument
  [ADR 0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 4 makes for the token modifiers, extended
  to the one axis a colour cannot express.
- The client stays free of language logic, so the allowlist test
  [ADR 0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 6 already runs keeps holding: the
  redaction is a range list from the server and a decoration, and there is nothing in it a parser would help
  with.
- One request serves both editors, so PhpStorm's eventual version costs its decoration API and nothing else.

**Negative**

- **The concealment is cosmetic and the list in *Decision § 7* is long.** A user who believes the editor is
  protecting them everywhere is worse off than one who knows it is not, which is why that list ships in the
  extension's own README and not only here. This is the real cost of the feature and it does not go away.
- **The AST panel gains a redaction rule**, which means `mwl ast --json`'s frozen schema now has a
  type-dependent field value — the first place that CLI's output depends on anything past the parse. Its
  snapshot test gains a case, and a reader of `--json` can no longer assume a literal node's text is the
  source text.
- **A second decoration mechanism sits beside the semantic-token one**, with two contracts to keep straight
  and two ways for a token to be described. *Decision § 1* argues the separation is required rather than
  incidental, but the extension does carry both.
- **The mid-edit rule in *Decision § 1* is a heuristic in a place this project usually refuses one.** A
  range is redacted on its binding's declared type when its own type is unknown, which is an
  over-approximation; it is chosen because the alternative failure — the value flashing visible on every
  keystroke inside a string being typed — is the one that loses the credential.
- **`mwl.taint.mark` is a setting nobody may ever change from `off`**, which is a contribution that costs
  documentation and a test for a feature that ships inert. Accepted because *Decision § 4*'s alternative is
  overriding a user's theme by default, and because the `sink` value has real teaching worth once
  [ADR 0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md)'s classification exists on the
  `mwl-stdlib` member rows, which it does not yet.

## Alternatives rejected

- **Detect the range in the client with a regex or a filename rule**, the way every shipping secret-hiding
  extension does. Rejected twice over: [ADR 0016](0016-ide-integration.md) § 1 forbids language logic in a
  client, and the heuristic would be strictly worse than the checked fact the server already holds — it
  would miss every value that became `secret` by propagation, which is most of them.
- **Carry the redaction as a third semantic-token modifier.** Rejected in *Decision § 1*: that channel
  degrades to "fall back to the underlying token type" when a theme has no opinion, and a security default
  may not have un-redacting as its degradation.
- **Fold the value away with a `FoldingRangeProvider`.** Rejected: folding is line-granular, so it cannot
  express a literal inside a one-line declaration, which is the shape the request came from. It would also
  hide the declaration itself, which *Decision § 2* keeps legible on purpose.
- **Serve a redacted document through a `TextDocumentContentProvider` or a custom editor.** Rejected: it
  stops being an editable text document — no LSP edits, no diff, no git integration, no format-on-save —
  which is a far larger loss than the exposure it closes, and it substitutes the buffer, which
  *Decision § 2* rules out.
- **`display: none` on the concealed span.** Rejected in favour of a blur: removing the box breaks cursor
  and selection arithmetic across the range, and the goal is a text editor that still behaves like one.
- **Ship the `tainted` marker on by default** (the biohazard glyph the request was framed around).
  Rejected under *Decision § 4*: it overrides the user's theme for a construct that is not a security event,
  and *Context* names the density problem — most string expressions in a request handler are `tainted`, so
  the default would be a wall of glyphs and the setting that turns it off would take the `secret` redaction
  with it in the user's mind.
- **Intercept copy** by rebinding `editor.action.clipboardCopyAction` with a `when` clause. Rejected: it
  overrides a global keybinding for the entire editor to protect a value that is on disk anyway, and a
  redaction that fights the user's own clipboard is past the threat model in *Context*.
- **Infer that the screen is being shared** and redact only then. Rejected: no such API exists, so this
  would be a guess whose failure is a leaked credential.

## Revisiting

- **Whether `mwl check` should refuse, or warn about, a `secret`-typed *source literal*.** The request this
  ADR answers began with a hardcoded API key, and concealing one in the editor treats the symptom — the
  credential is in the working tree and in every clone. The checker already knows both facts it would need
  (the type is `secret`, the expression is a literal token), so the rule is cheap; what is not settled is
  whether it is right, since a test fixture, an example and a `.mwlt` case all legitimately spell one. Due
  with whatever milestone designs `Core`'s credential-handling surface, alongside
  [ADR 0033](0033-secret-qualifier-for-confidential-values.md) *Revisiting*'s own open `Core\Secret` roster.
- **The PhpStorm side of *Decision § 6***, when that plugin is built at M10.
- **Whether `mwl.taint.mark`'s `sink` value earns its place**, once
  [ADR 0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md)'s classification exists on the
  `mwl-stdlib` member rows. If it does not, the setting narrows to two values rather than growing a third.
- **Whether the diff view's "before" side can be redacted at all** — it needs a type answer for a document
  revision the server never analysed, which is a different question from anything in the M4B request set and
  may simply not be worth its cost.

Verification, in the order it becomes possible (M4B, on
[ADR 0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 8's two tiers):

- **Headless, `.lspt`**: `mwl/redactions` over a document holding a `secret string` literal, a `secret`
  interpolation slot, a plain `string` literal and a `tainted string` literal answers exactly the first two
  ranges and neither of the last two; the identifier and the `secret string` annotation are absent from the
  answer, per *Decision § 2*.
- **Headless, `.lspt`**: the same request over a document that does **not** parse still answers the range of
  a literal whose binding declares `secret`, per *Decision § 1*'s fail direction — the case that would
  otherwise flash the value on every keystroke.
- **Headless**: the contributions test asserts `mwl.secrets.redact`, `mwl.taint.mark`, `mwl.revealSecret`
  and `mwl.hideSecrets` are declared with the names frozen here, and that the extension's dependency
  allowlist is unchanged by this feature.
- **Headless**: `mwl ast --json` over a file with a `secret` literal emits the placeholder rather than the
  literal's text, frozen in the same snapshot test
  [ADR 0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 7 already runs over `examples/`.
- **Extension host, once per green tree**: opening the file decorates the secret range; `mwl.revealSecret`
  at the cursor clears exactly that range and leaves a second secret in the same file concealed; closing and
  reopening the editor re-conceals the revealed one; `mwl.taint.mark` at its default decorates nothing.
