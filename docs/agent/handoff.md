# Handoff

## State

**Goal `decided-closures`, stage 4 — the library.** A union parameter now carries a classification:
`CoreTy::classification` folds a union's arms, so the mark is read off the arm that has a cell for it
(`crates/nvs-stdlib/src/registry.rs:1155`). `qual_of` needed no change — it already delegates — which
is the decided answer to `crates/nvs-stdlib/src/regex.rs` gap 1 reached from the other side: the
registry's own gate has always *required* the mark on the arm, so the arms were the slot.

**`Core\Regex`'s `Pattern|string` positions refuse a tainted pattern by `Qual::Sink` now**, the one
written on `PATTERN_OR_STRING`'s text arm, rather than by the refusing default. `compile`'s own
`string` parameter and every union beside it read the same way, and the new registry test
`a_union_parameter_carries_the_classification_its_arms_declare` holds both halves.

**Seven parameters widen, and that is the change with teeth.** A union whose arms declared `Neutral`
or `Contagious` admitted nothing before; it does now — `Core\Cli::write`, `Core\Compress`'s
`bytes|string` pair, `Arr::hasKey`, `Arr::column`, `Db\Rows::column`, `Regex\Match::group`,
`Response\Stream::write`. Each mark was written by the row's author and ignored by the checker;
`cli.rs:428`'s `WRITABLE` doc argued for exactly this admission. The `secret` axis is untouched.
The two comments that argued from the old `None` (`socket.rs:267`, `http/socket.rs:198`) and the card
that promised a refusal (`response.rs:573`) say what is true now.

**`rule:security/unclassified-parameter-refuses-tainted` gained the union paragraph**, which is its
home. `rule:security/jwe-compact-subset`'s "a union carries no classification" clause was a mechanism
claim, not the decision, and now gives the reason that survives it. ADR 0179 stays frozen.
`python tools/owners.py --closes decided-closures` names 19 gaps, down from 20.

## Next group

**Stage 4: the prepared-pattern channel gains its second and third callers** — one file set:
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-stdlib/src/cldr.rs`, `crates/nvs-stdlib/src/time.rs`,
and `nvs-ir`'s lowering. The channel is **already built** for `Core\Regex::compile`, so this is a
roster entry and a second descriptor rather than new plumbing: `crates/nvs-ir/tests/prepared_patterns.rs`
is what it looks like working, and the goal's reserved ADR slot may end up unspent.

- [ ] **`crates/nvs-stdlib/src/registry.rs:3198` — `PREPARED_MEMBERS` is one row, and the CLDR
      pattern members are the next ones.** What a prepared argument is and how it reaches the helper:
      `crates/nvs-ir/src/ir.rs:1851` and `crates/nvs-ir/src/lower/mod.rs:2222`
      (`rule:expressions/intrinsic-literals`).
- [ ] **`crates/nvs-stdlib/src/cldr.rs:212` — a literal date pattern is prepared while checking.**
      The same work `cldr`'s `compile` does, moved off the request path, with the diagnostic becoming
      a compile error. Stage 4's check
      `a_literal_cldr_pattern_is_prepared_at_compile_time_and_not_per_call` is this item's test.
- [ ] **`crates/nvs-stdlib/src/time.rs:102` — `$d->format` and `Core\Time::parse` read that same
      prepared pattern.** One gap with the one above; the module gains a caller and nothing else.

## Backlog

- Stage 4's other two checks are goal items not yet taken — Zip64/CRC/seeded/UUID/UNC/EBML, and JSON
  depth on a heap stack plus the queue's boot refusal (`docs/agent/loop-goal.toml:11607`, `:11625`).
- `crates/nvs-stdlib/src/regex.rs:69` gap 1 (the cache's accounting bracket) waits on the request
  arena and is the only gap that file still owns.
- `[context] modules` was left alone on purpose: the driver's `context-sync.py` sweeps it from the
  paths a session's own commits touched.
