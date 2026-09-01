# Handoff

## State

**Stage 11 is closed, including the half the previous handoff claimed and had not landed**:
ADR 0116 § 2's live list and teardown sweep were on disk, but item 36's *crossing relink*
and its debug owner stamp were not, and neither were the two tests
`docs/agent/loop-goal.toml`'s stage-11 check names. All three are now, and
`examples/cycles.nvs` prints `built=1000 / crossed=ok / done` — the acceptance check that
had been failing since session 0007.

- **The relink lives inside `Live::adopt`** (`crates/nvs-runtime/src/graph.rs:415`) and at no
  call site, because adopting *is* the allocation changing owners. Its destination is the
  context running at the crossing, which is the receiving one on the way out of an isolate;
  on the way **in** there is no destination context yet, so an adopted argument stays on the
  parent's list, which is safe only because a parent outlives its child. `object.rs`'s
  module doc is the home of both halves.
- **The debug stamp asserts a structural invariant, not an ownership one**: an object is
  linked on the list its `ObjHeader::owner` names, checked in `dismantle` against the
  object's list neighbour and in `sweep` against the list itself. It deliberately does *not*
  assert that the context taking an object apart is the one that made it — see the playbook
  bullet, which cost this session two false positives and one abort. `loop-goal.toml`'s
  check comment said the stronger thing and has been corrected in both copies.
- **`FIELDS_OFFSET` now differs between profiles** — five words in debug, four in release.
  Nothing outside `nvs-codegen` reads it, and it asks this constant.
- **An object crosses an isolate boundary outward only if it crossed inward first**; the
  playbook bullet has the mechanism. `examples/cycles/child.nvs` is built around it.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's
gate over a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot
pass inside this goal and is not a regression. `cargo deny check`'s `advisories` leg is red
on a yanked `chacha20` that predates this goal; the other three legs are green.

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

- A cycle closed through an `array<T>` element rather than a field slot still survives the sweep —
  named known gap in `crates/nvs-runtime/src/lib.rs`.
- An in-flight collector for a long-running CLI script that builds cycles *between* teardowns is a
  separate open decision — `docs/agent/loop-goal.md`, stage 11's header.
- Making an object cross *outward* means sharing a class table between compiled units —
  `crates/nvs-runtime/src/graph.rs`'s `Live::admit` names the cost.
- `cargo deny check` advisories: a yanked `chacha20` arriving through `rand`.
- `every_part_two_spec_member_is_registered` needs spec §§ 15-19, which are goal 6's.
