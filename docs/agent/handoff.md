# Handoff

## State

**Goal `xml-tree`, stage 4 is complete and green.** `Core\Html::parse(string $document):
Core\Xml\Node` drives `html5ever` through `crate::html`'s own `Sink` — a `TreeSink` over a flat
arena — into `crate::xml`'s `Parsed` nodes, so the boundary between the crate and the tree is where
the one-node-family rule is enforced rather than restated. All three tests the stage's acceptance
check names pass, and `rule:core-classes/html-parsing` is now `shipped` with them and the three new
`.nvst` cases as its guards.

**The dependency is `html5ever = "0.39"`, seventeen crates, MIT or Apache-2.0 and no C.** The audit
under `rule:packaging/a-c-dependency-answers-two-questions` is written on the workspace row
(`Cargo.toml`, the *HTML parsing* section); `markup5ever_rcdom` is deliberately not taken.
`THIRD-PARTY-LICENSES.txt` is regenerated and `cargo deny check licenses` is clean.

**One pre-existing crash is fixed on the way, and it was `Core\Xml`'s.** `instance_of` recursed once
per node, so a document nested past about 900 elements exhausted the task stack before
`DEPTH_CEILING` refused it — a crash where the bound was supposed to be. It is now iterative over a
heap stack, and `Core\Html::parse` holds the same ceiling from the other side by flattening a tree
past it rather than refusing one, since that door has no way to refuse.

**`examples/html-sanitize.nvs` is still the failing acceptance check** and is still not a
regression: it is stage 5's fixture and cannot be written before `Core\Html::sanitize` exists.

## Next group

**Stage 5: `Core\Html::sanitize`, which is why the tree matters** — one file set,
`crates/nvs-stdlib/src/html.rs` plus `examples/html-sanitize.nvs` and new cases under
`tests/conformance/core/`. The whole group is specified by `rule:core-classes/html-sanitize` and
`rule:security/tainted-qualifier`, and stage 5's `[[check]]` in `docs/agent/loop-goal.toml` names
the four tests it must end with.

- [ ] **A serialiser over `crate::xml::Parsed`, written here rather than in `crate::xml`**, because
      `rule:core-classes/html-parsing` says serialization follows the door: WHATWG rules through
      this one, XML rules through the other. It is what `sanitize` rebuilds a document with, and it
      is the piece with no caller yet, so write it against the walk `crates/nvs-stdlib/src/html.rs:1362`
      already uses to read a tree in the tests. Specified by `rule:core-classes/html-sanitize`.
- [ ] **The closed allowlist and `Core\Html::sanitize(tainted string): Core\Html\Markup`** — the
      row at `crates/nvs-stdlib/src/html.rs:130`, the card beside `PARSE_DOC` at
      `crates/nvs-stdlib/src/html.rs:279`, the body, and the `address()` arm at
      `crates/nvs-stdlib/src/html.rs:302`. It is the **second** launderer on this class and
      `escape` is the auto-applied one, so `Qual::Launder` on the parameter is what makes
      `html_escape_launders_for_the_html_sink_and_for_no_other`'s set claim need re-reading rather
      than re-asserting. Specified by `rule:core-classes/html-sanitize`.
- [ ] **The four tests stage 5's check names**, in `crates/nvs-stdlib/src/html.rs:1284`'s `mod
      tests`: `sanitize_is_an_adr_0024_launderer_beside_escape`,
      `the_allowlist_is_closed_and_is_not_configurable_by_a_caller`,
      `an_element_outside_the_allowlist_is_dropped_rather_than_escaped_in_place` and
      `parse_sanitize_serialise_reparse_reaches_a_fixed_point_over_the_mxss_corpus` — the last is
      the acceptance property, so its corpus is the slice's real content.
- [ ] **`examples/html-sanitize.nvs`**, the acceptance fixture that has been the failing check since
      this goal opened. It is named at `docs/agent/loop-goal.toml:10`, and that file's header says
      the expected output is frozen while a fixture's *source* is not — so write the program to the
      output rather than the other way round. Specified by `rule:core-classes/html-sanitize`.

## Backlog

- `Core\Html`'s automatic escape-and-lift at the HTTP sink still waits on the response existing —
  `crates/nvs-stdlib/src/html.rs`'s module doc, § *Known gaps*.
- `spec-classes-part-two-outstanding.txt` loses `§17 Core\Xml` at the end of stage 5 —
  `docs/agent/loop-goal.md` § *Stage 5*.
- Whether a subset-free `<!DOCTYPE html>` should parse through `Core\Xml` is still undecided;
  `rule:core-classes/xml-refuses-by-construction` currently says no, and `Core\Html` now drops one.
- `Core\Xml`'s streaming reader and `Core\Html`'s parse share no code and need not, but neither has
  a differential case against PHP — `tests/differential/` holds none for either.
