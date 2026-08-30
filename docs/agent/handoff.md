# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35, each naming the
section that is now the rule and the `file.rs:NN` where the binary breaks it;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.

**Item 31's first two slices are landed.** P15 is closed at its cause: a landing site's *catchable*
exit is now a block of its own (`crates/nvs-ir/src/lower/mod.rs:1844` `landing_block`), and that
block rather than the landing block is what goes on `TryFrame::edges`. `merge_envs` writes a
merged-away name's release into an incoming block, and the landing block is the one place two exits
leave through a single terminator — so that release also ran on the uncatchable one, where
`Terminator::Catch`'s `onward` had already swept the frame. Any `foreach` inside a `try` supplies
the conditionally-bound refcounted name, and a `FATAL` there released its array twice. D34 is
closed too: `crates/nvs-stdlib/src/ordering.rs`'s `against_decimal` gives the natural ordering a
`decimal` row that reaches the other numeric rows as well, per ADR 0054 §§ 3-4.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8 is as before**: six of its
eight named cases written, conformance/differential against floors of 1050 / 210.

**The driver's last acceptance check failed on the driver, not the tree** — `failed to remove file
target\debug\nvs.exe`, a Windows file lock. Every build this session succeeded; nothing was found
wrong with the tree, and the playbook's *acceptance check can fail on the driver* bullet is the one
that applies.

**A `#[Test]` cannot read the configuration.** `--RUN-- test` runs each test in its own isolate but
the runner resolves no tree (`crates/nvs-cli/src/runner.rs:245`), so `Core\Config` answers empty
there. The playbook's *Writing a test case* bullet owns the spelling.

## Next group

**Item 31, the rest — the wrong answers a working program hits.** One file set: three library
modules and one CLI entry, each with a wrong arm. None of them touches the runtime pair the abort
did, so this is a fresh set. The item in `loop-goal.md` § *Stage 0c* carries the rule each fix
answers to; the check `conformance (findings: the abort and the wrong answers, item 31)` names the
case each owes.

- [ ] **D5** — `crates/nvs-config/src/capability.rs:256` `resolved()`: a bare relative path's parent
      is `""`, which never canonicalises, so `Core\File::read("missing.txt")` under a valid grant is
      a capability miss and `write("copy.txt")` under `write = ["."]` is refused. ADR 0118 § 2 is the
      rule; the case is `tests/conformance/core/a-bare-relative-path-resolves-against-its-grant.nvst`.
      (`orient.py`'s map printed no `crates/nvs-stdlib/src/capability.rs` — the manifest's pattern
      names a file that moved to `nvs-config`; resolve the anchor before reading.)
- [ ] **D25** — `crates/nvs-types/src/defaults.rs:397` `literal_default` has no `ExprKind::Null`
      arm, so `?int $x = null` is E0472. Case
      `tests/conformance/core/a-nullable-property-defaults-to-null.nvst`.
- [ ] **U15** — `crates/nvs-cli/src/main.rs:897-925` drops `--filter` for `#[Test]` methods; it
      filters `.nvst` paths only. The check is the `cargo-named` one, *nvs-cli (--filter selects
      #[Test] methods, item 31)*.
- [ ] **D24, D14** — one case each: `1.0 / 0` throws `ArithmeticError` whatever the operand types
      (ADR 0007 § 4), against the row at `crates/nvs-runtime/src/helpers.rs:990` that deliberately
      does not; and a child ending without `return` hands back `null` rather than `1`
      (ADR 0006 § *Values cross by copy*), decided in `crates/nvs-host/src/isolate.rs:393`. Cases
      `tests/conformance/error/float-division-by-zero-throws.nvst` and
      `tests/conformance/core/a-child-without-return-answers-null.nvst`.

## Backlog

- U11 and D15 — `password_file` under `nvs run`, and `mode.default`'s re-derivation; the same
  item 31 check names both cases. `docs/agent/loop-goal.md` § *Stage 0c*.
- Items 32–35: the missing refusals, the inert modifiers, the reference cards.
  `docs/agent/loop-goal.md` § *Stage 0c*.
- Stage 9 — ADR 0119's expression `catch`, items 21–23. `docs/agent/loop-goal.md`.
- Stage 8's two remaining named cases and the conformance/differential floors.
  `docs/agent/loop-goal.md`.
- Whether `nvs test` should resolve `./nvs.toml` for a `#[Test]` isolate.
  `crates/nvs-cli/src/runner.rs`'s module doc.
- `orient.py` reported four `[context] modules` patterns matching nothing —
  `crates/nvs-host/src/budget.rs`, `crates/nvs-stdlib/src/capability.rs`,
  `crates/nvs-types/src/calls.rs`, `crates/nvs-types/src/literals.rs`. They moved;
  `docs/agent/loop-goal.toml`'s manifest owns the fix.
