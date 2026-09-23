# Handoff

## State

Goal `core-html-and-1-more` is under way. `Core\Html::escape`, `Core\Html::join` and `Core\Html::parse`
own every feature proof: an `about.md`, three examples, one attack, one bench and a Rust `#[test]`
carrying a `covers:` marker in `crates/nvs-stdlib/src/html.rs`'s `mod tests`. No proof found a bug.
Three members are left: `Core\Html::sanitize`, `Core\Html::toSource` and `Core\Http::allowUrl`
(`docs/agent/loop-goal.md` § *The item list*). The driver's `dossier: Core\Html` check stays red
until the first two land.

## Next group

**Stage: the dossier item list** — one file set: `crates/nvs-stdlib/src/html.rs` and the
`core/Html/<member>` directories under `docs/examples/`, `tests/hostile/` and `benches/members/`.
One slice is one feature with all its feature proofs.

- [ ] **`Core\Html::sanitize`** — owes about, examples, hostile, perf, tests. `rule:testing/feature-proofs`,
      `rule:core-classes/html-parsing`. `crates/nvs-stdlib/src/html.rs:1683`. Its tests
      `sanitize_is_an_adr_0024_launderer_beside_escape` and the mXSS fixed-point test already exist
      and only lack a `covers:` marker, or a new end-to-end one through `call` in the shape of
      `html_parse_answers_a_whole_document_for_a_fragment_and_for_nothing`.
- [ ] **`Core\Html::toSource`** — owes about, examples, hostile, perf, tests. `rule:testing/feature-proofs`.
      `crates/nvs-stdlib/src/html.rs:739`. `to_source_is_the_only_way_out_of_markup_and_it_takes_a_reason`
      is the existing test that needs the marker.

## Backlog

- `Core\Http::allowUrl` — the goal's sixth item, `crates/nvs-stdlib/src/http.rs:147`; a different
  file set, so it is its own session.
