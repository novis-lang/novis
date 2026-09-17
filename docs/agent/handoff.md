# Handoff

## State

**Goal `decided-closures`, stage 4 — the library.** The prepared-pattern channel carries a second
grammar. `Core\Time\DateTime::format` and `Core\Time::parse` are on
`crates/nvs-stdlib/src/registry.rs:3204`'s `PREPARED_MEMBERS` roster beside `Core\Regex::compile`, so
each takes `crate::cldr::PREPARED_PATTERN` as argument 0 where its pattern was written at the call
site and `PREPARED_NONE` where the program assembled one.

**What the word buys is the compile itself, not a routing decision.** `cldr::compiled` keys a
per-core cache on the pattern text and **only a prepared pattern is ever keyed there**: what a
program writes is a set fixed when it was compiled, while what a request assembles is not, so the
bound stays O(written patterns per core) and a request cannot mint an entry. `cldr.rs` gap 1 and
`time.rs` gap 1 are struck, and each module's own docs now state that split and what it spends.

**The channel needed a descriptor, not plumbing.** `PreparedFact::CldrPattern` (nvs-types) and
`Prepared::CldrPattern` (nvs-ir) are the two ends; `intrinsics.rs`'s `Grammar::DateFormat` arm
records on success where it previously only refused. `crates/nvs-ir/tests/prepared_patterns.rs`
holds the end-to-end claim for both call paths — the static member and the instance one, whose word
is emitted ahead of the receiver. **The goal's reserved ADR slot is unspent**, and nothing here needs
it: the record it would have carried is the rule's own paragraph.

**`rule:expressions/intrinsic-literals` said the derived artifact is stored with the unit**, which
no row has ever done. Amended to what the code does, under the goal's standing decision that tested
code beats a record: a *fact* crosses with the unit, the artifact belongs to the core that builds
it. The cross-request accounting question both caches raise has one home,
`crates/nvs-stdlib/src/regex.rs:69` gap 1, which now names the CLDR one too.
`python tools/owners.py --closes decided-closures` names 17 gaps, down from 19.

## Next group

**Stage 4: `Core\Reflect`'s two gaps** — one file set: `crates/nvs-stdlib/src/reflect.rs` and its
cases under `tests/conformance/core/`. Both are decided and neither needs another crate.

- [ ] **`crates/nvs-stdlib/src/reflect.rs:126` — gap 1: § 1's roster is short of four of the classes
      it names.** `METHOD_INFO` and the cards beside it are the shapes the missing ones take
      (`rule:tooling/reflection-and-source-parsing-are-core-features`), and
      `conventions.md` § *A `Core` member* is the five edits each costs.
- [ ] **`crates/nvs-stdlib/src/reflect.rs:139` — gap 2: a `protected` member is not reached
      reflectively from the declaring class's own body.** The visibility test is the member's, not
      the caller's, today (same rule).

## Backlog

- The cross-request compiled-pattern caches are bracketed by nothing — `crates/nvs-stdlib/src/regex.rs:69` gap 1, waiting on the request arena.
- 15 further `decided-closures` gaps, one per line of `python tools/owners.py --closes decided-closures`.
- `crates/nvs-stdlib/src/test.rs:94` and `:102` — two gaps in one file, the next coherent group after `reflect.rs`.
- `crates/nvs-types/src/defaults.rs:58` gap 1 — a named constant as a parameter default.
