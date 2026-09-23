# Handoff

## State

Goal `core-html-and-1-more` is under way. Every `Core\Html` member now has all its feature proofs:
`escape`, `join`, `parse`, `sanitize` and `toSource`. Each has an `about.md`, three examples, one
attack, one bench with a perf ledger figure, and a Rust `#[test]` with a `covers:` marker in the
`mod tests` of `crates/nvs-stdlib/src/html.rs`. No proof found a bug. The `toSource` slice rewrote a
stale doc comment on `nvs_core_html_to_source`. That comment said nothing refused a computed reason,
but `E0805` does. One member is left: `Core\Http::allowUrl` (`docs/agent/loop-goal.md` §
*The item list*).

## Next group

**Stage: the dossier item list** — one file set: `crates/nvs-stdlib/src/http.rs` and the
`core/Http/allowUrl` directories under `docs/examples/`, `tests/hostile/` and `benches/members/`.
One slice is one feature with all its feature proofs.

- [ ] **`Core\Http::allowUrl`** — owes about, examples, hostile, perf, tests.
      `rule:testing/feature-proofs`, `rule:http-server/allow-url-pins-the-address`.
      `crates/nvs-stdlib/src/http.rs:515`. The Rust test goes in that file's `mod tests`, through
      `nvs_runtime::call`, with `ctx.take_pending()` for a refusal. `python tools/dossier.py --id
      'Core\Http::allowUrl'` prints the paths it owes. When this lands, the goal is done: run the
      DONE gates in the session prompt.

## Backlog

- `benches/members/core/Html/escape.nvs` picks the same name on every round (playbook: *A bench
  chained as `$inputs[$total % N]`*). Fix the chaining and re-measure with `--record-perf --force`.
