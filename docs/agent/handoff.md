# Handoff

## State

**Goal 6, M7. The stage-5 acceptance failure is closed.**
`body_stream_is_exclusive_with_body_and_with_files` is written and green in
`crates/nvs-stdlib/src/request.rs`, so all four names under the
`nvs-stdlib (ADR 0105 §§ 1-3 …)` check now exist. It asks the pair its twin
`a_body_is_claimed_by_the_member_that_read_it_and_refused_to_the_other` leaves unasked —
`bodyStream` against `files`, both directions, **with no chunk ever pulled** — which is where an
implementation claiming at the first pull rather than at the naming would let both through.

**Bookkeeping the next session may want:** that check's own comment says a landed name moves up into
the sibling `nvs-stdlib (ADR 0105 -- what is landed)` check; all four have been landed for a while
now and none was moved. Both `docs/agent/loop-goal.toml` and `docs/agent/goals/6-server.toml` would
have to change together. Nothing fails while it is undone. That comment also cites "§§ 3, 6-8" of
ADR 0105, which has six sections — the exclusivity rule's home is spec § 15.

**ADR 0086 § 6's union is in the corpus.** Two cases landed:
`command-run-converts-a-union-of-literals-argument-by-its-word` and
`command-run-refuses-a-union-argument-outside-its-set`, the second reading the refusal off standard
error so that "names every accepted word" is asserted rather than only the status. The group's third
item — *a subset of an enum's cases still refuses to convert* — **was already on disk** under
`command-run-throws-for-a-parameter-no-argument-converts-into.nvst`, which pins exactly that
`Level::Quiet|Level::Loud` parameter and its `LogicError`. No new case was written for it.

**§ 1's `Core\Session` class is still not on disk**, unchanged: `crates/nvs-stdlib/src/session.rs`
has the key, the drawn identifier, `load`/`save` and the three directives, and there are no registry
rows, no cards and no `.nvst` cases. That module's own doc says so, and it is the next group.

## Next group

**ADR 0139 § 1's seven members, over one file set:** `crates/nvs-stdlib/src/session.rs`,
`crates/nvs-stdlib/src/registry.rs` and `tests/conformance/core/session-*.nvst`. The member list is
`docs/spec/01-core-library.md:1093`; the five edits a `Core` member owes are conventions.md's.

- [ ] **`Core\Session::start` opens the record the store issued** (ADR 0139 §§ 1-2) — the drawn
      identifier is `crates/nvs-stdlib/src/session.rs:149` and the read is
      `crates/nvs-stdlib/src/session.rs:173`; the class joins `crates/nvs-stdlib/src/registry.rs:1224`,
      and the cookie the response carries is `crates/nvs-stdlib/src/session.rs:144`.
- [ ] **`get`, `set` and `remove` over the started record** (ADR 0139 § 1) — the write-back is
      `crates/nvs-stdlib/src/session.rs:189` and the key it is stored under is
      `crates/nvs-stdlib/src/session.rs:98`; § 4's last-write-wins is why no lock is taken between
      the read and the write.
- [ ] **`clear`, `regenerate` and `destroy`** (ADR 0139 §§ 1, 4) — `regenerate` draws a second
      identifier at `crates/nvs-stdlib/src/session.rs:149` and `destroy` expires the entry the store
      holds through `crates/nvs-stdlib/src/session.rs:189`'s neighbour `set_expiring`.
- [ ] **Three `.nvst` cases per member the conformance floor asks for**
      — the floor is `crates/nvs-stdlib/tests/conformance_coverage.rs:155` and the per-member
      reachability check beside it is `crates/nvs-stdlib/tests/conformance_coverage.rs:52`; a
      session case configures a backend, so
      each is a multi-file case, and the playbook's *Writing a test case* bullets own both traps.

## Backlog

- Raw/unparsed body access for an arbitrary content-type — ADR 0024 *Revisiting*, narrowed by m7.md.
- Move ADR 0105's four landed test names into the "what is landed" check — `docs/agent/loop-goal.toml`.
- The `[session] backend = "db"` tier stores nothing yet — `crates/nvs-stdlib/src/session.rs`'s doc.
- ADR 0086 § 6's `Core\Uuid` and `decimal` arguments have corpus; the options bag does not —
  `crates/nvs-stdlib/src/command.rs`.
