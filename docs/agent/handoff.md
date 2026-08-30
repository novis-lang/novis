# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31, 32, 33 and 34 are closed**; 35 is the only one left, and three of its cards closed here.

**D3, D2 and D4 closed, all in `crates/nvs-stdlib/src/time.rs`.** `WEEKDAY_DOC` keeps `date("N")` for
the *order* and states that the numbering is not its — the cases are zero-based. `Duration::compareTo`
says `==` on two objects is identity, so `$a->compareTo($b) == 0` is the content comparison (ADR 0090
§ 3). The three cards on ADR 0057 § 1's roster — `Duration::parse`, `DateTime::format` and
`Core\Time::parse` — name `E0769` and say only a computed argument reaches the throw; the
`DateTime::format` card also names `Date::format` and `TimeOfDay::format` as the off-roster half,
which `nvs_types::intrinsics`'s module doc owns. **A card may not contain the word `ADR`** —
`no_registry_card_cites_an_adr` in `crates/nvs-stdlib/src/registry.rs` — so a card names the
diagnostic code or states the rule instead of citing the decision.

**Item 35's remaining halves have no acceptance check and are open**: D6, D9, D20, D29, U13, M2, M3,
M4, and the two `docs/reference/lang/` chapter fixes. Its own check (M10's
`no_registry_card_cites_an_adr`) is green.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1077, differential
206. The driver's standing acceptance failure is stage 8's
`a-child-inherits-a-narrowed-capability-and-cannot-widen-it.nvst`, one of the two isolate cases that
were never written; it is an open item, not a regression, and it outranks nothing here.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`. The next free `E02xx` is `E0247` and `E07xx` is `E0794`;
this session added no diagnostic.

**`orient.py`'s `[context]` gaps.** This session needed **0090 § 3** and **0057 § 1** in `adrs`, and
`crates/nvs-types/src/intrinsics.rs`, `crates/nvs-diagnostics/src/lib.rs` and
`crates/nvs-stdlib/src/test.rs` in `modules`. Still missing, each proven by an earlier session:
**0072 §§ 6-7**, **0012 § 6**, **0013 §§ 2-4**, **0046 §§ 2, 4-5**, **0053 §§ 1-3**, **0007 §§ 2-3**,
**0027 § 1**, **0031 § 3**, **0033 §§ 3-4**, **0047 § 2**, **0011**, **0086 § 6**, **0096 §§ 1-1a**
and **0117 § 1**; and in `modules`, `crates/nvs-types/src/enums.rs`, `crates/nvs-types/src/lib.rs`,
`crates/nvs-runtime/src/ctx.rs`, `host.rs`, `script.rs`, `crates/nvs-stdlib/src/task.rs`,
`crates/nvs-config/src/snapshot.rs`, `directive.rs`, `crates/nvs-types/src/layout.rs`,
`crates/nvs-types/src/expr/args.rs`, `retrieval.rs`, `attributes.rs`, `serialize.rs`,
`crates/nvs-hir/src/errors.rs`, `crates/nvs-ir/src/lower/call.rs`,
`crates/nvs-cli/src/openapi.rs` and `crates/nvs-codegen/src/lib.rs`. `orient.py` still warns that
`crates/nvs-host/src/budget.rs` matches nothing, which is the forward anchor its own comment
describes.

## Next group

**Item 35's two `Core\Test` findings, which are one file.** The file set is
`crates/nvs-stdlib/src/test.rs`; each is a finding's verdict weighed against what that file already
holds, and [docs/reference/findings.md](../reference/findings.md) § *Triage* holds both.

- [ ] **M4: the two classes the finding says no card's `errors` names** — half of it is already
      stale: `crates/nvs-stdlib/src/test.rs:314` is one of nine cards naming `Core\Test\Failure`.
      `crates/nvs-runtime/src/throwable.rs:109` is `RecursionError`'s one home and the depth guard
      raises it rather than any member, so decide whether a card owes it at all and write the verdict
      into the finding.
- [ ] **M3: `Core\Test` holds ten members where ADR 0079 names a wider roster** —
      `crates/nvs-stdlib/src/test.rs:158` is `CLASS`'s ten rows,
      `docs/adr/0079-testing-is-a-language-feature.md:186` is where `assertStartsWith` is named. The
      verdict is *docs*, so the fix is the doc that promises a member the registry does not hold, not
      a new row.

## Backlog

- D6: `Core\Router::url`'s card — a computed name throws for every name until the route table is
  built (`docs/reference/findings.md`).
- D9: private properties are `#[Json\Derive]` fields — `crates/nvs-types/src/derive.rs`'s module doc.
- D20: an empty shape `{}` is satisfied by every attached literal, so a bare marker is `E0728` —
  `nvs_types::retrieval`.
- D29: `await`'s result renders as `Core\Script\Result#1` — `crates/nvs-host/src/isolate.rs`.
- U13: a throwable's properties are writable though a conformance case calls `location` readonly.
- M2: `Core\Fatal::onUncaughtThrow` (ADR 0020 § 2) — only `onLimit` exists.
