# Handoff

## State

**M4 — language completeness.** The closure's entry now pays for the class label a four-bit tag
cannot hold. `mwl_ir::lower::closure::check_param_class` branches the body's first block on one
`instanceof` per class-declared parameter and throws `LogicError` on the miss, so the type
confusion that stood behind this group — two `final` classes and one wrong `Core\Arr::filter`
callback writing an `int` over a `string` field, `misaligned pointer dereference`, from a program
with no `unsafe` in it — is closed. The tag word gained no class channel.

`docs/adr/README.md` § *Decisions taken at project start* states the rule and its **three
deliberate remainders**: a `?C` parameter is unchecked on both lines (it erases to `Ty::Tagged`
before any class survives), a `Core` class parameter keeps the objecthood-only check (a unit's
class table is `mwl_types::layout`'s declared tree, so there is no descriptor and no `instanceof`
spelling either), and the refusal does not name the class that *arrived* — no IR instruction reads
an object's class name, so `must be of type Marker, another class given` is the message's limit
until something other than a diagnostic wants that value shape.

`tests/conformance/core/out-a-callback-parameter-naming-a-class-checks-the-argument-class-at-entry.mwlt`
(renamed from `…-is-checked-by-representation-not-by-class`) pins both lines at once — the tag
word's around objecthood, the entry check's around ancestry, with a parent class and an
implemented interface both accepting the instance an exact class does. `tools/leak-check.sh` is
clean over the refusal path; `verify.py` green — conformance 624, differential 173.

**Gap in the pack:** `orient.py` printed neither `docs/adr/README.md` § *Decisions taken at project
start* (which the item names as its specification) nor the `.mwlt` case it was about to flip, so
both were read by hand. `[context] adrs` takes a `README.md:"## Decisions taken at project start"`
selector for the first.

## Next group

**Close `mwl-ir`'s statement-slice refusals.** The file set: `crates/mwl-ir/src/lower/stmt.rs`,
`crates/mwl-diagnostics/src/lib.rs`, `tests/conformance/lang/`. `python tools/holes.py --item 4`
and `--item 7` attribute every site below.

- [ ] **A nullsafe assignment target is a diagnostic, not a panic.** `crates/mwl-ir/src/lower/stmt.rs:814`
      panics on `$a?->b = v`; `docs/agent/loop-goal.md` § *Standing decisions* already settles it as
      a compile error taking a new `E`-code, and the orientation pack's *next free number* section
      names it. The array-element write through an ADR 0014 § 1 hooked property is the same
      decision's other half.
- [ ] **The control-flow slice's "only lowers …" panics name a shape the checker accepted.**
      `crates/mwl-ir/src/lower/stmt.rs:200`, `:331`, `:1120` — item 4's `$x++`/`--$x` and compound
      assignment reach the first two, a reassignment target that is neither a local nor a
      compile-time-known property the third. Each is a refusal that must become a lowering or a
      diagnostic naming the rule, never a panic.
- [ ] **`reduce` counts positions, not roles** — carried, untaken twice now, and on its own file
      set: `crates/mwl-stdlib/src/arr.rs:2611`. The carry is argument 1, the element 2 and the key
      3, and the refusal names the position rather than the role. Take it alone, not as this
      group's third.

## Backlog

- ADR 0007 § 4's promotion table still has 11 refusal sites (`holes.py --item 1`).
- A named argument and a spread argument type-check and lower — 3 sites, `holes.py --item 16`.
- `object` as a declared type has a representation arm — `crates/mwl-ir/src/lower/mod.rs:2399`, `:2482`.
- `<=>` answers for a scalar — 1 site, `holes.py --item 6`.
- Two unattributed refusal sites in `crates/mwl-codegen/src/ty.rs:116`/`:121` belong to no item.
- 14 of 32 named cases still unwritten, all under milestone 8's corpus (`holes.py --cases`).
