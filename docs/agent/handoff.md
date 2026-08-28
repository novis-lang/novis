# Handoff

## State

**M4's Stage 8, depth.** The tree is at **821 conformance plus 189 differential**. Nothing is
blocked.

`Core\Bytes`'s addressing half is now asserted as one rule rather than as five members that each
look right alone. Two cases landed, both over `crates/nvs-stdlib/src/bytes.rs`'s own doc comments:

- **The needle-length bound**, asked of `startsWith` and `endsWith` together. The existing sweep
  case agrees the two predicates with `slice`/`compare` over a table but never walks the length,
  so a member stopping one byte early agreed with a derivation making the same mistake. The new
  case makes the width the variable — `k` runs 0 … 9 against an 8-byte subject, with the k = 9
  needle cut from a *longer* buffer so it is a real 9-byte needle rather than a `slice` that
  stopped short — and adds the two full-width needles that differ in one octet at opposite ends,
  which is where a comparison of `len - 1` octets still answers `true`.
- **`at` against `slice`**, over a 19-wide index sweep. Where they agree, agreeing is the whole
  content of `at`; the agreement is checked by comparing `slice($b, $i, 1)` against
  `Core\Bytes::fill(1, <what at answered>)` rather than by reading the window back with `at`.
  The three indexes where they part are named one at a time, because the divergence is deliberate
  and of two different kinds: past the end `at` throws while `slice` is empty, and *before the
  start* `slice` clamps and answers the first octet at an index `at` refuses outright.

Both cases are exact — every count is fixed whichever octets the subject holds — and no `Fault`
in `bytes.rs` was reachable that a case does not already catch.

The gap ten handoffs back still stands: **no `Core` class reaches `nvs_hir::implements_interface`**,
so `Core\Uri::compareTo` exists while `$a < $b` over two `Uri`s is `E0411`. It is in the backlog
and still deserves a session of its own.

## Next group

**`Core\Uuid`, `gaps.py`'s thinnest remaining class with a floor of 2** — one file set:
`crates/nvs-stdlib/src/uuid.rs` and `tests/conformance/core/`. Four cases exist already
(`uuid-parses-only-the-canonical-form.nvst` and three others), so read the member doc comments
first and look for the rule stated there that no case can observe — over `Core\Random`,
`Core\Debug` and `Core\Bytes` alike the gap was never a missing row.

- [ ] **`tryParse` and `parse` are one reader with two answers** (`uuid.rs:334` and `:362`) — the
      *agreement* shape: every subject of one table asked of both, asserting `tryParse` is `null`
      exactly where `parse` throws, counted rather than read off a line.
- [ ] **`v7`'s layout, not just its ordering** (`uuid.rs:309`) — `uuid-v7-never-goes-backwards`
      covers the monotonic sweep; the version and variant nibbles a draw must carry, and that
      `v4` and `v7` differ in exactly that nibble, are the bound nothing asks about.
- [ ] **Re-run `python tools/gaps.py` and take the next class it ranks** — the rank moves once
      `Core\Uuid` lands; `Core\Hash\Stream` (floor 4 over 2 members) and `Core\Csv` (floor 5) are
      the next two whose whole class is one small file.

## Backlog

- No `Core` class reaches `nvs_hir::implements_interface`, so `Core\Uri::compareTo` exists while
  `$a < $b` over two `Uri`s is `E0411` — its own session (`docs/agent/loop-goal.md` item list).
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md` is the list and the three causes.
- `crates/nvs-stdlib/src/csv.rs:512` is the one `thrown` refusal unreachable from source and owed
  no case (`docs/agent/playbook.md`, *Divergences and refusals already pinned*).
- 65 unasserted `Fault::fatal` sites remain; `python tools/gaps.py --errors` judges each.
