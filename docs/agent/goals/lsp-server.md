---
milestone: M4B
---
# Loop goal 16 — `nvs lsp`, and the format that can prove it

Build `crates/nvs-lsp`: `lsp-server` and `lsp-types`, **synchronous, no async runtime**, speaking LSP over
stdio behind `nvs lsp`. [docs/plan/m4b.md](../../plan/m4b.md) is the scope and this file does not restate
it; `rule:ide/one-grammar-one-tree` is the current rule wherever it
and `rule:ide/every-feature-is-staged-behind-its-dependency` seem to differ.

**What is different about this goal, and what every session must hold: for the first time the loop is
verifying something that is not a program's stdout.** An LSP answer is not printed by anything, and the
driver's stop path is exit codes and exact output only. The mechanism that closes that gap is a `.lspt`
case and `nvs lsp-test`, and it lands in **stage 3, before any request handler does**, because a request
built before its case format exists is a request nobody can prove.

Goal `resilient-tree` already built what this goal reads: the `SyntaxIndex`, explicit recovery, and `utf16_col` /
`offset_of` in `nvs-diagnostics`. **Position arithmetic has one home and this crate is not it** — no
conversion of any kind is written here.

## Stage 0 — the catch-up

Nothing. Every half this goal stands on was built by name in goal `resilient-tree`.

## Stage 1 — the floor

Goal `surface`'s whole acceptance list. Nothing in this goal touches `nvs-stdlib`, `nvs-db` or `nvs-server`, so a
failure there is a real regression and never a scope question.

## Stage 2 — the crate, the subcommand, the handshake, and the wire

The crate with its two dependencies — each owes its `[workspace.dependencies]` line saying why that crate,
a `cargo deny check`, and `python tools/gen-attribution.py`. Synchronous, stdio, behind `nvs lsp`.
`initialize` declares **exactly** the capabilities of this goal's items and no others, and reports the
binary's version so a client can refuse a mismatch. Position encoding is negotiated per LSP 3.17: offer
`utf-8` and `utf-16`, take `utf-8` when the client offers it, using goal `resilient-tree`'s conversions.

This stage also lands the rule that costs nothing now and a day later: **nothing but the protocol writes to
stdout.** `nvs-lsp` does not take `nvs-cli`'s `clippy::print_stdout` allowance, and a test asserts that no
crate the server links calls `println!`. It holds today only by accident, and its failure mode is the
server dying for no visible reason.

## Stage 3 — the case format, its runner, and the gate

The `.lspt` case exactly as [conventions.md](../conventions.md) § *An `.lspt` case* already specifies it —
`--TEST--`, `--FILE--`, `--FILE <path>--`, one `<|>` cursor, `--REQUEST--`, a frozen `--EXPECT--` — over
the section lexer goal `resilient-tree` extracted. **The format's home is this crate's module doc** and the canonical
rendering of every response kind is `nvs_lsp::render` and nowhere else, so no case invents a spelling.

`nvs lsp-test <paths>` walks directories for `*.lspt` and prints `N passed, M failed` — the exact line
`tools/loop.py`'s `nvs-suite` kind parses, which is why the driver needs no change to gate editor
behaviour. `--coverage` prints the request × construct matrix, **inferred from the node each cursor
resolved to and never declared by the case**, and `every_request_answers_every_construct` fails naming
each empty cell.

**That guard is this goal's real definition of done.** A count of cases is a proxy that six hundred cases
about one construct can satisfy; a test that enumerates the grammar is not. The `min_passing` in the TOML
is a floor, not the gate.

## Stage 4 — the document store

`Full` sync, open buffers overlaid on the `require`/`autoload` graph the document is the entry point of, a
debounce (150 ms, `nvs.lsp.debounce`), `$/cancelRequest`, and cancellation of an analysis whose document
version nobody is looking at any more. Two things here are invisible when they work and baffling when they
do not: editing `B.nvs` must re-analyse an open `A.nvs` that requires it, and a BOM or CRLF document must
answer the same offsets an LF one does — spans are byte offsets, so normalizing line endings here shifts
every column in the editor.

## Stage 5 — `publishDiagnostics`, phase-gated

The existing `nvs check` pipeline, at the negotiated encoding, with `code` from `Code` and **no
`codeDescription`** — it needs a URL, the website's own are still placeholders, and a link to a Rust
constant is worse than none. Published for **open documents only**.

The gate is the content of this stage, not a detail of it:
`rule:ide/diagnostics-are-phase-gated` has the worked example where
one typo yields a spurious `E0301` *above* the `E0102` that caused it. A file that produced an `E00xx` or
`E01xx` diagnostic suppresses `E03xx` and `E04xx` **for that file only**. Both directions are cases:
gated, and `phase=all`.

## Stage 6 — `hover`, `definition`, `completion`

One group of three: they share the `SyntaxIndex`-plus-type-table path, and never fewer than one per
session.

- **`hover`** — the declared type under the cursor from `nvs_types::ExprTypeTable`; for a `Core` member its
  `nvs_stdlib::registry` signature row and reference card
  (`rule:core-api/reference-card`); for a
  declaration, its own **doc comment**, which goal `doc-comments` made a checked structure rather than raw trivia
  (`rule:tooling/doc-comment-is-three-slashes`).
