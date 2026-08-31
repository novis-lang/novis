# Handoff

## State

**Stage 6's members are all four on disk.** `Core\Cache::local`/`::shared` and
`Core\RateLimit::consume`/`::shed` are rows, cards, bodies and arms, and `examples/cache.nvs` still
prints its five frozen lines against `tests/db/compose.yaml`'s `redis`.

**A member that reaches nothing now declares that, and there is no allowlist left.**
`registry::CAPABILITIES` is `(class, member, Option<Cap>)`: `Core\Cache::local` and
`Core\RateLimit::shed` carry `None` rows beside the comment saying what they reach instead, the
frozen `NEEDS_NO_CAPABILITY` beside `crates/nvs-stdlib/tests/capability.rs` is deleted, and
`every_capability_bearing_member_declares_its_capability` now claims that *every* member of a
door-bearing class owes exactly one row. ADR 0118 §§ 3 and 7 carry the decision — declaring nothing
costs what declaring `fs.read` costs, so the one move § 7 forbids has no cheap spelling left. The
table is still audit data that grants nothing, so nothing about enforcement moved.

**`shed`'s state is `Core\Cache`'s local tier, not a map of its own** — one per-core store means one
thing to bound, and ADR 0059 § 3's cap will bound both by bounding it. That cap is **not on disk**,
so the local tier and `shed` both grow with distinct keys per core; it is the first item below and
`AGENTS.md`'s O(in-flight) rule is what makes it the first. `crates/nvs-stdlib/src/ratelimit.rs`'s
module doc is the home of that decision, of why the clock is monotonic, and of what each tier spends.

**The driver's acceptance stops on stage 7, not on a regression**: `examples/logging.nvs` wants
`Core\Log::write` and no `Core\Log` exists in `nvs_stdlib::registry` yet. That is an item still open,
and it is the group after the two below.

## Next group

**The local tier's cap, over `crates/nvs-stdlib/src/cache.rs`, `crates/nvs-config/src/tree.rs` and
`crates/nvs-config/src/directive.rs`.**

- [ ] **A `[cache.local]` cap in `nvs.toml`, and eviction rather than a failed allocation** — ADR
      0059 § 3. The block goes beside `[cache.shared]` at `crates/nvs-config/src/tree.rs:600` with
      its `Directive` row at `crates/nvs-config/src/directive.rs:127`; the store it bounds is
      `crates/nvs-stdlib/src/cache.rs:333` and the write to charge is
      `crates/nvs-stdlib/src/cache.rs:344`. Both `Core\Cache::local` and `Core\RateLimit::shed` write
      there, so the cap closes both — and the `None` rows in `registry::CAPABILITIES` say in as many
      words that this cap, not a grant, is what bounds them.
- [ ] **A case that pins eviction as *forgetting*, not failing** — ADR 0059 § 1's "any entry may be
      absent at any time" is what makes a cap legal at all, and `crates/nvs-stdlib/src/cache.rs:353`'s
      `store_get` answering `None` is the whole contract. `crates/nvs-stdlib/src/ratelimit.rs:565`'s
      `stored_tat` reads the same absence as an arrival forgotten, which is § 1's approximation for
      `shed` and the reason `consume` may not live in that tier.

## Backlog

- Stage 7 opens next: `Core\Log` and `Core\Fatal`'s log half — `docs/agent/loop-goal.toml`'s stage 7
  checks and `examples/logging.nvs`.
- `Core\Cache::local`/`::shared` sit at the conformance floor of three with the shed cases now
  landed — `python tools/gaps.py --coverage` ranks what is thinnest.
- ADR 0118 § 7's wording now claims a total set; if a future class gains a member with no row, the
  message at `crates/nvs-stdlib/tests/capability.rs:290` is the one that has to teach it.
