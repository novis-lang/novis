# Handoff

## State

**Stage 6's local tier is bounded.** `[cache.local] max_size` is a `System`/`Reload` directive
(`crates/nvs-config/src/directive.rs`, `crates/nvs-config/src/tree.rs`), and
`nvs_stdlib::cache`'s `Local::put` forgets the key written longest ago until an arrival fits —
never refusing, which is ADR 0059 § 3's "evicts rather than failing an allocation" over § 1's
"absent at any time". The shipped cap is `DEFAULT_MAX_SIZE`, 32 MiB per core; `nvs_config::Quantity`
is the parser, so `32M` cannot come to mean two things.

**Both writers of that tier are now bounded by one number.** `Core\Cache::local` and
`Core\RateLimit::shed` pass the same `cache::local_cap(ctx)`, which is what the `None` rows in
`registry::CAPABILITIES` claimed bounds them. The eviction order holds one slot per *live key*, not
one per write, so `shed` rewriting a single key every request is O(in-flight) as `AGENTS.md`
requires. The reasoning, the cost and what is still absent (a TTL, a `forget`) are that module's
doc comment, which is their one home.

**The driver's acceptance stops on stage 7, and it is not a regression**: `examples/logging.nvs`
wants `Core\Log::write` and no `Core\Log` is registered. That is the group below, and it is the
last thing between this goal and its acceptance.

## Next group

**Stage 7's `Core\Log`, over `crates/nvs-stdlib/src/log.rs` (new),
`crates/nvs-stdlib/src/registry.rs` and `crates/nvs-stdlib/src/fatal.rs`.**

- [ ] **Register `Core\Log` and the `Core\Log\Level` enum** — rows, cards, bodies and `address()`
      arms, in a module beside its ADR 0020 sibling `crates/nvs-stdlib/src/fatal.rs:1`. The class
      row goes with that sibling's at `crates/nvs-stdlib/src/registry.rs:1169`, and because the
      class is door-bearing or is not, ADR 0118 § 7 owes it a row either way at
      `crates/nvs-stdlib/src/registry.rs:1312` — a `Cap` if writing the log is an effect a grant
      names, an explicit `None` beside the comment saying what it reaches instead if it is not.
- [ ] **`write` reaches the engine floor's own serialiser rather than a second one** — ADR 0020
      § 1, and the goal's standing decision that two writers agreeing today is the bug. The floor's
      side is `crates/nvs-stdlib/src/fatal.rs:1`; a record a program writes and one the engine
      writes have to be schema-identical because they are one walk, not because a test compares
      them.
- [ ] **`Core\Log::write` refuses a `secret` argument and accepts `tainted` freely** — ADR 0033 is
      an output sink's axis and a logged field is *data* (`examples/logging.nvs:12` says so in the
      fixture's own words). The two axes live at `crates/nvs-types/src/expr/quals.rs:1`.
- [ ] **`examples/logging.nvs` green on its three frozen lines** — `docs/agent/loop-goal.toml:2455`
      holds them, and the fixture's child and handler scripts are already on disk
      (`examples/logging/throws.nvs`, `examples/logging/handler.nvs`), so nothing but the class is
      missing.

## Backlog

- A TTL and a `Core\Cache\Store::forget` — `crates/nvs-stdlib/src/cache.rs`'s *What is not here yet*.
- A `[cache.shared]` password, database index or TLS — same module doc, same section.
- Neither `[cache.local]` nor `[cache.shared]` appears in `docs/novis.md` C.2's `nvs.toml` tour.
- `orient.py` printed no map line for `nvs-config/src/value.rs`, which is where `Quantity`/`Unit`
  live and the one parser every size directive shares — `[context] modules` wants it.
- Goals 4 and 5 still need a reachable Docker daemon — `docs/implementation-plan.md`'s *Blocking*.