- **`definition`** — within the document or anywhere in its resolved graph.
- **`completion`** — keywords filtered by position; members off a resolved receiver, instance and static,
  user classes and `Core` registry classes alike; enum cases after `Type::`; in-scope variables.
  **No workspace symbol search** — that needs M10's index, and `rule:ide/five-features-are-one-reference-index` puts it there by name.

## Stage 7 — the five projections

`semanticTokens/full` with [ADR 0099 § 4](../../decisions/0099.md)'s
legend — `defaultLibrary` on a `Core` class, and the `tainted` / `secret` modifiers included, which is the
point of the item and not a detail of it. Then `documentSymbol`, `foldingRange`, `selectionRange` and
`documentLink`: namespace, class, interface, enum, method, property, class constant and type alias for the
first; the same walk plus trivia's comment blocks for the second; the `SyntaxIndex` ancestor list
*unchanged* for the third; and the path literal in a `require`/`autoload` for the fourth. Four requests,
one walk each, **no new analysis in any of them.**

## Stage 8 — `nvs/redactions`, the one request of Novis's own

`rule:ide/redaction-ranges-come-from-the-server` and `rule:ide/redaction-covers-bytes-only`:
a `TextDocumentIdentifier` in, a list of `{range, kind}` out, `kind` being `secretLiteral` today and an
open string for whatever a later qualifier needs. The server computes the ranges because the alternative is
the client deciding what a secret is, which `rule:ide/one-server-two-thin-clients` forbids.

**Its fail direction is named and is not a preference:** a range whose expression cannot be typed but whose
binding declares `secret` is answered **anyway**, so a value does not flash on screen on every keystroke
while its literal is being typed. A security default may not have becoming-visible as its failure mode.
The client half — concealment, reveal, the two commands — is goal `editor`'s.

## Stage 9 — the two code actions, and the boundary

Casing ([0029](../../decisions/0029.md)/[0030](../../decisions/0030.md))
and `(int)$x` → `$x as int` ([0034](../../decisions/0034.md)), both translations of a
`Suggestion` the `Diagnostic` already carries, registered under `source.fixAll.nvs`. **A code action whose
fix the checker would have to compute is off path** — that boundary is the whole content of this stage, and
`rule:ide/narrow-an-annotation-to-its-literal` is the worked
example of one that sits on the far side of it, at M10.

Expect `Diagnostic::suggestions` to be sparsely populated: it has been carried since M0 and read by
nothing, and there are three producers in the whole workspace. **Filling it for the two codes above is the
actual work of this stage**, not the code-action plumbing.

## Stage 10 — the latency guard

A full re-analysis of a ~1,000-line document under a named bound, in the shape
`benches/abi-probe/tests/perf_guards.rs` already uses. Goal `resilient-tree` traded incremental reparse away; this is the
measurement that says the trade still holds, and it is not optional.

## Stage 11 — the reference chapter

`docs/reference/tools/40-editor.md` is created here with two headings — `# nvs lsp` and `# nvs lsp-test` —
each owing what a tool feature owes
(`rule:testing/feature-proofs`, `POLICY["tool"]`): one test, one
example under `docs/examples/`, one program under `tests/hostile/`. Goal `editor` adds the extension's heading to
the same chapter. `python tools/reference.py --check` is in the acceptance list.

## Standing decisions — pre-authorized, do not stop the loop for these

- **`lsp-server` and `lsp-types`, and no async runtime.** `tokio` does not enter this workspace. Goal `server`
  brought `hyper` and its five dependencies in and that claim survived, because it is about a *runtime* and
  never about the `Future` trait. If a needed capability appears to require one, that is a real `BLOCKED`
  naming the capability — not a judgement call.
- **Nine standard requests, one of Novis's own, two code actions, and that list is closed.** The test to
  apply to a tenth standard request is the one that admitted `selectionRange`, `foldingRange` and
  `documentLink`: the data structure this goal already builds **is** the answer, so the request is a
  projection rather than a feature. `documentHighlight` is the worked example of one that fails it —
  `rule:ide/five-features-are-one-reference-index` owns it, at M10. A session that finds another one "would be easy" applies the test honestly and
  puts it in `## Backlog` when it fails.
- **No ADR slots.** ADRs 0099, 0101, 0040, 0117 and 0137 decide everything here. Anything smaller is
  decided-and-recorded in this crate's module doc, never a new number and never `BLOCKED`.
- **The expected output of a `.lspt` case is frozen; its *source* is not.** Correcting a case's document,
  cursor or request is a bug fix needing no permission. Editing its `--EXPECT--` to make it pass is the
  thing that must never happen — re-freeze deliberately, say why in the commit message, move on.
- **`.lspt` and `.nvst` stay two suites.** They share a section lexer and nothing else.
- **Every request slice ships its own `.lspt` cases.** A handler landing without them is not a finished
  slice, and a case corpus made largely of deliberately broken files is the point rather than a smell.
- **No colour work here.** The grammar, the legend's client half and the extension are goal `editor`'s.
