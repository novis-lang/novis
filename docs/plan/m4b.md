# M4B — Minimal `nvs-lsp`, syntax highlighting and the VS Code extension (~3 weeks)

**It was pulled ahead of M10 to get Novis into an editor early**, and it was placed behind the server
and the databases for the same reason it was
placed behind M4: completion, hover and diagnostics written against a language that cannot open a file
or reach a
database are written twice, and every `.lspt` case authored in the meantime is authored against a surface
about to change.

**One assertion in *Verify* re-anchors.** `hyper` and its five dependencies came in with the server and one of
them is `tokio`, so what `crates/nvs-runtime/tests/manifest_policy.rs` checks is the property that
assertion was reaching for: no manifest of ours names an async runtime, no crate of ours depends on one,
and the graph's single route to a scheduler crate compiles `sync` alone. The claim is about a *runtime*,
never about the `Future` trait; `rule:ide/one-grammar-one-tree`'s own bullet now says so.

Pulled ahead of M10 by `rule:ide/every-feature-is-staged-behind-its-dependency` so real-world
testing in an editor starts the moment M4 makes Novis a usable CLI language, rather than after M5–M9.
`rule:ide/one-grammar-one-tree` settles the four things that ADR left
open or specified against a parser that turned out to be shaped differently; where the two disagree, 0099
is the current rule and 0040's body has been folded to match.

**The tree.** `nvs-syntax` gains a **lossless parse result** — the same one grammar and the same AST, plus
a `trivia` vector (every comment and whitespace run, by span) and a `SyntaxIndex` answering byte offset →
innermost node and its ancestors. `tokens ⊕ trivia` reproduces the file byte-for-byte, which is the
losslessness `nvs fmt` needs at M10; recovery becomes explicit (`MemberName::Missing`, a span on
`ExprKind::Error`) so a consumer never has to infer "did the user write this name or did the parser invent
it at the cursor". There is **no second parser and no second tree**: `nvs check`/`nvs run` are that same
parse followed by "refuse if anything was reported", which is what they already do.

**The `trivia` half of that paragraph is built**:
`rule:tooling/doc-comment-is-three-slashes` needs a doc comment to survive
lexing, which is the same one edit to `skip_trivia`, so the `Trivia` vector
(`crates/nvs-syntax/src/token.rs:43`, read back at `crates/nvs-syntax/src/lexer.rs:171`), its
`TriviaKind` variants and the losslessness property `rule:ide/one-grammar-one-tree` specifies came with it. What is still
this milestone's, and has no consumer before it: the **`SyntaxIndex`**, and the explicit-recovery half —
`MemberName::Missing` and `ExprKind::Error`'s span — which completion needs and a doc comment does not.

