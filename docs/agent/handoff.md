# Handoff

## State

Goal `core-html-and-1-more` is under way. `Core\Html::escape`, `Core\Html::join`, `Core\Html::parse`
and `Core\Html::sanitize` have every feature proof: an `about.md`, three examples, one attack, one bench
with a perf ledger figure, and a Rust `#[test]` with a `covers:` marker in the `mod tests` of
`crates/nvs-stdlib/src/html.rs`. No proof found a bug. Two members are left: `Core\Html::toSource` and
`Core\Http::allowUrl` (`docs/agent/loop-goal.md` § *The item list*). The driver's `dossier: Core\Html`
check stays red until `toSource` lands.

## Next group

**Stage: the dossier item list** — one file set: `crates/nvs-stdlib/src/html.rs` and the
`core/Html/<member>` directories under `docs/examples/`, `tests/hostile/` and `benches/members/`. Then
`crates/nvs-stdlib/src/http.rs` with the `core/Http/allowUrl` directories. One slice is one feature
with all its feature proofs.

- [ ] **`Core\Html::toSource`** — owes about, examples, hostile, perf, tests. `rule:testing/feature-proofs`,
      `rule:core-classes/html-auto-escape`. `crates/nvs-stdlib/src/html.rs:739`. The Rust test goes
      in `mod tests` beside `html_sanitize_rebuilds_a_fragment_and_its_answer_is_a_fixed_point`, through
      `call`; `carried` is the helper that reads a `Markup`'s bytes. `toSource` takes a second argument,
      the reason text, which every example must pass.
- [ ] **`Core\Http::allowUrl`** — owes about, examples, hostile, perf, tests.
      `rule:testing/feature-proofs`, `rule:http-server/allow-url-pins-the-address`.
      `crates/nvs-stdlib/src/http.rs:515`.

## Backlog

- Only a source literal can be lifted with `as Core\Html\Markup`. `'a' . "\n" as Core\Html\Markup` gives
  `E0716`/`E0710`, so write a single double-quoted literal (`rule:core-classes/html-auto-escape`).
- A bench whose chained index always lands on the same input needs `+ $i % 2`, as
  `benches/members/core/Html/parse.nvs` and `sanitize.nvs` do (`benches/members/README.md`).
