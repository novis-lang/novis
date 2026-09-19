---
milestone: M4B
---
# Loop goal 17 — `editors/vscode`, and colour

Build the extension: TypeScript, outside the Cargo workspace, exactly where
[ADR 0016 § 5](../../decisions/0016.md) puts it.
[docs/plan/m4b.md](../../plan/m4b.md) is the scope; `rule:ide/highlighting-is-two-layers` and `rule:ide/contributions-are-frozen-and-only-ever-added` are the colour lists and the frozen
contribution roster, and **neither is a starting point to improve on during the run**.

This is M4B's last entry and the only one on the chain whose source is not Rust — goals `request-json` and `test-request` follow
it, and neither touches this tree. Two things follow from that
and every session should hold them. **The extension holds no language logic** — enforced by a
dependency-allowlist test rather than by review, because "we'll keep it thin" is not a check. And
**Novis ships no colours**: every scope name comes from the standard TextMate vocabulary and every token
type from LSP's standard legend, because a theme styles only names it recognises and an invented scope
renders as unstyled body text — a grammar that is technically correct and visibly broken.

## Why here

`editors/vscode` — the first non-Rust source in the repository, and the last entry of M4B's four.

## Stage 0 — the catch-up

Nothing, and two things worth knowing rather than rediscovering: `tools/orient.py` already globs
`editors/*/src/**/*.ts` for its module map, and `tools/verify.py` already carries the extension step and
its `npm` plumbing, dormant until `editors/vscode` exists. If a session finds either missing, that is a
blocker for stage 2 and belongs in the handoff — not a tooling slice invented mid-goal.

## Stage 1 — the floor

Goal `lsp-server`'s whole acceptance list, which is everything: the parity program, goals `temp-sweep` through `surface`, and the server.

## Stage 2 — the package, and the identifiers that are public API

`.nvs` registration (**and not `.php`**), `tsconfig`, lint, npm scripts, a committed `package-lock.json`
(`npm ci` needs one, and an unpinned tree makes the grammar snapshots reproducible only by luck), the four
`.gitignore` lines (`node_modules/`, `out/`, `.vscode-test/`, `*.vsix` — a session that commits
`node_modules` is a session whose commit nobody can review), and extension id `nvs-lang.nvs`.

`language-configuration.json` is content rather than a checkbox: comments, brackets, auto-closing and
surrounding pairs, indentation and on-enter rules, folding markers — and **`wordPattern` must include
`$`**, which is the one a borrowed PHP config gets wrong and which makes double-clicking `$total` select
`total`.

**The identifiers are frozen here** — a setting lives in someone's `settings.json` and a command id in
their keybindings, so they are added later and never renamed:

- settings — `nvs.path`, `nvs.lsp.enable`, `nvs.lsp.debounce`, `nvs.lsp.trace.server`,
  `nvs.secrets.redact`, `nvs.taint.mark`
- commands — `nvs.run`, `nvs.test`, `nvs.showAst`, `nvs.restartServer`, `nvs.revealSecret`,
  `nvs.hideSecrets`

The stage's own test asserts `package.json` declares what the extension claims, **depends only on the
allowlist**, and contributes **no colour-customization defaults**. That test is how "the extension holds no
language logic" stops being a promise.

## Stage 3 — the TextMate grammar

Colour the instant a file opens, before the server exists. Everything it must colour — the dual-mode
`<?nvs`/`<?php`/`<?=`/`?>` openers with inline HTML outside them, heredoc and nowdoc with interpolation
only in the former, type annotations in every slot including `rule:types/object-top`'s inline shapes, the
`tainted`/`secret` qualifiers and `decimal`, Novis's own keywords (`spawn`, `spawn script`, `autoload`,
`type`, `by`, property hooks), `rule:types/duration-literal`'s duration literals, `#[...]` attributes told apart from `#`
comments — and the constructs it must **not** colour as valid, is
[ADR 0099 § 4](../../decisions/0099.md)'s list. **Do not re-derive it
and do not shorten it.** Goal `surface` landed `|>` and `let`/`is`, so that list's refusals are now real
diagnostics the grammar can be checked against; goals `typed-callable` and `doc-comments` landed `callable<…>` signatures and `///`
doc comments, which are colour surface `rule:ide/highlighting-is-two-layers` predates and which this stage adds.

The harness is a headless snapshot test through `vscode-textmate` + `vscode-oniguruma` — plain Node, no
editor, no display — asserting every emitted scope against a standard-name allowlist. **This is the largest
single item in the goal and is expected to take more than one session: split it by construct family, never
by file.**

## Stage 4 — the `.nvst`/`.lspt` grammar

A `begin`/`end` rule per section, anchored on `^--NAME--$` and ending at `(?=^--[A-Z])`, with stage 3's
grammar `include`d inside the four sections that hold a program — `--FILE--`, `--FILE <path>--` (its own
rule: the header carries an argument), `--SKIPIF--` and `--CLEAN--` — PHP's inside `--ORACLE--`, and
`constant.character.escape` on the `%` escapes in `--EXPECTF--`/`--EXPECTF-ERROR--`. Every other section is
literal text with only its delimiter coloured.

**The section list is [`crates/nvs-test`](../../../crates/nvs-test/src/lib.rs)'s module doc** — read it
rather than inferring the set from the corpus, and treat a section it names but this stage does not as
literal text. The one real risk is the PHP leg: VS Code splits PHP across `source.php` and `text.html.php`,
and only the latter handles the `<?php` opener every `--ORACLE--` body starts with, so settle which one in
the snapshot test rather than by reading documentation.

Nearly free, **group it with stage 3** — same directory, same harness — and it is the cheapest possible
check that stage 3's grammar is embeddable at all. It is also the one grammar whose audience is this
repository's own loop, which writes hundreds of those files and reads them as flat grey text today.

