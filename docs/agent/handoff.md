# Handoff

## State

**Stage 11 is closed: ADR 0116 § 2's live list and teardown sweep are on disk**, and the
acceptance check that had held the whole list to one check is closed with them.

- **Every object links into its context's live list** at `NvsObj::alloc` and out at
  `dismantle`. The list is its own allocation, held by the `Ctx` through an `Rc` and reached
  from the allocation path through a second thread-local beside `CURRENT`, because an object
  links itself in while a helper above it may be holding `&mut Ctx`.
- **The sweep frees only what it can show is unreachable.** "Whatever the drain left on the
  list" is not that set — `abi::call` answers a `Value` to its Rust caller, and every `Core`
  member returning an instance does the same — so the sweep tallies each member's references
  that come from another member's field slot, treats any member whose count that tally does not
  exactly account for as externally reachable along with everything under it, and dismantles
  only the remainder through `crate::release`'s one worklist. Freeing the list wholesale
  corrupted the heap across `-p nvs-stdlib`; this is priority 1 deciding it.
- **A survivor is detached before the list dies.** See the playbook bullet: this is the load
  bearing half, not tidying.
- **`examples/cycles.nvs` is on disk and valgrind-clean** (`tools/leak-check.sh`, run over it
  and over `objects.nvs`/`serialize.nvs`). A cycle closed through an `array<T>` element rather
  than a field slot still survives, which is a named known gap in `nvs-runtime`'s `lib.rs` where
  "no cycle collector" used to be.

The mechanism's one home is `crates/nvs-runtime/src/object.rs`'s module doc and `sweep`'s own
comment; ADR 0116 § 2 carries the decision and the priority-1 argument.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's
gate over a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot
pass inside this goal and is not a regression.

**`cargo deny check`'s `advisories` leg is red on a yanked `chacha20`** that predates this goal
and arrives through `rand`; the other three legs are green. See the playbook bullet.

## Next group

**`[log] target` is the last owed piece of stage 7 — the floor and `Core\Log::write` are already
one serialiser and are still two destinations. The file set is `crates/nvs-runtime/src/ctx.rs`,
`crates/nvs-runtime/src/floor.rs`, `crates/nvs-stdlib/src/log.rs` and
`crates/nvs-config/src/tree.rs`; ADR 0092 § 2 and ADR 0020 § 6 specify it.**

- [ ] **One place reads `[log] target`, and both writers reach it.** `Core\Log::write` writes
      through `ctx.write_output` and the floor writes its own way; the destination the config
      names — `stderr`, `file:<path>` or `syslog` — is read nowhere.
      `crates/nvs-runtime/src/ctx.rs:257` is the doc comment that already says what `file:…`
      selects "once something reads that"; `crates/nvs-stdlib/src/log.rs:202` and
      `crates/nvs-runtime/src/floor.rs:172` are the two call sites.
- [ ] **An unspelled target is refused where it is written, not where it is used.** A
      `[log] target` naming neither `stderr` nor `file:<path>` nor `syslog` is a configuration
      diagnostic in `crates/nvs-config/src/tree.rs:1`, beside the directives it already reads,
      rather than a run-time surprise on the first record written.
- [ ] **A case that both writers land in the target the deployment named** — ADR 0092 § 6's
      "two writers, one destination", asserted by writing one record from each and finding both
      in the file. `tests/conformance/error/the-log-floor-and-core-log-agree-on-shape.nvst:1` is
      the case they already agree on shape in.

## Backlog

- Stage 10's `every_part_two_spec_member_is_registered` waits on goal 6's spec §§ 15-19 —
  `docs/agent/loop-goal.md` § *Stage 10*.
- A cycle closed through an `array<T>` element is still leaked at teardown —
  `crates/nvs-runtime/src/lib.rs` known gap 7.
- The in-flight collector for a CLI script that builds cycles *between* teardowns stays open —
  `docs/agent/loop-goal.md` § *Stage 11*.
- `cargo deny check`'s yanked `chacha20` advisory — `docs/agent/playbook.md`.
