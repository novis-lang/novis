# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31, 32, 33 and 34 are closed**; 35 is the only one left, and its two `Core\Test` halves
closed here.

**M4 is closed and M3 is triaged down to one member.** M4's first half was stale — nine of
`Core\Test`'s cards name `Core\Test\Failure` in `errors` — and its second half is *no card owes it*:
`RecursionError` has one raise site, `nvs_runtime::ctx::nvs_stack_check`, the depth guard every
non-leaf function entry runs, so it is raised by the call rather than by any member body and belongs
to the exception tree alone. M3's headline name is on no roster: the assertion roster's one home is
`docs/spec/01-core-library.md`'s Part II class table, and `assertStartsWith` appears only as ADR 0079
§ 4's example of what `nvs-lsp` ranks by subject type. Against that table `Core\Test` is missing
exactly one member the landed milestone owes — **`assertContains`** — and the rest of the row is ADR
0079 § 24's M5-to-M8 schedule, absent on time. M3 therefore stays open as a five-edit `Core` member
slice, not a documentation fix.

**Item 35's remaining halves have no acceptance check and are open**: D6, D9, D20, D29, U13, M2, M6,
M7, and the two `docs/reference/lang/` chapter fixes. Its own check (M10's
`no_registry_card_cites_an_adr`) is green.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1077, differential
206. The driver's standing acceptance failure is stage 8's
`a-child-inherits-a-narrowed-capability-and-cannot-widen-it.nvst`, one of the two isolate cases that
were never written; it is an open item, not a regression, and stage 0c outranks it by the goal's own
ordering.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`. The next free `E02xx` is `E0247` and `E07xx` is `E0794`;
this session added no diagnostic.

**`orient.py`'s `[context]` gaps.** This session needed **0079 §§ 4 and 24** in `adrs`, and — the gap
that cost the most — `docs/spec/01-core-library.md`'s Part II class table, which no `[context]` field
selects at all: it, not any ADR, is where a `Core` class's roster is fixed, so every M-series finding
turns on a file the pack cannot print. Still missing, each proven by an earlier session: **0072
§§ 6-7**, **0012 § 6**, **0013 §§ 2-4**, **0046 §§ 2, 4-5**, **0053 §§ 1-3**, **0007 §§ 2-3**,
**0027 § 1**, **0031 § 3**, **0033 §§ 3-4**, **0047 § 2**, **0011**, **0086 § 6**, **0090 § 3**,
**0057 § 1**, **0096 §§ 1-1a** and **0117 § 1**; and in `modules`,
`crates/nvs-types/src/enums.rs`, `lib.rs`, `intrinsics.rs`, `crates/nvs-runtime/src/ctx.rs`,
`throwable.rs`, `host.rs`, `script.rs`, `crates/nvs-stdlib/src/task.rs`, `test.rs`,
`crates/nvs-config/src/snapshot.rs`, `directive.rs`, `crates/nvs-types/src/layout.rs`,
`crates/nvs-types/src/expr/args.rs`, `attributes.rs`, `serialize.rs`,
`crates/nvs-diagnostics/src/lib.rs`, `crates/nvs-hir/src/errors.rs`,
`crates/nvs-ir/src/lower/call.rs`, `crates/nvs-cli/src/openapi.rs` and
`crates/nvs-codegen/src/lib.rs`. `orient.py` still warns that `crates/nvs-host/src/budget.rs` matches
nothing, which is the forward anchor its own comment describes.

## Next group

**Item 35's two `#[Derive]`/attribute findings, which are one file set.** The file set is
`crates/nvs-types/src/derive.rs` and `crates/nvs-types/src/attributes.rs`; each weighs a finding's
verdict against what those two files already do, and
[docs/reference/findings.md](../reference/findings.md) § *Triage* holds both.

- [ ] **D9: private properties are JSON fields under `#[Json\Derive]`** — decide whether ADR 0071
      § 1's derive is visibility-blind on purpose (a codec that drops a field when it gains a
      `private` is the silent-shadowing failure again) or owes a refusal, and write the verdict into
      the finding. `crates/nvs-types/src/derive.rs:671` is `codec_field`, the one place a property
      becomes a field; `crates/nvs-types/src/derive.rs:644` and `:660` are the two ways one arrives.
- [ ] **D20: an empty shape `{}` is satisfied by every attached attribute literal**, so a bare marker
      retrieves as any other marker. ADR 0046 § 4 makes retrieval structural on purpose and ADR 0071
      § 1 makes the *compiler-recognized* list nominal; decide whether that pair already answers the
      finding. `crates/nvs-types/src/attributes.rs:105` is `check_attribute` and
      `crates/nvs-types/src/attributes.rs:314` is the `Ty::Shape` test the match runs through.

## Backlog

- `Core\Test::assertContains` — the one M4S-tail roster member absent from the registry; the five
  edits of *A `Core` member* over `crates/nvs-stdlib/src/test.rs` (M3, findings.md).
- M2, M6, M7, D6, D29, U13 and the two `docs/reference/lang/` chapter fixes — item 35's rest.
- Stage 8's two unwritten isolate cases, one of which is the driver's standing acceptance failure
  (`docs/agent/loop-goal.toml`, stage 8).
- Stage 9's ADR 0119 lowering, items 21–23, resuming when stage 0c is green.
