# Handoff

## State

**M4's frontier is ADR 0043 § 4, whose first bullet now runs.** A `by $field`
clause's field is judged where it is written and the conformance check is per
member, so a class delegating one interface is still judged on every other one it
claims.

- **The mechanism has one home each and is not restated here**: the field check in
  `nvs_types::conformance::check_delegate_field`
  (`crates/nvs-types/src/conformance.rs:264`), the covered-member set it feeds in
  `resolve_delegations` (`:158`), the two codes in `nvs-diagnostics`
  (`E_DELEGATE_TYPE_MISMATCH`, `E_DELEGATE_MEMBER_NOT_FORWARDABLE`), and the rules
  themselves in ADR 0043 § 4's own bullet list.
- **`static`, variadic and `inout` members are `E0721` now**, not silence: they are
  a limit of the forward's shape rather than a rule, and the way out is § 4's own
  "write the member by hand".
- **§ 4's worked example does not compile**, and that is the next slice: it
  delegates to a promoted constructor parameter, which `nvs_types::layout` does not
  record as a property, so `resolve_property` answers `None` and the new `E0720`
  fires on correct code. The ADR's bullet 1 names the shape explicitly.
- **`orient.py`'s pack was complete for this item.** `[context] modules` still has
  no `nvs-diagnostics` entry, and the pack does not print `nvs-types`'
  `signatures`/`layout` map lines, which is where both remaining slices live.

## Next group

**ADR 0043 § 4's last edge, and the two `Delegation` shapes around it.** The file
set is `crates/nvs-types/src/` (`layout.rs`, `conformance.rs`) and
`crates/nvs-ir/src/lower/call.rs`.

- [ ] **A promoted constructor parameter is a property**, so § 4's own worked
      example compiles. `crates/nvs-types/src/layout.rs:271`'s `own_properties` and
      `:334`'s `flatten_methods` are the two ends; the field lookup that now
      depends on it is `crates/nvs-types/src/conformance.rs:264`. Add the worked
      example itself as a running `.nvst` once it does.
- [ ] **A delegate reached before `$field` is written**, ADR 0043 § 4 bullet 2:
      the forward reads the slot and dispatches on it, so an unwritten
      non-nullable property has to throw ADR 0022's own checked error rather than
      dispatch on whatever the slot holds. The read is
      `crates/nvs-ir/src/lower/call.rs:911`'s `delegation_forward`.
- [ ] **Two interfaces delegating to one field**, § 4 bullet 3's other half, has
      no case: `implements A by $x, B by $x` should synthesize both member sets
      onto the one field. `crates/nvs-types/src/conformance.rs:158`'s clause loop
      already allows it; a `.nvst` beside the two this session wrote is the work.

## Backlog

- A `require` whose path is not a string literal runs nothing, silently, in both
  forms — `nvs_hir::requires`' own known gap.
- `E_INTERFACE_MEMBER_CONFLICT`, ADR 0043's shared conflict rule, has no site: a
  name reachable from two defaults or two delegations is not refused.
- `nvs-ir`'s known gap list and `holes.py`'s remaining items are the wider M4
  worklist — `python tools/holes.py` is the live count.
- ADR 0024 § 4's sink list and ADR 0033's `Core\Log` inspection wait on M7/M8.
