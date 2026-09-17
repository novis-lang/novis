# Handoff

## State

**Goal `decided-closures`, stage 4 — the library — is under way, and stage 4's third check's first
test is green.** `rule:core-classes/regex-two-tiers`'s step budget is `[limits] max_regex_steps`, an
ordinary `Runtime` directive whose default is the constant `crates/nvs-stdlib/src/regex.rs` states,
so a request may widen or narrow its own and a `[limits.hard]` entry is how a host bounds that. The
rule's fragment says so now; the gap it recorded is deleted.

**The budget is part of this core's compiled-pattern cache key**, beside the text and the flags.
`fancy-regex` bakes the limit in when it builds the program, so a shared entry would run one request
under another's ceiling — `crates/nvs-stdlib/src/regex.rs`'s `CACHE` doc is the home of that and of
what it spends. `false` reads as the shipped budget: there is no spelling for a tier that may
backtrack forever.

**The carried floor check that had been red since session 0005 is repointed, not a regression in the
tree.** `Core\Queue\Stats` gained its fifth counter under this goal and `queue_sqlite.rs` renamed
its case with it, so the name goal `sqlite-queue` drafted named a test no crate declares.
`python tools/owners.py --closes decided-closures` names 20 gaps now, down from 21.

**`[context] modules` prints no `nvs-config` or `nvs-server` module**, though this item's own anchor
was `crates/nvs-config/src/tree.rs:171` — the map block said nothing about the file the work opened.
Adding `crates/nvs-config/src/*.rs` and `crates/nvs-server/src/schedule.rs` to that field is what
closes it.

## Next group

**Stage 4: the two patterns compiled per call, and the sink spelling regex left open** — one file
set: `crates/nvs-stdlib/src/cldr.rs`, `crates/nvs-stdlib/src/time.rs`,
`crates/nvs-types/src/core_lib.rs`.

- [ ] **`crates/nvs-types/src/core_lib.rs:377` — `qual_of` reads a parameter's declared `Qual`
      whatever its type.** The decided answer to `crates/nvs-stdlib/src/regex.rs:69` gap 1: the seven
      members taking `Pattern|string` refuse a tainted pattern today by the `None` default rather
      than by a rule a reader can find, and a `CoreTy::Union` has nowhere to hold a mark
      (`rule:security/regex-pattern-is-a-sink`). One registry-shape change, and the refusal becomes
      readable.
- [ ] **`crates/nvs-stdlib/src/cldr.rs:212` — a literal date pattern is prepared while checking.**
      Stage 4's third check's second test, `a_literal_cldr_pattern_is_prepared_at_compile_time_and_not_per_call`.
      The channel is the one `crates/nvs-stdlib/src/regex.rs:@prepared_tier` already runs on
      (`rule:expressions/intrinsic-literals`'s fold), and the goal's one ADR slot is reserved for it.
- [ ] **`crates/nvs-stdlib/src/time.rs:102` — `$d->format` and `Core\Time::parse` read that same
      channel.** The same gap in the other module, and the reason the ADR is one record rather than
      two: both members walk `cldr.rs`'s pattern grammar.

## Backlog

- 20 module-doc gaps still name this goal — `python tools/owners.py --closes decided-closures`.
- `crates/nvs-stdlib/src/regex.rs:83` gap 2 (the cache is a cross-request store) waits on the request
  arena, and is a deferral rather than a build — that module's own doc states what it costs today.
- `[context] modules` in `docs/agent/loop-goal.toml` names no `nvs-config` or `nvs-server` module.
- ADR 0192 is unclaimed, reserved by `docs/agent/loop-goal.md` § *Standing decisions* for the
  prepared-pattern channel.