## Stage 5 — the client

`nvs lsp` spawned via `vscode-languageclient` with a configurable path falling back to `PATH`, the
`LanguageStatusItem` for server health and version, and the settings block. The extension **refuses a
binary whose version it does not understand** rather than answering confusingly. Plus the headless
**protocol round-trip** that drives the real binary from Node — the one test that proves the two halves
speak the same protocol without an editor in the room.

## Stage 6 — `secret` concealment, and `tainted` left alone

`rule:ide/redaction-ranges-come-from-the-server`: the
ranges come from goal `lsp-server`'s `nvs/redactions` and the client draws them and knows nothing. **A `secret`
literal is concealed by default** — blurred in place, the character cells kept, so every edit still
addresses the real text — and a reveal is **per range** and dies when the editor closes, because the threat
is an unattended screen and no API reports one. Revealing one secret leaves a second in the same file
hidden; that is a case, not a nicety.

`tainted` is decorated **only if the user asks** (`nvs.taint.mark`, default `off`): a glyph is added
content, and how a construct looks stays the theme's call.

## Stage 7 — Tasks, the problem matcher, and the AST panel

`nvs run` and `nvs test` as Tasks **with a `problemMatcher`** — two regexes over the renderer's existing
format (`error[E0301]: message`, then `  --> file:line:col`). The matcher is the difference between the
Tasks being useful and being decorative: without it a failure is terminal text nobody can click.

The AST panel over `nvs ast --json --resilient`, which goal `resilient-tree` built. It **inherits the redaction
obligation**: the panel prints the placeholder rather than the literal, or it renders in a webview the
credential the buffer behind it is concealing.

## Stage 8 — the extension host, the artifact, and CI

The `@vscode/test-electron` suite: activation on `.nvs` and not `.php`, Tasks present, status item
rendering, the panel populating, and **the semantic-token legend the client registers equal to the one the
server declares** — which no unit test on either side alone can see. It runs **in CI and nowhere else**
(see *Standing decisions*) and it isolates its profile. Then `.vsix` packaging, which is headless and does
gate an iteration. Two CI jobs are added beside the ones already there —
[ci.yml](../../../.github/workflows/ci.yml) is the count of those and this file does not restate it: the
headless suites on all three platforms, since a `.vsix` is cross-platform and a path bug is not, and the
extension-host run on Linux under `xvfb-run`.

## Stage 9 — the reference chapter's last heading

`# The VS Code extension` joins goal `lsp-server`'s two in `docs/reference/tools/40-editor.md`, owing what a tool
feature owes (`rule:testing/feature-proofs`, `POLICY["tool"]`): one
test, one example, one hostile program.

## Standing decisions — pre-authorized, do not stop the loop for these

- **The extension is `.nvs` only.** It does not claim `.php` even though `nvs-syntax` parses it — that
  fight is with every PHP extension a user already has, and losing it silently looks like Novis being
  broken.
- **Nothing is published.** `.vsix` as a CI artifact; no Marketplace publisher, no listing, no icon or
  branding work. `rule:ide/one-server-two-thin-clients` *Revisiting* keeps that open and this goal does not close it.
- **Colour is specified, not designed.** `rule:ide/highlighting-is-two-layers` lists what the grammar must colour, what it must
  refuse to colour, and the semantic token types and modifiers. A gap in it is a handoff note, never an
  improvement made during the run.
- **Dependencies: one is named, the rest are yours.** `vscode-languageclient` for the client; the Node
  test stack and the grammar test libraries you pick against
  [ADR 0051 § 4](../../decisions/0051.md)'s two questions. An npm dependency owes the
  allowlist entry stage 2 builds and nothing else, and lives in `devDependencies` wherever it can — a
  runtime dependency ships to users and a test library does not.
- **The extension-host tier is CI's, and is not on the acceptance list.** It needs a display, and the
  display on the machine the loop runs on belongs to a person who has this repository open in VS Code.
  Launching a second one there is worse than rude: without an isolated profile it attaches to the running
  instance and exits, so the suite reports no results and the iteration goes red over a window manager.
  It cannot be expressed as a skip either — a `command` check has no platform key, and `memoize` only
  short-circuits a check that has already *passed* — so the only way to keep it off that desktop is to
  leave it out. **Do not add it**; CI runs it on Linux under `xvfb-run`, which is where `rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone` puts
  the milestone gate. A session never runs it by hand.
- **The host suite isolates its profile, wherever it runs.** `--user-data-dir` and `--extensions-dir` to a
  throwaway directory, and it opens a fixture folder, never this repository. Unisolated it loads the
  developer's own extensions, and a test touching `ConfigurationTarget.Global` writes the six frozen
  `nvs.*` settings into their real `settings.json`. Prefer `@vscode/test-cli`, which sets an isolated
  profile up per run, over driving `@vscode/test-electron` directly.
- **No ADR slots.** ADRs 0099, 0101 and 0016 decide everything here; anything smaller is
  decided-and-recorded in the extension's own README, never a new number.
- **No language logic in TypeScript.** If the client seems to need to know what a construct means, the
  answer is a server request that already exists or a `## Backlog` entry — never a regex in the client.
- **No pixel tier.** Driving the real editor with Playwright to assert a decoration was *drawn* is
  rejected, and not on cost grounds: anything needing a display is outside the tier the loop gates on, so
  it cannot buy the loop a check at all. What is left after the range test (the server's), the position
  conversion (a unit test) and the reveal state machine (logic) is a CSS constant that never varies — the
  test that never fires. Record it in the extension's own README as decided-and-rejected, so the next
  session does not re-derive it.
