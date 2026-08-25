# Handoff

## State

**Stage 0 items 1, 2, 3, 4, 5 and 9 are done.** ADR 0047 is whole in the checker *and* runs: a union whose
members all erase to one representation folds back to that representation (`lower_checked_ty`
[lower/mod.rs:1849](../../crates/mwl-ir/src/lower/mod.rs#L1849)), so `"a"|"b"` is a `Ty::Str` and `1|2` a
`Ty::Int` rather than the `Ty::Tagged` every union used to be, and § 4's checked `as` emits § 5's
membership test — one comparison per member, `Helper::LiteralMismatch` throwing at the far end with the
accepted set named exactly as `E0469` names it. A `mixed` operand is tested against its own runtime tag
*before* the base conversion, so `1 as "1"|"b"` throws instead of being rendered into the set it names.

`python tools/verify.py` is green (1348 tests), `mwl test tests/` is 426/0, and the WSL valgrind leg is
clean over 50 throw/catch iterations.

**One pre-existing double free was in the way and is fixed:** a `from == to` conversion row handed back
borrowed storage while every consumer reads `is_aliasing_read` off the `as` node and believes it owns a
fresh value, so `string $y = $x as string;` corrupted the heap. `convert`
([expr.rs:558](../../crates/mwl-ir/src/lower/expr.rs#L558)) now retains on that row. See the playbook's new
bullet: **exit 127 with correct output is that corruption**, not a missing command.

**Not yet true, and recorded as `mwl-ir` gap 20:** § 3's enum-case subset over an operand only known at run
time (`$any as Mode::Read|Mode::Write`) still panics. It needs each case's backing value, which lives in
`mwl_types::enums` and is not handed to `mwl-ir` — the same missing check as ADR 0010 § 5's `int`-into-an-
enum row, so plumbing that table closes both at once. `as ?"a"` runs no membership test either.

## Next group — Stage 0 item 6, `private`/`protected` are enforced

**Shared file set:** `crates/mwl-types/src/expr/members.rs`, `crates/mwl-types/src/expr/calls.rs`,
`crates/mwl-diagnostics/src/lib.rs` and `tests/conformance/lang/`. The rule is
`docs/agent/loop-goal.md` § *Stage 0* item 6: one pass keyed on the **accessing** class, over property
access and method resolution. Item 3 (ADR 0094) made every declaration state a level; this makes the level
mean something. `E0471` is the next free type code.

- [ ] **6a — a property access checks the accessing class.** `check_property_access`
      ([members.rs:217](../../crates/mwl-types/src/expr/members.rs#L217)) resolves the declaring class and
      today stops there. Add the level test: `private` reachable only from the declaring class itself,
      `protected` from it or a subclass, keyed on the frame's own class rather than on the receiver's.
      `mwl_hir::members`' own gap list ([members.rs:38](../../crates/mwl-hir/src/members.rs#L38)) says the
      `$this` half is not checked either — that list is what to update when it is.
- [ ] **6b — method resolution takes the same test.** `expr::calls` already calls
      `check_interface_private_visibility` ([members.rs:537](../../crates/mwl-types/src/expr/members.rs#L537))
      from two sites ([calls.rs:71](../../crates/mwl-types/src/expr/calls.rs#L71),
      [calls.rs:140](../../crates/mwl-types/src/expr/calls.rs#L140)) for ADR 0043 § 3's private *interface*
      method. Widen that to every class method rather than adding a second pass beside it.
- [ ] **6c — `.mwlt` cases under `tests/conformance/lang/`** pinning both refusals with
      `--EXPECTF-ERROR--`, plus one case that a subclass reaches a `protected` member and the same access
      from outside does not. Expect fixture fallout: the corpus was written with nothing enforcing this.

## Backlog

- `mwl-ir` gap 20's enum-case membership row, which also closes ADR 0010 § 5's `int`-into-an-enum
  conversion — `crates/mwl-ir/src/lib.rs`'s known gaps.
- `mixed as int`/`as uint` has no row in `convert` at all, so an `int`-literal set off a `mixed` panics
  before the membership test is reached — same file's gap list.
- ADR 0088's registry-wide qualifier classification for `mwl-stdlib` member rows — plan, `Open now`.
- `Comparable`/`Stringable` member signatures (item 7) and ADR 0061's `autoload` (item 8) —
  `docs/agent/loop-goal.md` § *Stage 0*.
- `equality_domain` puts a literal type in its base's ADR 0090 domain, so `$mode == "z"` compares two
  strings; refusing non-overlapping literal *sets* would be a new row in ADR 0090 § 2.
- `Core` breadth resumes at `examples/collect.mwl` — plan, `Open now`.

`orient.py` printed everything this session needed. One manifest gap, which 5d paid: `[context] modules`
selects `mwl-ir/src/lower/*.rs` but nothing from `mwl-codegen` or `mwl-runtime`, and a new `Helper` is
five files across all three — add `mwl-codegen/src/emit.rs` and `mwl-runtime/src/helpers.rs` to it.
