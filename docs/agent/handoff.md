# Handoff

## State

**Stage 0 items 1, 2, 3, 4, 5 and 9 are done, and item 6 is half done.** ADR 0094's levels now mean
something for a **property**: `mwl_types::expr::members::check_member_visibility`
([members.rs:508](../../crates/mwl-types/src/expr/members.rs#L508)) refuses `E0471` keyed on the
*accessing* class (`Ctx::current_class`) and never on the receiver's static type, so `$other->n` inside the
declaring class is legal and the identical line at file scope is not. `private` reaches only the declaring
class's own bodies — a parent's `private` is refused from a subclass — and `protected` reaches down an
`extends`/`implements` chain through `mwl_hir::implements_interface`. The rule itself is
`signatures::is_visible_from`, and the level comes from `ClassSignature::property_visibility`, filled from
the plain keyword only: `private(set)` is the write half of ADR 0094 § 3's pair and is not modeled.

The static spelling `Foo::$n` takes the same test (`expr/mod.rs`'s `StaticPropertyAccess` arm now resolves
through `resolve_property_owned`), and a write needs no separate arm — it reaches the member through the
same `PropertyAccess` span a read does.

`python tools/verify.py` is green (1358 tests) and `mwl test tests/` is 426/0; the three `examples/*.mwl`
that declare non-public members still run clean, so nothing in the corpus was reaching a member it should
not have been.

**Two shapes are deliberately outside it**, both recorded in `mwl-types`' `signatures` known gaps: a
promoted constructor parameter (no table records one as a property, so nothing resolves it to check), and a
**method**, which carries no visibility in `MethodSig` at all — that is item 6b below.

## Next group — Stage 0 item 6, the method half and its cases

**Shared file set:** `crates/mwl-types/src/signatures.rs`, `crates/mwl-types/src/expr/calls.rs`,
`crates/mwl-types/tests/visibility.rs` and `tests/conformance/lang/`. The rule is
`docs/agent/loop-goal.md` § *Stage 0* item 6; `E0471` already exists and is reused, so no new code is
claimed.

- [ ] **6b — method resolution takes the same test.** Give `MethodSig` a `visibility: Visibility` field
      next to `interface_private` ([signatures.rs:111](../../crates/mwl-types/src/signatures.rs#L111)),
      filled in `collect_members`' `ClassMemberKind::Method` arm
      ([signatures.rs:656](../../crates/mwl-types/src/signatures.rs#L656)) from the same
      `declared_visibility` helper the property arm uses. Then call `check_member_visibility` at the two
      sites `check_interface_private_visibility` is already called from —
      [calls.rs:71](../../crates/mwl-types/src/expr/calls.rs#L71) (`$obj->m()`) and
      [calls.rs:140](../../crates/mwl-types/src/expr/calls.rs#L140) (`C::m()`) — passing
      `&format!("{name}()")` as the member label. Watch the constructor:
      [calls.rs:217](../../crates/mwl-types/src/expr/calls.rs#L217) resolves `constructor` for `new C()`,
      and a `private` one is PHP's singleton idiom, so it must be refused from outside too rather than
      skipped. `mwl_hir::members`' gap list ([members.rs:38](../../crates/mwl-hir/src/members.rs#L38)) says
      the method half is still open — that sentence is what to update when it is.
- [ ] **6c — `.mwlt` cases under `tests/conformance/lang/`** pinning both refusals with
      `--EXPECTF-ERROR--` (a `private` property and a `private` method reached from another class) and one
      running case where a class reaches its own `private` members through a public one. The Rust half is
      already pinned in `crates/mwl-types/tests/visibility.rs`, so these exist to prove the diagnostic's
      rendered text, indentation included.

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

Two manifest gaps this session paid, both in `loop-goal.toml`'s `[context]`: `adrs` still names only ADR
0047's sections, so item 6's own ADR 0094 printed nothing — add `0094` §§ 1-3; and `modules` selects
`mwl-types/src/expr/*` but not `mwl-types/src/signatures.rs`, which is where every member's declared shape
actually lives. `[context] modules` also still lacks `mwl-codegen/src/emit.rs` and
`mwl-runtime/src/helpers.rs`, which a new `Helper` needs.
