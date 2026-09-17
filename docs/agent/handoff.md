# Handoff

## State

**Goal `unowned-closures`, stage 6 (the register).** `python tools/owners.py` reads 79 items with
`unowned: 2`, `untagged: 0`, `broken-tag: 0`, `unreasoned: 0`, and `--deferrals` green. Both remaining
items are in `nvs-stdlib`; `crates/nvs-db` owns no unowned gap.

**The span's debug-flag item is deferred to M10, and the reason is in the gap's own prose.**
`docs/plan/m10.md` states the scope — `Core\Debug`, the `[debug]` `nvs.toml` section and the
`debug.trace`/`debug.profile` capabilities — and that is the first point at which a program can ask for
`DebugFlags::TRACE` at all. `rule:testing/debug-mode-directive` arms the bit from `[debug] mode`, whose
directive row does not exist (`crates/nvs-config/src/tree.rs:425`), while the grant says where a trace
may be *written*: so reading the capability set into a request's `Ctx` today would gate a bit nothing
can turn on and leave the sampled path exactly as it is.

**The TDS `bytes` item took a chain entry.** Goal `tds-bytes` is goal 64, between `core-class-tests`
and `gap-zero` (now 65, `dossier` 66). It is a driver's encoder rather than a rule change for five
drivers, because the mechanism the gap called an open question is built: `TdsPlan`
(`crates/nvs-db/src/tds/plan.rs:38`) already carries the `@params` text a plan was compiled against and
makes a mismatch a miss that unprepares it, so ADR 0067 § 1's key does not move. Prose, manifest and
seed handoff are on disk, `chain.py --check` and `plan.py --check` are green, and the gap's
`docs/agent/carried-gaps.md` bullet is now a row in that file's § *Owned* table.

## Next group

**Stage 6: the register** — one file set: `crates/nvs-stdlib/src/`. Each item is a scheduling decision
written into the gap's own `— owner:` line: a goal slug on the chain, or an M9+ deferral whose plan
states the scope (`python tools/owners.py --deferrals` is the gate on the second kind).

- [ ] **A shared store behind a password, a database index or TLS** — `crates/nvs-stdlib/src/cache.rs:155`
      gap 1. The URL read is `redis://host[:port]` and each of the three is refused with a sentence:
      `AUTH` wants `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s plumbing to carry the
      credential, an index is a second namespace nothing names, and `rediss://` is a second caller of
      the TLS client `crate::http::transport` dials through. Each is a key beside `[cache.shared] url`
      before it is a connection, which is what makes the three one owner rather than three.
- [ ] **Nothing adjudicates between two declarations** — `crates/nvs-stdlib/src/response.rs:199` gap 2,
      where the last declaration wins by construction. § 4's sixth row is enforced by `E0801` in
      `nvs_types::response`, whose module doc owns what that rule reaches and what it still admits —
      two typed body members in one handler — so the owner owns a checker's reach, not this module's.
      The sibling gap above it is already M7's, which is past the program and not a deferral this one
      may copy.

## Backlog

- `crates/nvs-stdlib/src/uuid.rs:104` gap 1's § *Unowned* bullet in `docs/agent/carried-gaps.md` reads
  as a scheduling question while the gap itself is owned by goal `unowned-closures` — check the two
  agree when the register is swept.
- Stage 6 ends when `python tools/owners.py` reads `unowned: 0` and `--deferrals` stays green; the
  goal's own remaining stages are in `docs/agent/goals/60-unowned-closures.md`.
- When goal `unowned-closures` is green the driver takes `class-scoped-types`, then
  `worker-placement`, `core-class-tests`, `tds-bytes`, `gap-zero`.
