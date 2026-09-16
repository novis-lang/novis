# Handoff

## State

**Goal `unowned-closures`, and the register is at `unowned: 15`.** `python tools/owners.py` reports
`unowned: 15`, `untagged: 0`, `broken-tag: 0`, `unreasoned: 0` and `sections outside Known gaps: 0`;
`python tools/owners.py --deferrals` is green. The 15 still need answers only the user can give, bar
`crates/nvs-runtime/src/graph.rs:74` gap 1, so `unowned: 0` stays a `BLOCKED` the moment that one is
settled.

**`ctor_init.rs` keeps no `Known gaps` block: the code was already ahead of it.** A class that
declares no constructor refuses every own required property with no default, inherited constructor or
not — `check_class_init`'s no-constructor arm (`crates/nvs-types/src/ctor_init.rs:142`) has always run
for a class that `extends`, which running the compiler over the fixture confirmed, so no `E0824` was
claimed and the arm is unchanged. What the slice wrote is the bound as prose, the amended fragment
(`rule:classes/definite-property-initialization` now says inheriting a constructor is not declaring
one) and the guard case that pins the inheriting shape the existing case did not cover.

**`requires.rs` gap 1 is half built, and the half left is the `const` one.** A `require` path built
out of literals joined by `.` now folds and resolves (`literal_require_path`,
`crates/nvs-hir/src/requires.rs:1405`); a class constant in one does not, because the only constant
Novis has is a class constant and reading one needs the table this walk is building. The sheet's
`Decided:` sentence stays on the narrowed bullet rather than being re-priced.

## Next group

**Stage 5: the cache side of the probe trace** — one file set: `crates/nvs-cli/src/script.rs`,
`crates/nvs-config/src/cache.rs` and `crates/nvs-cli/src/main.rs`.

- [ ] **The probe trace's digest joins the unit key** — `crates/nvs-config/src/cache.rs:269`
      (`UnitKey::new`) takes it beside the content hash, and `crates/nvs-cli/src/main.rs:1564` is
      where the `AutoloadMap` that carries `probe_trace()` already exists and is dropped, so `Checked`
      carries it out to `crates/nvs-cli/src/script.rs:648`.
      `rule:packaging/autoload-probes-fold-into-the-cache-key` is the rule.
      **The design problem to answer first:** the key can only be formed from a trace the compile
      produces, so a path with no recorded trace has nothing to look up — record the trace per content
      digest beside the unit table and insert the compiled unit under the key the *post-compile* trace
      gives, or a cold path compiles twice before it settles.
- [ ] **A probed miss becomes a negative path entry** — `crates/nvs-cli/src/script.rs:163`
      (`PathEntry`, every field `Copy`) and `crates/nvs-cli/src/script.rs:712` (`observe`) are the
      anchors: re-stat the recorded probe paths under the same `validate`/`revalidate_freq` gate the
      content stat rides, so a file created where a probe missed recompiles the unit.
      `rule:packaging/autoload-probes-fold-into-the-cache-key`, and
      `crates/nvs-hir/src/requires.rs:1952` is the hir-side half already pinned.
- [ ] **The rule stops saying half of it is on disk** —
      `docs/rules/packaging/autoload-probes-fold-into-the-cache-key.md:22` is the paragraph, and
      `crates/nvs-hir/src/requires.rs:107`'s gap 2 is struck in the same slice once both land.

## Backlog

- `crates/nvs-runtime/src/graph.rs:74` gap 1 — the last unowned item a session could settle, and the
  one that turns this goal into a `BLOCKED`; the module doc owns it.
- `crates/nvs-hir/src/requires.rs:96` gap 1's `const` half — the obstacle is written in the bullet.
- Stage 3's M1 pair — `crates/nvs-syntax/src/lib.rs` gaps 2 and 3 — is untaken; the goal file owns
  the description.
- `crates/nvs-types/src/intrinsics.rs` gaps 1–5 (the prepared-pattern channel) are the stage's
  largest Decided block and the one ADR slot this goal may open.
