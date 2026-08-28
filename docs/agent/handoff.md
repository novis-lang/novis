# Handoff

## State

**M4's Stage 8, depth.** The tree is at **845 conformance plus 189 differential**, all green.
Nothing is blocked.

`Core\Json` was `gaps.py`'s floor-2 class and is now floor 3 at depth 5.0 (57 cases over 4
members); it is no longer among the thin ones. Two members gained a case each, both in
`tests/conformance/core/`:

- **`decodeAs` admits exactly the wire spellings its field's declared type names.** One
  single-field class per codec type crossed with every spelling JSON has, rendered as a grid, then
  the three containments counted over the whole sweep rather than read off a row: `uint` ⊂ `int` ⊂
  `float` ⊂ `mixed`, each strict at a named spelling (`-1`, `1.0`, `[]`). `null` is the one
  spelling even a `mixed` field refuses, because ADR 0071 § 4's nullability check runs ahead of the
  type switch. `uint`'s bound is named at both ends and the ceiling is the surprise: the type holds
  2^64−1 but `visit_u64` refuses a literal past `i64::MAX` before any field is looked at, so that
  refusal carries an **empty** issue path while the `-1` one carries `v` — a refusal names the
  field only when it got as far as one.
- **`isValid` is `decode` at the *default* depth, and nowhere else.** Over a nine-document window
  either side of 512 it agrees with option-free `decode` 9 times out of 9, and then splits from it
  in *both* directions — five documents `decode` accepts at `maxDepth: 1024` that `isValid` calls
  invalid, four it refuses at `maxDepth: 100` that `isValid` calls valid. That is the pinning the
  helper's doc comment claims and that an agreeing sweep alone cannot distinguish from "`isValid`
  is `decode`". Its own bound is asserted at 511/512 brackets rather than only at the far-away 900
  the cap case uses.

Item [3] of the previous group, `Core\Json::decode` (now 6 cases), was **not taken**: the class had
left the thin list by then and `Core\Random` is a better spend. `orient.py`'s pack was complete for
this group; nothing outside it was read.

The gap nineteen handoffs back still stands: **no `Core` class reaches
`nvs_hir::implements_interface`**, so `Core\Uri::compareTo` exists while `$a < $b` over two `Uri`s
is `E0411`. It is in the backlog and still deserves a session of its own.

## Next group

**`Core\Random`**, now `gaps.py`'s thinnest class — floor 2, depth 4.0, 10 cases over 7 members.
One shared file set: `crates/nvs-stdlib/src/random.rs` plus `tests/conformance/core/`. A random
member's case asserts a *property over a sweep*, never a value, so read the existing cases'
`--TEST--` lines first for which properties are already counted.

- [ ] **`Core\Random::float`** (2 cases) — row `random.rs:87`, helper `random.rs:285`. The likely
      gap is *a bound asserted on both sides*: it takes no argument, so the whole of its contract is
      the half-open interval, and `[0, 1)` is two claims — nothing below 0, and 1.0 itself never
      produced — neither of which one draw can show. Count over a sweep.
- [ ] **`Core\Random::token`** (3 cases) — row `random.rs:101`, helper `random.rs:374`. *Agreement*:
      its length and alphabet are a function of the requested size, so every draw at a given size
      agrees on both while agreeing on nothing else. The `0` size is the bound's other half.
- [ ] **`Core\Random::bytes`** (4 cases) — row `random.rs:94`, helper `random.rs:316`. Take it only
      if the first two left you well short of the ceiling: same shape as `token` minus the alphabet,
      and a `uint` size means the upper bound is a refusal rather than a value.

## Backlog

- No `Core` class reaches `nvs_hir::implements_interface`; `$a < $b` over two `Uri`s is `E0411` —
  ADR 0013, and a session of its own.
- 54 of `loop-goal.toml`'s 156 guard tests match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
- `Core\Json::decode` (6 cases) and `encode`'s option rows — `crates/nvs-stdlib/src/json.rs`.
- ADR 0007 § 2's `array<T> as array<U>` row panics `nvs-ir`, which is why a case cannot index an
  `array<mixed>`'s elements — playbook, *Writing a test case*.
- `nvs_stdlib::json` gap 6: an `Opaque` codec field is a `Fault::fatal` no case can reach.
- 68 unasserted error paths, 65 of them `Fault::fatal` — `python tools/gaps.py`.
