# M4B — Minimal `mwl-lsp`, syntax highlighting and the VS Code extension (~3 weeks)

Pulled ahead of M10 by [ADR 0040](../adr/0040-vscode-deep-tooling-and-resilient-parsing.md) so real-world
testing in an editor starts the moment M4 makes MWL a usable CLI language, rather than after M5–M9.
[ADR 0099](../adr/0099-the-resilient-tree-is-the-ast-plus-trivia.md) settles the four things that ADR left
open or specified against a parser that turned out to be shaped differently; where the two disagree, 0099
is the current rule and 0040's body has been folded to match.

**The tree.** `mwl-syntax` gains a **lossless parse result** — the same one grammar and the same AST, plus
a `trivia` vector (every comment and whitespace run, by span) and a `SyntaxIndex` answering byte offset →
innermost node and its ancestors. `tokens ⊕ trivia` reproduces the file byte-for-byte, which is the
losslessness `mwl fmt` needs at M10; recovery becomes explicit (`MemberName::Missing`, a span on
`ExprKind::Error`) so a consumer never has to infer "did the user write this name or did the parser invent
it at the cursor". There is **no second parser and no second tree**: `mwl check`/`mwl run` are that same
parse followed by "refuse if anything was reported", which is what they already do.

**The server.** `crates/mwl-lsp` — `lsp-server` and `lsp-types`, **synchronous, no tokio** — speaking LSP
over stdio behind a `mwl lsp` subcommand. Nine requests and no more: `publishDiagnostics` (the existing
`mwl check` pipeline), `hover` (declared type, a `Core` member's registry signature, a declaration's own
doc comment out of the trivia layer), `definition`, `completion` (keywords, members off a resolved
receiver including `Core` classes, enum cases after `Type::`, in-scope variables — **no** workspace symbol
search, that needs M10's indexing), `semanticTokens/full`, `documentSymbol`, and three that are
projections of data the tree already holds rather than features built on it: `selectionRange` (the
`SyntaxIndex`'s ancestor list *is* the response), `foldingRange` and `documentLink`. Plus exactly **two
code actions**, admitted because their replacement text already sits in `Diagnostic::suggestions`: the
casing fix ([0029](../adr/0029-identifier-casing-is-checked.md)/[0030](../adr/0030-no-leading-underscores-constructor-spelling.md))
and `(int)$x` → `$x as int` ([0034](../adr/0034-legacy-cast-syntax-rejected.md)).

**Diagnostics are phase-gated**, which is the one rule an editor needs and a compiler does not: the front
end runs parse → resolve → types with no gate, so today one typo yields a *spurious* `E0301` reported
above the `E0102` that caused it. A file that produced a lexer or parser diagnostic publishes those and
its declaration diagnostics and suppresses `E03xx`/`E04xx` **for that file only** — presentation, not
analysis, and `mwl check` is untouched. Position encoding is negotiated per LSP 3.17; `mwl-diagnostics`
grows `utf16_col` beside its existing `char`-counting `line_col`, because position arithmetic has one
home. Document sync is `Full`, analysis is debounced and cancellable, editing one document re-analyses
every open document whose graph contains it, and the unit of analysis is one open document as its own
entry point with open buffers overlaid on its `require`/`autoload` graph. **Nothing but the protocol
writes to stdout** — stdio is the wire, and that invariant holds today only by accident, so it is held by
a test.

**Syntax highlighting, which is two layers and not one feature.** A **TextMate grammar** gives colour the
instant a file opens, before the server exists: the dual-mode `<?mwl`/`<?php`/`<?=`/`?>` openers with
inline HTML outside them, heredoc and nowdoc with interpolation only in the former, type annotations in
every slot the grammar allows one including ADR 0036's inline shapes, the `tainted`/`secret` qualifiers
and `decimal`, MWL's own keywords (`spawn`, `spawn script`, `autoload`, `type`, `by`, property hooks),
ADR 0070's duration literals, and `#[...]` attributes told apart from `#` comments — and **nothing MWL
rejects may be coloured as valid** (`===`, legacy casts, `|>`, the alternative colon syntax). Then
**semantic tokens** colour what a regex structurally cannot know: `defaultLibrary` on a `Core` class,
enum members, type aliases, and two modifiers of MWL's own — **`tainted` and `secret`** — so a qualified
value is visibly qualified at every use site, which is the cheapest teaching surface ADRs
[0024](../adr/0024-taint-tracking-for-injection-sinks.md) and
[0033](../adr/0033-secret-qualifier-for-confidential-values.md) have. **MWL ships no colours** — colour is
the user's theme's — so both layers ship *names*, every one of them from the standard TextMate vocabulary
or LSP's standard legend, because a theme styles only names it recognises and an invented scope renders as
unstyled body text. The two custom modifiers reach a theme through `semanticTokenScopes`, and the
extension contributes no colour-customization defaults at all. A **second grammar** colours `.mwlt` and
`.lspt` themselves — sections, with MWL embedded in `--FILE--` and PHP in `--ORACLE--` — which is nearly
free and is the grammar whose audience is this project's own loop, currently reading hundreds of those
files as flat grey text.

**The extension.** `editors/vscode`, a TypeScript package outside the Cargo workspace: `.mwl` registration
(**and not `.php`**), `language-configuration.json` — whose `wordPattern` must include `$`, or
double-clicking `$total` selects `total` — the two grammars above, `mwl lsp` spawning via
`vscode-languageclient` with a configurable path falling back to `PATH`, a `LanguageStatusItem` for server
health and version, `mwl run`/`mwl test` as Tasks **with a `problemMatcher`** over the diagnostic
renderer's existing format, so a failure is a clickable Problems-panel entry rather than terminal text,
and an AST explorer panel backed by **`mwl ast --json --resilient`** — which is built here, because
`--json` does not exist today and `{stmts:#?}` has no stability contract. The extension refuses a binary
whose version it does not understand rather than answering confusingly, and holds **no language logic**,
enforced by a dependency-allowlist test rather than by review. The setting and command identifiers
(`mwl.path`, `mwl.lsp.enable`, `mwl.lsp.debounce`, `mwl.lsp.trace.server`; `mwl.run`, `mwl.test`,
`mwl.showAst`, `mwl.restartServer`) are frozen at M4B because they are public API — a rename breaks a
user's `settings.json` silently. Extension id `mwl-lang.mwl`; `package-lock.json` is committed because
`npm ci` needs it; CI produces an installable `.vsix`; **nothing is published** —
[ADR 0016](../adr/0016-ide-integration.md) *Revisiting* keeps that open.

**How editor behaviour is checked.** A **`.lspt` case** is the sibling of `.mwlt`: the same section lexer,
a `<|>` cursor, a `--REQUEST--` line and a frozen canonical `--EXPECT--` rendering, run by **`mwl
lsp-test`** printing the same `N passed, M failed` line — so the loop gates editor behaviour through the
check kind it already has. Coverage is **inferred** from the node each cursor resolved to, never declared,
and `every_request_answers_every_construct` fails naming each empty cell of the request × construct
matrix. Extension tests run in two tiers: headless Node every iteration (grammar snapshots via
`vscode-textmate`, a contributions/allowlist test, a protocol round-trip against the real binary), and
`@vscode/test-electron` in the real extension host once per green tree.

**Not here:** format-on-save (`mwl fmt` is M10 — [ADR 0039](../adr/0039-canonical-code-formatting.md)),
rename, extract refactorings, workspace symbol search, inlay hints, signature help, `documentHighlight`,
the Test Explorer, profiler visualization, debugger UI, and any code action whose fix a diagnostic does
not already compute. The last two of those look adjacent to what M4B does build and are not:
`documentHighlight` needs resolution applied to *every* occurrence, a different walk from resolving one,
and inlay hints encode idioms still moving through M5–M9. No PhpStorm work at all — PhpStorm stays
entirely at M10 per [ADR 0016](../adr/0016-ide-integration.md).

**Verify:** concatenating tokens and trivia in offset order reproduces every file in `examples/`, `tests/`
and the vendored `php-src` corpus byte-for-byte. Parsing every prefix of every `examples/*.mwl` at a token
boundary panics on none, answers a `SyntaxIndex` lookup at the final offset on all, and reports a
diagnostic on each prefix that is genuinely incomplete; a fuzz target over truncated and mid-edit inputs
finds no panic in five minutes. `mwl lsp-test tests/lsp/` reports `0 failed` and the coverage matrix has
no empty cell — including a case per request proving that an unclosed brace or a trailing `->` does not
stop diagnostics, hover, completion or semantic tokens working on the well-formed code around it, which is
ADR 0040's core claim. A full re-analysis of a 1,000-line document stays under the named latency bound —
the measurement that says giving up incremental reparse still pays. The grammar snapshot assigns the
expected scope to every construct listed above, `#[Route]` included and `===` receiving no operator scope,
and every scope it emits is on the standard-name allowlist. The gate of *Diagnostics are phase-gated* is a
case in both directions — gated, `E0102` and not `E0301`; with `phase=all`, both. A position round-trips
through both encodings on a multi-byte line, a BOM document answers correct offsets, and a CRLF document's
columns match an LF one's; `println!` appears in no crate the server links; editing a required file
re-publishes the requiring document's diagnostics untouched. The extension activates on `.mwl` and not on
`.php`, shows TextMate colour before the server answers and semantic colour after, registers a legend
equal to the one the server declares, round-trips all nine requests and both code actions with no language
logic in its own source, exposes `mwl run`/`mwl test` as Tasks whose failures populate the Problems panel,
selects `$total` whole on a double-click, opens a `.mwlt` case coloured, and renders the AST panel for a
file that does not compile. `tokio` appears in neither `Cargo.toml` nor `Cargo.lock`.
