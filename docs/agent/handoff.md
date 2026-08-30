# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31–34 are closed**; 35 is the only one left, and its two `docs/reference/lang/` chapter fixes
closed here.

**Both landed differently from how the item stated them, and the binary is why.** `\v`, `\e` and `\f`
are *not* a divergence: Novis cooks them to `0x0B`/`0x0C`/`0x1B` exactly as PHP does, as do the octal
`\101` and a literal-`\q` fallthrough. The real gaps in `20-types.md` § *Literals* were that a
single-quoted literal was called "verbatim" when it has `\\` and `\'`, that `\0` was listed alone
rather than as the head of an octal family to `\777`, and that `\v`/`\f`/`\e` were missing from the
roster; all three are now stated and proved by an executed example. The decimal `017` was already
correct in the chapter — what it lacked was a home in the register, so **ADR 0007 § 4 now states the
four integer-literal forms and why a leading zero is not one**, and `docs/adr/divergences.md` gained
the row that indexes it. `40-statements.md:303` now names **`E0406`** for two `catch` clauses binding
one name, as the declare-once rule rather than a `catch`-specific one, with a ` ```nvs error ` example
holding it.

**Item 35's remaining halves have no acceptance check and are open**: D6, D29, U13, M2, M3, M6 and M7.
**M3 is the one implementable slice** — against `docs/spec/01-core-library.md`'s Part II class table
`Core\Test` is missing exactly `assertContains`, and the rest of that finding's roster is ADR 0079
§ 24's M5-to-M8 schedule, absent on time rather than missing. Item 35's own check (M10's
`no_registry_card_cites_an_adr`) is green.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors, nothing
implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1077, differential 206. The
driver's standing acceptance failure is stage 8's
`a-child-inherits-a-narrowed-capability-and-cannot-widen-it.nvst`, one of the two isolate cases never
written; it is an open item, not a regression, and stage 0c outranks it by the goal's own ordering.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`. The next free `E02xx` is `E0247` and `E07xx` is `E0794`;
this session added no diagnostic.

**`orient.py`'s `[context]` gaps.** The one that cost most is unchanged from last session and is now
proven twice: **no `[context]` field selects `docs/reference/lang/*.md` or `docs/reference/core/*.md`**,
which are the files every item-35 finding is fixed in, so the pack cannot print the file the item is
about. New this session: **`docs/adr/divergences.md`** (the register an item is told to check, and its
link from a chapter is `docs/adr/`, not the `docs/reference/` the item named) and
**`docs/reference/README.md`** § *Examples: the fence grammar*, without which the executed-example
fences are invisible. In `adrs`, **0007 § 4**. Still missing, each proven by an earlier session:
`docs/spec/01-core-library.md`'s Part II class table, **0079 §§ 4 and 24**, **0072 §§ 6-7**,
**0012 § 6**, **0013 §§ 2-4**, **0046 §§ 2, 5**, **0053 §§ 1-3**, **0007 §§ 2-3**, **0027 § 1**,
**0031 § 3**, **0033 §§ 3-4**, **0047 § 2**, **0011**, **0086 § 6**, **0090 § 3**, **0057 § 1**,
**0096 §§ 1-1a** and **0117 § 1**; and in `modules`, `crates/nvs-types/src/enums.rs`, `lib.rs`,
`intrinsics.rs`, `crates/nvs-runtime/src/ctx.rs`, `throwable.rs`, `host.rs`, `script.rs`,
`crates/nvs-stdlib/src/task.rs`, `test.rs`, `crates/nvs-config/src/snapshot.rs`, `directive.rs`,
`crates/nvs-types/src/layout.rs`, `crates/nvs-types/src/expr/args.rs`, `attributes.rs`, `serialize.rs`,
`crates/nvs-diagnostics/src/lib.rs`, `crates/nvs-hir/src/errors.rs`, `crates/nvs-ir/src/lower/call.rs`,
`crates/nvs-cli/src/openapi.rs` and `crates/nvs-codegen/src/lib.rs`. `orient.py` still warns that
`crates/nvs-host/src/budget.rs` matches nothing, which is the forward anchor its own comment describes.

## Next group

**Item 35's M3 — `Core\Test::assertContains`, the conventions' five edits, one file set.**
`crates/nvs-stdlib/src/test.rs` holds four of them and the fifth is a new `.nvst`.
`docs/spec/01-core-library.md`'s Part II class table is the signature, and `assertCount` is the model
to copy at every anchor: it is the neighbouring row that also takes a container and a needle.

- [ ] **The four in-module edits for `assertContains`** — the `CoreMethod` row beside `assertCount`'s
      at `crates/nvs-stdlib/src/test.rs:207`, its `MethodDoc` card in row order beside
      `crates/nvs-stdlib/src/test.rs:449`, the `address()` arm at
      `crates/nvs-stdlib/src/test.rs:620` (a miss here is a runtime panic naming the symbol, never a
      link error), and the `nvs_helper!` body beside `crates/nvs-stdlib/src/test.rs:769`. `args: [N]`
      must equal the row's arity with the receiver counted and the `params` not.
- [ ] **The `.nvst` case that calls it** — a new file under `tests/conformance/core/`, picked up with
      no registration; `crates/nvs-stdlib/tests/conformance_coverage.rs:1` fails
      `cargo test -p nvs-stdlib` until one exists. Assert the boundary rather than a second happy row:
      the empty container and the needle that is a prefix of a member but not a member.

## Backlog

- **D6** — `Core\Router::url` with a computed name throws for every name; `docs/reference/findings.md:222`.
- **D29** — `await`'s result renders as `Core\Script\Result#1` in `Core\Debug::render`; `docs/reference/findings.md:334`.
- **U13** — `$e->message = "b"` on a throwable is accepted and reads back; `docs/reference/findings.md:164`.
- **M2, M6, M7** — absent surface on ADR 0079 § 24's schedule, not defects; `docs/reference/findings.md:366`.
- **Stage 8's two unwritten isolate `.nvst` cases**, the driver's standing acceptance failure; `docs/agent/loop-goal.toml`.
- **Stage 9 items 21–23**, ADR 0119's expression `catch`, anchors already written; `docs/agent/loop-goal.md`.
