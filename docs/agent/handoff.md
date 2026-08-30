# Handoff

## State

**Stage 0c — the reference findings — is the open stage, and it runs ahead of everything else in this
goal, stage 9 included.** The user decided it on 2026-08-30 in an interactive session that triaged all
82 items of [docs/reference/findings.md](../reference/findings.md) — its § *Triage* table is each item's
verdict and owner — put the sixteen open decisions to the user, and folded every answer into the ADR
that owns it: 0006, 0007, 0033, 0047, 0071, 0090, 0091, 0094, 0103, 0107, `docs/adr/README.md`
§ *Decisions taken at project start*, and `divergences.md`. `docs/agent/loop-goal.md` § *Stage 0c* is
five items, 31–35, each naming the section that is now the rule and the `file.rs:NN` where the binary
breaks it; the six `[[check]]`s under `stage = "0c findings"` in `docs/agent/loop-goal.toml` name the
cases and tests each item owes, and `[context] modules` gained the fourteen files they touch. **Nothing
of it is implemented yet.** Every item is a bug against a decision that exists; none is `BLOCKED`.

**Stage 9 stands where the previous handoff left it** — ADR 0119 accepted, items 21–23 written with
their anchors, nothing implemented — and resumes when stage 0c is green. **Stage 8 is as before**: six
of its eight named cases written, 1017 conformance / 206 differential against floors of 1050 / 210, and
its two remaining cases are listed after the groups below.

**A `#[Test]` cannot read the configuration.** `--RUN-- test` runs each test in its own isolate, but the
runner resolves no tree (`crates/nvs-cli/src/runner.rs:245`), so `Core\Config` answers empty there. The
playbook's *Writing a test case* bullet owns the spelling; whether the runner *should* resolve
`./nvs.toml` is in `## Backlog`.

## Next group

**Item 31 — the abort, and the wrong answers a working program hits.** One file set: the runtime pair
the abort touches and three library modules with a wrong arm. It starts with P15 because that is the one
memory-safety item. The item in `loop-goal.md` § *Stage 0c* carries the rule each fix answers to; the
check `conformance (findings: the abort and the wrong answers, item 31)` names the case each owes.

- [ ] **P15** — `crates/nvs-runtime/src/release.rs:54`, reached from `crates/nvs-runtime/src/array.rs:1247`:
      a memory-limit abort inside `try { … } finally { … }` releases a value twice ("attempt to subtract
      with overflow", exit 127). Without the `finally` it is clean. Case
      `tests/conformance/error/a-limit-breach-inside-finally-is-a-clean-fatal.nvst`.
- [ ] **D34** — `crates/nvs-stdlib/src/ordering.rs:107` has no `decimal` arm, so `Arr::sort` over
      `array<decimal>` throws at run time. Case `tests/conformance/core/arr-sort-orders-decimals.nvst`.
- [ ] **D5** — `crates/nvs-stdlib/src/capability.rs:256` `resolved()`: a bare relative path's parent is
      `""`, which never canonicalises. Case
      `tests/conformance/core/a-bare-relative-path-resolves-against-its-grant.nvst`.
- [ ] **D25** — `crates/nvs-types/src/defaults.rs:397` `literal_default` has no `ExprKind::Null` arm.
      Case `tests/conformance/core/a-nullable-property-defaults-to-null.nvst`.
- [ ] **D24, D14, U11, D15** — one case each, named in the check. `1.0 / 0` throws (ADR 0007 § 4):
      `crates/nvs-ir/src/lower/operator.rs:878` guards the zero divisor only for `Int | Uint`. A child
      without `return` answers `null` (ADR 0006): `crates/nvs-ir/src/lower/mod.rs:1175` is the implicit
      `1`, which `require` keeps and `spawn script` does not. `password_file` is read under `nvs run`
      and `config dump` masks the value and shows the path (ADR 0103 §§ 7, 9):
      `crates/nvs-config/src/secret.rs:74`. `mode.default` is the key (ADR 0091 § 4):
      `crates/nvs-config/src/directive.rs:107`.
- [ ] **U15** — `crates/nvs-cli/src/main.rs:897-925` drops `--filter` for `#[Test]` methods. Test
      `test_filter_selects_test_methods_by_name` in `nvs-cli`.

**Then, in this order:** items 32, 33, 34 and 35 of stage 0c, each its own file set named in the item;
then stage 9's items 21–22 exactly as the previous handoff carried them (ADR 0119 §§ 1–5 first, anchors
in the goal file), then item 23; then the two stage-8 cases:

- `tests/conformance/isolate/a-child-inherits-a-narrowed-capability-and-cannot-widen-it.nvst` — M6's
  *Verify* and ADR 0118 § 1. `spawn script`'s `with(grants: …)` is `E0777` at its own site, so the
  narrowing a child inherits comes from the configuration (`crates/nvs-config/src/capability.rs:189`),
  not the spawn expression; a `Core\Config::set` in a parent is visible to a child it spawns.
- `tests/conformance/error/a-limit-fatal-is-not-catchable.nvst` — ADR 0020: a `[limits]` breach is a
  `FATAL` no ordinary `catch` sees. `crates/nvs-runtime/src/budget.rs:6`, `crates/nvs-runtime/src/ctx.rs:1128`,
  `crates/nvs-config/src/tree.rs:194`.

## Backlog

- `Core\Html::escape` is **M7** in ADR 0087's table, `crates/nvs-syntax/src/bidi.rs:6`, `lib.rs:19` and
  `docs/plan/m1.md:29`, and **M8** in `docs/plan/m8.md:66` and goal 4's item 27. One side is wrong and
  the user decides which — surfaced by the findings triage, not decided by it.
- `docs/agent/goals/3-governance.md`/`.toml` carry neither stage 0c nor stage 9; the live
  `loop-goal.md`/`.toml` do. `goal-switch.py` reads the live file so nothing is lost, but a reader of the
  goals directory is two stages behind.
- Decide whether `nvs test` resolves the configuration tree the way `nvs run` does — `docs/plan/m6.md`.
- 33 more conformance cases and 4 more differential ones to reach stage 8's floors — `docs/agent/loop-goal.toml`,
  the stage 8 `conformance` and `differential` checks.
- `orient.py` warns that `[context] modules`' `crates/nvs-host/src/budget.rs` matches no module; it is
  `crates/nvs-runtime/src/budget.rs` now — `docs/agent/loop-goal.toml`.
- `[context] modules` is missing `crates/nvs-test/src/*.rs`: the `--RUN--` subcommand roster
  (`crates/nvs-test/src/case.rs:60`) is what a stage 8 case has to pick from — `docs/agent/loop-goal.toml`.
- ADR 0042's cache payload, decided in `crates/nvs-cli/src/cache.rs`'s *Known gaps* — that module doc.
- A lint naming `as ?int` for `$s as int catch (…) => null` — ADR 0119 *Revisiting*.