**The server.** `crates/nvs-lsp` — `lsp-server` and `lsp-types`, **synchronous, no tokio** — speaking LSP
over stdio behind a `nvs lsp` subcommand. Nine requests and no more: `publishDiagnostics` (the existing
`nvs check` pipeline), `hover` (declared type, a `Core` member's registry signature, a declaration's own
doc comment out of the trivia layer), `definition`, `completion` (keywords, members off a resolved
receiver including `Core` classes, enum cases after `Type::`, in-scope variables — **no** workspace symbol
search, that needs M10's indexing), `semanticTokens/full`, `documentSymbol`, and three that are
projections of data the tree already holds rather than features built on it: `selectionRange` (the
`SyntaxIndex`'s ancestor list *is* the response), `foldingRange` and `documentLink`. Plus exactly **two
code actions**, admitted because their replacement text already sits in `Diagnostic::suggestions`: the
casing fix ([0029](../decisions/0029.md)/[0030](../decisions/0030.md))
and `(int)$x` → `$x as int` ([0034](../decisions/0034.md)). Beside the nine standard
requests sits exactly **one of Novis's own**, `nvs/redactions`, answering the ranges the editor conceals —
a literal or interpolation slot whose static type carries `secret`
([0101](../decisions/0101.md) §§ 1–2). It is
not a token modifier: that channel degrades to "whatever the theme thinks", and a security default may not
have becoming-visible as its failure mode. Its fail direction is named — a range whose expression cannot be
typed but whose binding declares `secret` is answered anyway, so a value does not flash on every keystroke
while its literal is being typed.

**Diagnostics are phase-gated**, which is the one rule an editor needs and a compiler does not: the front
end runs parse → resolve → types with no gate, so today one typo yields a *spurious* `E0301` reported
above the `E0102` that caused it. A file that produced a lexer or parser diagnostic publishes those and
its declaration diagnostics and suppresses `E03xx`/`E04xx` **for that file only** — presentation, not
analysis, and `nvs check` is untouched. Position encoding is negotiated per LSP 3.17; `nvs-diagnostics`
grows `utf16_col` beside its existing `char`-counting `line_col`, because position arithmetic has one
home. Document sync is `Full`, analysis is debounced and cancellable, editing one document re-analyses
every open document whose graph contains it, and the unit of analysis is one open document as its own
entry point with open buffers overlaid on its `require`/`autoload` graph. **Nothing but the protocol
writes to stdout** — stdio is the wire, and that invariant holds today only by accident, so it is held by
a test.

**Syntax highlighting, which is two layers and not one feature.** A **TextMate grammar** gives colour the
instant a file opens, before the server exists: the dual-mode `<?nvs`/`<?php`/`<?=`/`?>` openers with
inline HTML outside them, heredoc and nowdoc with interpolation only in the former, type annotations in
every slot the grammar allows one including `rule:types/object-top`'s inline shapes, the `tainted`/`secret` qualifiers
and `decimal`, Novis's own keywords (`spawn`, `spawn script`, `autoload`, `type`, `by`, property hooks),
`rule:types/duration-literal`'s duration literals, and `#[...]` attributes told apart from `#` comments — and **nothing Novis
rejects may be coloured as valid** (`===`, legacy casts, `|>`, the alternative colon syntax). Then
**semantic tokens** colour what a regex structurally cannot know: `defaultLibrary` on a `Core` class,
enum members, type aliases, and two modifiers of Novis's own — **`tainted` and `secret`** — so a qualified
value is visibly qualified at every use site, which is the cheapest teaching surface ADRs
[0024](../decisions/0024.md) and
[0033](../decisions/0033.md) have. **Novis ships no colours** — colour is
the user's theme's — so both layers ship *names*, every one of them from the standard TextMate vocabulary
or LSP's standard legend, because a theme styles only names it recognises and an invented scope renders as
unstyled body text. The two custom modifiers reach a theme through `semanticTokenScopes`, and the
extension contributes no colour-customization defaults at all. A **second grammar** colours `.nvst` and
`.lspt` themselves — sections, with Novis embedded in the four that hold a program (`--FILE--`,
`--FILE <path>--`, `--SKIPIF--`, `--CLEAN--`), PHP in `--ORACLE--`, and `--EXPECTF--`/`--EXPECTF-ERROR--`'s
`%` escapes as escapes; every other section is literal text with only its delimiter coloured. Which
sections exist and what each holds is [`crates/nvs-test`](../../crates/nvs-test/src/lib.rs)'s module doc,
not this paragraph. Nearly free, and the grammar whose audience is this project's own loop, currently
reading hundreds of those files as flat grey text. It is **colour only** — the server's unit of analysis is
one `.nvs` document and a `.nvst` case is not a program, so a `--FILE--` body gets no hover, completion,
diagnostics or semantic tokens; projecting sections into virtual documents is an M10 question, not a cheap
extension of this.

**The extension.** `editors/vscode`, a TypeScript package outside the Cargo workspace: `.nvs` registration
(**and not `.php`**), `language-configuration.json` — whose `wordPattern` must include `$`, or
double-clicking `$total` selects `total` — the two grammars above, `nvs lsp` spawning via
`vscode-languageclient` with a configurable path falling back to `PATH`, a `LanguageStatusItem` for server
health and version, `nvs run`/`nvs test` as Tasks **with a `problemMatcher`** over the diagnostic
renderer's existing format, so a failure is a clickable Problems-panel entry rather than terminal text,
and an AST explorer panel backed by **`nvs ast --json --resilient`** — which is built here, because
`--json` does not exist today and `{stmts:#?}` has no stability contract. The extension refuses a binary
whose version it does not understand rather than answering confusingly, and holds **no language logic**,
enforced by a dependency-allowlist test rather than by review. The setting and command identifiers
(`nvs.path`, `nvs.lsp.enable`, `nvs.lsp.debounce`, `nvs.lsp.trace.server`, `nvs.secrets.redact`,
`nvs.taint.mark`; `nvs.run`, `nvs.test`, `nvs.showAst`, `nvs.restartServer`, `nvs.revealSecret`,
`nvs.hideSecrets`) are frozen at M4B because they are public API — a rename breaks a user's
`settings.json` silently. **A `secret` literal is concealed by default** — blurred in place, the character
cells kept, so every edit still addresses the real text — and a reveal is per range and dies when the
editor closes, because the threat is an unattended screen and no API reports one
([0101](../decisions/0101.md)). `tainted`
is decorated only if the user asks (`nvs.taint.mark`, default `off`): a glyph is *added content*, and how
a construct looks stays the theme's call. The AST panel inherits the same placeholder in
`nvs ast --json` itself, or it prints in a webview the credential the buffer behind it is hiding. Extension id `nvs-lang.nvs`; `package-lock.json` is committed because
`npm ci` needs it; CI produces an installable `.vsix`; **nothing is published** —
`rule:ide/one-server-two-thin-clients` *Revisiting* keeps that open.

**How editor behaviour is checked.** A **`.lspt` case** is the sibling of `.nvst`: the same section lexer,
a `<|>` cursor, a `--REQUEST--` line and a frozen canonical `--EXPECT--` rendering, run by **`nvs
lsp-test`** printing the same `N passed, M failed` line — so the loop gates editor behaviour through the
check kind it already has. Coverage is **inferred** from the node each cursor resolved to, never declared,
and `every_request_answers_every_construct` fails naming each empty cell of the request × construct
matrix. Extension tests run in two tiers: headless Node every iteration (grammar snapshots via
`vscode-textmate`, a contributions/allowlist test, a protocol round-trip against the real binary), and
`@vscode/test-electron` in the real extension host once per green tree.

**Not here:** format-on-save (`nvs fmt` is M10 — `rule:tooling/fmt-is-one-canonical-style`),
rename, extract refactorings, workspace symbol search, inlay hints, signature help, `documentHighlight`,
the Test Explorer, profiler visualization, debugger UI, and any code action whose fix a diagnostic does
not already compute. The last two of those look adjacent to what M4B does build and are not:
`documentHighlight` needs resolution applied to *every* occurrence, a different walk from resolving one,
and inlay hints encode idioms still moving through M5–M9. No PhpStorm work at all — PhpStorm stays
entirely at M10 per `rule:ide/one-server-two-thin-clients`.

**Verify:** concatenating tokens and trivia in offset order reproduces every file in `examples/` and `tests/`
byte-for-byte. Parsing every prefix of every `examples/*.nvs` at a token
boundary panics on none, answers a `SyntaxIndex` lookup at the final offset on all, and reports a
diagnostic on each prefix that is genuinely incomplete; a fuzz target over truncated and mid-edit inputs
finds no panic in five minutes. `nvs lsp-test tests/lsp/` reports `0 failed` and the coverage matrix has
no empty cell — including a case per request proving that an unclosed brace or a trailing `->` does not
stop diagnostics, hover, completion or semantic tokens working on the well-formed code around it, which is
`rule:ide/every-feature-is-staged-behind-its-dependency`'s core claim. A full re-analysis of a 1,000-line document stays under the named latency bound —
the measurement that says giving up incremental reparse still pays. The grammar snapshot assigns the
expected scope to every construct listed above, `#[Route]` included and `===` receiving no operator scope,
and every scope it emits is on the standard-name allowlist. The gate of *Diagnostics are phase-gated* is a
case in both directions — gated, `E0102` and not `E0301`; with `phase=all`, both. A position round-trips
through both encodings on a multi-byte line, a BOM document answers correct offsets, and a CRLF document's
columns match an LF one's; `println!` appears in no crate the server links; editing a required file
re-publishes the requiring document's diagnostics untouched. The extension activates on `.nvs` and not on
`.php`, shows TextMate colour before the server answers and semantic colour after, registers a legend
equal to the one the server declares, round-trips all nine standard requests and both code actions with no
language logic in its own source, conceals a `secret` literal on open and reveals exactly the one range
`nvs.revealSecret` is fired on while a second secret in the same file stays hidden, decorates nothing for
`tainted` at the default setting, prints the placeholder rather than the literal in `nvs ast --json`, exposes `nvs run`/`nvs test` as Tasks whose failures populate the Problems panel,
selects `$total` whole on a double-click, opens a `.nvst` case coloured, and renders the AST panel for a
file that does not compile. No manifest of ours names an async runtime and no crate of ours depends on
one, the lock file's only scheduler crate being `hyper`'s `tokio` compiled as `sync` alone.
