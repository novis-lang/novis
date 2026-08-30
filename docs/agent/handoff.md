# Handoff

## State

**Item 35's M3 landed: `Core\Test::assertContains` exists** — the registry row, the ADR 0117 card,
the `address()` arm and the helper body in `crates/nvs-stdlib/src/test.rs`, plus the three `.nvst`
cases `conformance_coverage.rs`'s floor asks for (agreement with `Core\Arr::contains`, strictness
across the tags, identity over objects). Its subject and its comparison are `Core\Arr::contains`'s
rather than a second membership reading, so the two members cannot disagree; the helper's own doc
comment owns why, including why a `string` subject is refused and why ADR 0013's `compareTo` is not
the comparison here.

**Stage 0c now has no implementable slice left.** Item 35's remaining halves — D6, D29, U13, M2, M6
and M7 — are ADR 0079 § 24's M5-to-M8 schedule, absent on time rather than missing;
[docs/reference/findings.md](../reference/findings.md) § *Triage* holds each verdict. Item 35's own
check (M10's `no_registry_card_cites_an_adr`) is green, and items 31–34 are closed.

**The driver's standing acceptance failure is stage 8's, and with stage 0c out of implementable work
it is now the top of the queue.** `docs/agent/loop-goal.toml:2090`'s eight-case list has exactly two
cases unwritten — the isolate one the check names and `error/a-limit-fatal-is-not-catchable.nvst` —
and the other six are on disk. Conformance 1080, differential 206.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`. The next free `E02xx` is `E0247` and `E07xx` is
`E0794`; this session added no diagnostic.

**`orient.py`'s `[context]` gaps.** New this session, in `modules`: **`crates/nvs-stdlib/src/arr.rs`**
— it is the model for any `Core` member taking a container and a needle, and both `borrowed` and
`slot_of` live there, so a `test.rs` item that must not invent a second comparison cannot be worked
without it. Standing, each proven earlier: no field selects `docs/reference/lang/*.md` or
`docs/reference/core/*.md`; `docs/adr/divergences.md`; `docs/reference/README.md` § *Examples: the
fence grammar*; `docs/spec/01-core-library.md`'s Part II class table. In `adrs`: **0007 §§ 2-4**,
**0079 §§ 4 and 24**, **0072 §§ 6-7**, **0012 § 6**, **0013 §§ 2-4**, **0046 §§ 2, 5**,
**0053 §§ 1-3**, **0027 § 1**, **0031 § 3**, **0033 §§ 3-4**, **0047 § 2**, **0011**, **0086 § 6**,
**0090 § 3**, **0057 § 1**, **0096 §§ 1-1a** and **0117 § 1**. In `modules`, also
`crates/nvs-types/src/enums.rs`, `lib.rs`, `intrinsics.rs`, `crates/nvs-runtime/src/ctx.rs`,
`throwable.rs`, `host.rs`, `script.rs`, `crates/nvs-stdlib/src/task.rs`, `test.rs`,
`crates/nvs-config/src/snapshot.rs`, `directive.rs`, `crates/nvs-types/src/layout.rs`,
`crates/nvs-types/src/expr/args.rs`, `attributes.rs`, `serialize.rs`,
`crates/nvs-diagnostics/src/lib.rs`, `crates/nvs-hir/src/errors.rs`, `crates/nvs-ir/src/lower/call.rs`,
`crates/nvs-cli/src/openapi.rs` and `crates/nvs-codegen/src/lib.rs`. `orient.py` still warns that
`crates/nvs-host/src/budget.rs` matches nothing — the forward anchor its own comment describes.

## Next group

**Stage 8's two unwritten acceptance cases — the file set is `tests/conformance/` plus the two
surfaces they exercise, and closing them clears the driver's standing failure.** Both are ordinary
`.nvst` authoring over landed runtime work; `python tools/try.py` handles the single-file shape and
`target/debug/nvs test <path>` the multi-file one.

- [ ] **`tests/conformance/isolate/a-child-inherits-a-narrowed-capability-and-cannot-widen-it.nvst`**
      — the case the acceptance check names. The question is
      `crates/nvs-config/src/capability.rs:189`'s `allows(cap, scope, files)`: a `spawn script` child
      inherits the parent's grant, a narrowing in the child's own block holds, and a widening past the
      parent's scope is refused rather than granted. M6's *Verify* paragraph is the specification;
      `tests/conformance/isolate/a-child-shares-nothing-with-its-parent.nvst:1` is the shape to copy
      for spawning and awaiting a child.
- [ ] **`tests/conformance/error/a-limit-fatal-is-not-catchable.nvst`** — the other unwritten case in
      the same list. ADR 0020 § 1's rule is that a limit breach is a `FATAL` no ordinary `catch` sees
      and that tier 1 is the registered `Core\Fatal::onLimit` closure, owned at
      `crates/nvs-runtime/src/ctx.rs:1244`; the case pins both halves — a `catch (Throwable)` around
      the breach does not run, and the handler does.

## Backlog

- Item 35's M5–M8 rows are a schedule, not a gap — `docs/reference/findings.md` § *Triage*.
- Stage 9's items 21–23: ADR 0119's expression `catch`, parser through lowering — `docs/agent/loop-goal.md`.
- The `[context]` manifest gaps above, one selector each — `docs/agent/loop-goal.toml`.
- `crates/nvs-host/src/budget.rs` warns on every orient — `docs/agent/loop-goal.toml`.
- `docs/reference/core/Test.md` is prose plus one worked example, so it lists no roster to keep in
  step with the registry — check that assumption if a member is ever removed.
