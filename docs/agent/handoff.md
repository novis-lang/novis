# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
Items 31 and 32 are closed; **item 33 has seven of its nine findings landed** — U1 (`readonly`),
U2 (`final`), U3 (`abstract`), U4 (the `secret` sinks), U6 (the `#[Command]` shape), U12 (the
`get`-only hook) and U14 (the stray `#[Access]`). **U5 and P5 are what is left**, and the item-33
`[[check]]` names a case for P5 and none for U5.

**ADR 0033 § 4's two remaining sinks are closed at the ends the checker can see.** `E0790` refuses a
`secret` operand at `echo` and `print` — `crates/nvs-types/src/expr/quals.rs`'s
`reject_secret_output`, which covers § 4's *"`echo` and interpolation"* both, because interpolation
and `.` spread the qualifier to their result and so the composed literal arrives at the sink already
`secret`. `E0791` refuses `Core\Json::encode` from `reject_secret_encoded_argument`, which asks
`contains_secret` rather than `is_secret` so a declared `array<secret string>` is refused beside the
bare value. Both are call-site rules for the reason `reject_secret_debug_argument` documents: the
member declares `mixed`, so the written argument is the last place the qualifier exists.

**A written array literal erases the qualifier before any sink sees it**, and that is the container
axis rather than a gap in either rule — see the playbook bullet and
`reject_secret_encoded_argument`'s doc. The fix is an element type for a literal
(`crates/nvs-types/src/expr/literals.rs:718`), and it is in `## Backlog`.

**`Qual::Reveal` is declared and no row writes it** (`crates/nvs-stdlib/src/registry.rs:154`), which
is U5: every `secret` refusal's help already tells the author to call `Core\Secret::reveal`, and the
member does not exist.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247`, and `E07xx` is now `E0792`.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1050 (at its
floor), differential 206.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] adrs` names none of stage 0c's own
ADRs: this session's item is ADR 0033 § 4 and the pack printed 0103, 0078, 0042 and 0119 instead, so
**0033 § 4** was sliced by hand and should be added, as should **0086 § 6** and **0096 §§ 1-1a** from
the session before. `[context] modules` still misses `crates/nvs-types/src/attributes.rs`,
`routes.rs`, `commands.rs`, `derive.rs` and `testing.rs`, and `orient.py` itself warned that
`crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**Item 33's last two findings. They do not share a file set — take P5 first: it is the one the
item-33 `[[check]]` names, and it is the last case in that check's list.**

- [ ] **P5 an `array<T>`-typed class constant folds instead of panicking** — findings.md § *Triage*.
      `crates/nvs-ir/src/lower/expr.rs:286` panics *"a `Class::CONST` … with no value recorded"* for
      a constant whose value has no constant form, which an `array` has none of; the fold that must
      produce one is `crates/nvs-types/src/consts.rs:55`'s `Const`, and ADR 0047 § 2 is the scope it
      folds under. Case: `tests/conformance/core/an-array-constant-folds.nvst`.
- [ ] **U5 `Core\Secret::reveal` exists** — ADR 0033 § 3's named escape hatch, written as
      conventions.md § *A `Core` member*'s five edits over the mark
      `crates/nvs-stdlib/src/registry.rs:154` already declares and no row writes; the class roster it
      joins is `crates/nvs-stdlib/src/registry.rs:984`, and `crates/nvs-stdlib/src/json.rs` is the
      small module to copy the shape from. No `[[check]]` case names it — write one and add it to
      item 33's list.

## Backlog

- An array literal drops `secret`/`tainted` from its elements — `crates/nvs-types/src/expr/literals.rs:718`; ADR 0033 § 1 owns the axis.
- `Core\Log::write` is not a `secret` sink yet — ADR 0033 § 4 names it beside the two landed here.
- Items 34 and 35 of `docs/agent/loop-goal.md` § *Stage 0c*.
- Stage 9's items 21–23 — ADR 0119, nothing implemented.
- Stage 8: differential 206 against 210, two cases unwritten.
- P4, a `: never` method panicking the lowerer — findings.md § *Triage*.
