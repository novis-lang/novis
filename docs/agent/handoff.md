# Handoff

## State

**M4's Stage 8, depth.** The tree is at **819 conformance plus 189 differential**. Nothing is
blocked.

`Core\Random` is finished as a depth target. Its last unasserted rule was the one
`owned_value_at` is written around — a draw retains the subject's *own* entry rather than copying
it — and it needed a non-scalar element to be visible at all, since copying and sharing are
indistinguishable over a scalar. The new case draws objects with `pick`, `sample` and `shuffle`
and asserts identity two ways: `==` against the subject's entries and against field-identical
twins (ADR 0090 makes `==` over objects identity, so the twin count is the other side of the
bound), and a write through a drawn handle landing on the subject. It is fully deterministic —
every count is exact whichever entries were drawn.

`Core\Debug` was `gaps.py`'s thinnest class and its gap was an *agreement*, not an edge:
`array_node`'s doc comment says out loud that its list-vs-map reading of the keys is the one
`Core\Json::encode` already makes, and nothing asserted the two agree. The new case asks all
three of `Debug::render`, `Json::encode` and `Arr::isList` the same eight subjects — including
keys `0, 1` written out of order and a hole — and reports whether they *agreed*, with the
disagreement count taken by `Arr::diff` rather than read off the line.

Two spellings worth having: a `Core\Cli\Text` converts with `as string`, which is what lets a
rendering be asked a question instead of only shown; and a `.nvst` helper taking `array<int>`
takes a bare literal at the call site fine, while `Arr::diff`'s *second* argument needs a
declared binding (the array-literal `array<mixed>` trap, already in the playbook).

The gap eight handoffs back still stands: **no `Core` class reaches `nvs_hir::implements_interface`**,
so `Core\Uri::compareTo` exists while `$a < $b` over two `Uri`s is `E0411`. It is in the backlog
and still deserves a session of its own.

## Next group

**`Core\Bytes`, `gaps.py`'s next thinnest with a floor of 2** — one file set:
`crates/nvs-stdlib/src/bytes.rs` and `tests/conformance/core/`. Read the member doc comments
first; over `Core\Random` and `Core\Debug` alike the gap was a rule stated in a doc comment that
no case could observe, never a missing row.

- [ ] **`startsWith` and `endsWith` at their empty and full-width ends**
      (`crates/nvs-stdlib/src/bytes.rs:619` and `:629`) — the *bound on both sides* shape: an
      empty needle, a needle the length of the subject, and one byte longer than it, asked of
      both members so they agree at each end rather than each being plausible alone.
- [ ] **`at` at the ends of its index, and against `slice`**
      (`crates/nvs-stdlib/src/bytes.rs:490`, `slice` at `:519`) — index `0`, the last byte, and
      the first refused one; then the *agreement*: `at($b, $i)` and the one-byte `slice($b, $i, 1)`
      answer the same byte for every `$i` across a sweep, counted.
- [ ] **Re-run `python tools/gaps.py` and take the next class it ranks** — the rank moves once
      these land.

## Backlog

- No `Core` class implements a Novis interface, so `Comparable` is unreachable from one — `docs/agent/handoff.md` has carried this eight sessions; it wants its own session (`nvs_hir::implements_interface`).
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs — `docs/agent/guard-name-debt.md`.
- `Core\Str` and `Core\Arr` each have members with a single case (`fold`, `graphemes`, `indexOf`; `column`, `flattenDeep`, `overlayDeep`) — `python tools/gaps.py`.
- 68 unasserted error paths, of which 65 are `Fault::fatal` and want judging before writing — `python tools/gaps.py --errors`.
- `csv.rs:512`'s thrown refusal is unreachable from source and is owed no case — `docs/agent/playbook.md`.
