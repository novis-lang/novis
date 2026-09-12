# Handoff

## State

**Goal `markup-literal` is complete: every stage is on disk and `python tools/verify.py` is green
across the tree.** `Core\Html::join(array<Core\Html\Markup> $parts, Core\Html\Markup $separator)` is
a row on `Core\Html` with its reference card, its helper and its `address()` arm
(`crates/nvs-stdlib/src/html.rs:157`), and it neither trusts nor escapes — one string allocation and
one object allocation per call whatever the list holds.

`rule:core-classes/html-literal` is now **shipped** and its `guardedBy` names the ten cases this goal
added; `docs/novis.md` was regenerated and picks `join` up on its own. The carrier's four
enumerating sentences — the module doc, `MARKUP`'s own doc, the memberless test's doc and its
assertion message — now name the literal and `join` rather than a count.

Nothing is blocked. The conformance corpus is 1756 cases.

## Next group

**Stage 6 tail: the rosters outside the rulebook** — one file set: `docs/spec/01-core-library.md`
with `tests/differential/` for the twin.

- [ ] **The spec's `Core\Html` cell names neither `join` nor the literal** — it reads "`escape` (the
      auto-applied launderer), `sanitize`, `Markup`, and the WHATWG HTML parser" at
      `docs/spec/01-core-library.md:1196`, which is the one file `spec_registry_coverage.rs` reads
      spec-side. Add the member and the literal to that cell; a spec edit owes
      `python tools/check-migration.py`. `rule:core-classes/html-literal`.
- [ ] **A differential twin for the literal** — `rule:core-classes/html-literal`'s
      `divergesFromPhp` says the nearest PHP is `<?= ?>` with `htmlspecialchars`, so the oracle case
      is that concatenation against `` html`…` `` over the same data, as a new case under
      `tests/differential/core/` — the tree holds no HTML twin today. A
      `tests/conformance/` case may not carry `--ORACLE--`
      (`docs/agent/conventions.md:110`), which is why this is a second file.

## Backlog

- `Core\Html::toSource`'s `$reason` is not yet required to be a source literal — both type bands are
  full (`E0499`, `E0799`); `crates/nvs-stdlib/src/html.rs`'s *Known gaps* owns it.
- The HTML sink's automatic lift waits on the HTTP response existing — same module doc.
- `nvs fmt`, the LSP template region and `nvs convert` are rules with no code yet, by this goal's
  own standing decisions; goal `fmt` is the next chain entry and carries the first of them.
