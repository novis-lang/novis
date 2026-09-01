# Handoff

## State

**Stage 12 is closed**: ADR 0127's `Core\Script::onExit` is on disk end to end — the queue on `Ctx`,
the member and the report in `nvs-stdlib`, the three endings wired in `nvs-cli`, five named tests,
three conformance cases and `examples/onexit.nvs`, which prints `work done / onExit: Normal 0 /
second hook`. That fixture was the acceptance check that had been failing since session 0008 and
holding the whole check list to one check an iteration.

- **The routing lives in `nvs_stdlib::script::run_exit_hooks`, not at the call sites.** It turns the
  script frame's ABI status into § 2's report and answers early for a `FATAL`, so § 3's "a limit
  breach runs no hook" is one branch rather than a rule three `nvs-cli` arms remember. The module
  doc is the home of that split; `Ctx::exit_hooks` is the home of the queue's spend.
- **On the throw path the queue runs after the whole failure report** — tier 2, tier 3 and the floor
  — not just after tier 2. § 4's reason for ordering the hooks first is that "a misbehaving queue
  cannot starve the failure report", and the record *is* that report. `main.rs`'s comment says so.
- **`exit` inside a hook is refused in the drain, not at the call site.** `Ctx::abandon_exit_hook`
  turns `Err(EXITED)` into the `RuntimeError` § 5 names and continues the queue; a hook cannot
  therefore suppress the hooks behind it, which is the property that section is protecting. A
  `catch` written *inside* the hook does not see it — closing that gap means a check on the `exit`
  lowering, and nothing asks for one yet.
- **ADR 0127 § 1 now spells three accessors** rather than the three properties it sketched: a
  `Core`-owned instance has no property a program can reach. The playbook bullet has the general
  form.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside
this goal and is not a regression.

## Next group

**`[log] target` is the last owed piece of stage 7 — the floor and `Core\Log::write` are already one
serialiser and are still two destinations. The file set is `crates/nvs-runtime/src/ctx.rs`,
`crates/nvs-runtime/src/floor.rs`, `crates/nvs-stdlib/src/log.rs` and
`crates/nvs-config/src/tree.rs`; ADR 0092 § 2 and ADR 0020 § 6 specify it.**

- [ ] **One place reads `[log] target`, and both writers reach it.** `Core\Log::write` writes through
      `ctx.write_output` and the floor writes its own way; the destination the config names —
      `stderr`, `file:<path>` or `syslog` — is read nowhere.
      `crates/nvs-runtime/src/ctx.rs:257` is the doc comment that already says what `file:…` selects
      "once something reads that"; `crates/nvs-stdlib/src/log.rs:202` and
      `crates/nvs-runtime/src/floor.rs:172` are the two call sites.
- [ ] **An unspelled target is refused where it is written, not where it is used.** The directive is
      is `crates/nvs-config/src/tree.rs:353`'s `pub target: Option<String>`, and a `file:` with no
      path or a fourth spelling is a configuration diagnostic there rather than a run-time surprise
      on the first record.
- [ ] **A case that both writers land in the target the deployment named** — ADR 0092 § 6's sameness
      claim, asked of the destination rather than of the shape: one `.nvst` case over
      `crates/nvs-stdlib/src/log.rs:202` and the floor's own path.

## Backlog

- Stage 10's `every_part_two_spec_member_is_registered` needs spec §§ 15-19 — goal 6's, not this one.
- `cargo deny check`'s `advisories` leg is red on a yanked `chacha20` that predates this goal.
- `orient.py` warns that `[context] modules` names `crates/nvs-stdlib/src/fatal.rs` and
  `.../script.rs` as patterns that match no module — both files exist and are printed, so the glob
  in `docs/agent/loop-goal.toml` is wrong rather than the tree.
- An `exit` inside an exit hook is not catchable at its own call site — `docs/adr/0127` § 5's
  spelling, if it ever needs to be exact.
