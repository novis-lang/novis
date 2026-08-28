# Handoff

## State

**M4's Stage 8, depth.** The tree is at **813 conformance plus 189 differential**. Nothing
is blocked.

`Core\Encoding` is **done** as a depth target — `gaps.py` now ranks it 6.0/floor 4, out of
the thin end entirely. The three cases this session added are all sweeps over the whole
class rather than more rows: the three encoder/decoder pairs round-trip one table of
eleven buffers with the agreements *counted*, and `toBase64Url`'s output is pinned as
`toBase64`'s with `+`→`-`, `/`→`_` and the padding trimmed; each decoder's two independent
bounds are named on both sides — the symbol counts a partial group admits (base32
2/4/5/7/8 against 1/3/6, base64url 2/3/4 against 1 and 5, base64 multiples of four only)
and RFC 4648 § 3.5's canonical trailing bits, counted over each alphabet in full (4 of 64,
16 of 64, 8 of 32); and the empty subject crosses all eleven members, counted, with the
one-octet buffer beside it where base32 spends six of eight symbols on padding.

Two spellings confirmed on scratch runs: `Core\Str::replace($s, "+", "-")` takes its
option bag optionally, and `Core\Str::trimEnd($s, {characters: "="})` is the `rtrim` half
of the URL-safe idiom. The `Core\Str::slice` length/offset trap is now a playbook bullet.

The gap five handoffs back still stands: **no `Core` class reaches
`nvs_hir::implements_interface`**, so `Core\Uri::compareTo` exists while `$a < $b` over two
`Uri`s is `E0411`. It is in the backlog and still deserves a session of its own.

## Next group

**`Core\Test`, the thinnest class `gaps.py` now ranks** (depth 3.0, floor 2, 30 cases over
9 members) — one file set: `crates/nvs-stdlib/src/test.rs` and `tests/conformance/core/`.
Nothing here needs `encoding.rs` open.

- [ ] **`Core\Test::assertThrows` is bounded on both sides of the class it matches**
      (`crates/nvs-stdlib/src/test.rs:430`) — one case. The throw it accepts and the
      nearest one it does not, named together: the declared class exactly, a class the
      body does not throw, and a body that throws nothing at all. Read the member's own
      arity (`args: [3]`) first — the third slot is the option bag, and what it holds
      decides whether a subclass matches.
- [ ] **The ledger counts one entry per assertion whatever the outcome**
      (`test.rs:238` `assertSame`, `test.rs:365` `assertCount`, `test.rs:505`
      `assertDoesNotThrow`) — one case, the *agreement* shape. The same question asked of
      three members that share the ledger, asserted by counting entries rather than by
      reading one member's tally, so a member that records its pass and forgets its
      failure fails here while looking right on its own line.
- [ ] **The degenerate subject through each assertion** (`test.rs:365` `assertCount`) —
      one case: the empty container, the zero count, and `assertCount` against a subject
      that is not a container at all, which is where its `Fault::` constructor decides
      whether a case can catch it.

## Backlog

- **No `Core` class reaches `nvs_hir::implements_interface`** — `Core\Uri::compareTo`
  exists but `$a < $b` over two `Uri`s is `E0411`; `docs/agent/loop-goal.md` § *Standing
  decisions* leaves it open and it wants its own session.
- `Core\Random` (depth 3.0, floor 2) and `Core\Debug` (3.0, floor 2) are the next two thin
  classes after `Core\Test` — `python tools/gaps.py` ranks them.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md` is the list.
- `crates/nvs-stdlib/src/csv.rs:512`'s `thrown` is unreachable from source until ADR 0007
  § 2's `array<T> as array<U>` row lands — the playbook owns why.
