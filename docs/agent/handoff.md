# Handoff

## State

**Stage 8 — the corpus and the guards — is the open stage.** `docs/agent/loop-goal.toml:1929` names eight
conformance cases and floors of 1050 conformance / 210 differential. **Six of the eight are written** —
`tests/conformance/config/`'s three, `cap/`'s two and `reject/config-set-above-the-hard-ceiling-returns-false.nvst`
— and the counts stand at 1017 and 206. Every other stage the goal names (0b, 2–7) is closed at its
acceptance.

**The gate is red for a file this work does not touch, and that is the driver's acceptance failure.**
`crates/nvs-cli/src/meta.rs` is uncommitted in this tree: another agent is adding `kind`, `signature`,
`params`, `returns`, `type` and `value` keys to the `nvs meta --json` document, for the untracked
`tools/reference.py` and `docs/reference/`. Those keys fail the two goldens at
`crates/nvs-cli/tests/meta.rs:69` and `:175`, which pin the document's older shape. Updating the goldens
would commit tests against source no one else has, and reverting the file would throw away another
agent's work, so both were left alone; the failure clears when that agent commits. It also stops
`python tools/verify.py` at `fmt`, step 2 of 7, so this session's own verification is the build (green)
plus `target/debug/nvs test tests/conformance/` run directly: **1017 passed, 0 failed, 0 skipped**. Both
slices are `.nvst` data and no Rust, so no later step of the gate can see them. **Re-run
`python tools/verify.py` whole once `meta.rs` is committed or reverted.**

**A `#[Test]` cannot read the configuration.** `--RUN-- test` runs each test in its own isolate, but the
runner resolves no tree (`crates/nvs-cli/src/runner.rs:245`), so `Core\Config` answers empty there. The
playbook's *Writing a test case* bullet owns the spelling; whether the runner *should* resolve
`./nvs.toml` is in `## Backlog` and is a real decision, not an oversight this session found time to fix.

## Next group

**The two remaining named cases of stage 8's conformance check.** One file set: `tests/conformance/`
under `isolate/` and `error/`, with `--FILE nvs.toml--` as the setup mechanism in both, and
`tests/conformance/config/config-set-is-invisible-to-the-next-request.nvst` as the worked example for a
case that has to run more than one request.

- [ ] **`tests/conformance/isolate/a-child-inherits-a-narrowed-capability-and-cannot-widen-it.nvst`** —
      M6's *Verify* and ADR 0118 § 1. Check the spelling before designing the case: `spawn script`'s
      `with(grants: …)` is refused at its own site as `E0777`, "not enforced yet"
      (`tests/conformance/lang/an-uncompiled-construct-is-refused-where-it-is-written.nvst:23`), so the
      narrowing a child inherits comes from the configuration and not from the spawn expression. The
      check itself is `crates/nvs-config/src/capability.rs:189`, and a child does inherit the parent's
      resolved view — a `Core\Config::set` in a parent is visible to a child it spawns, measured this
      session.
- [ ] **`tests/conformance/error/a-limit-fatal-is-not-catchable.nvst`** — ADR 0020: a `[limits]` breach is
      a `FATAL` that no ordinary `catch` sees. `crates/nvs-runtime/src/budget.rs:6` names memory as the
      first of the five, `crates/nvs-runtime/src/ctx.rs:1128` is the limit the allocator is measured
      against, and `crates/nvs-config/src/tree.rs:194` is the five keys a `[limits]` block spells.

## Backlog

- Decide whether `nvs test` resolves the configuration tree the way `nvs run` does — `docs/plan/m6.md`.
- 33 more conformance cases and 4 more differential ones to reach stage 8's floors — `docs/agent/loop-goal.toml:1928`.
- `orient.py` warns that `[context] modules`' `crates/nvs-host/src/budget.rs` matches no module; it is
  `crates/nvs-runtime/src/budget.rs` now — `docs/agent/loop-goal.toml`.
- `[context] modules` is missing `crates/nvs-test/src/*.rs`: the `--RUN--` subcommand roster
  (`crates/nvs-test/src/case.rs:60`) is what a stage 8 case has to pick from, and this session paid to
  find it — `docs/agent/loop-goal.toml`.
- ADR 0042's cache payload, decided in `crates/nvs-cli/src/cache.rs`'s *Known gaps* — that module doc.
