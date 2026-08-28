# Handoff

## State

**A `match` or a `switch` over an enum subject lowers.** Both label chains
compare on the backing integer through
`Lowering::reinterpret_enum_to_backing`, the move `lower_binary` and
`lower_literal_membership` already made — `nvs_ir::lower::expr::lower_match`'s
doc comment is the rule's home and `lower_switch`'s body comment points at
it. The plan's `Open now` carries it.

- **`tests/conformance/enum/a-match-and-a-switch-over-an-enum-compare-on-the-backing-integer.nvst`
  is new** and pins both backings, the hit, the `default` fall-off, the
  throwing fall-off with no `default`, the same subject through a `mixed`,
  and a `switch` over each.
- **The agreement case reads `agreed=49/49`** — the enum `match` row agrees
  from *below* (the backing integer) where the tagged one agrees from
  `Helper::Identical`.
- **A `catch` binding still has no callable members** — `$e->getMessage()`
  panics `nvs-ir` at `lower/expr.rs:2281`. Unchanged.
- **`orient.py`'s pack was complete for this item.** The two standing
  manifest gaps are unchanged — `[context] modules` has no `nvs-runtime`
  and no `nvs-diagnostics` entry.

## Next group

**The `catch` binding's members, over the one lowering file plus the checker
one.** The files: `crates/nvs-ir/src/lower/expr.rs` and
`crates/nvs-types/src/expr/`.

- [ ] **`$e->getMessage()` on a `catch` binding** — `crates/nvs-ir/src/lower/expr.rs:2281`
      panics rather than lowering a call on the synthesized `Throwable`
      binding, so a case reads `$e->message` instead. Decide whether the
      member exists at all (spec § 10's `Throwable` shape) and either lower
      it or refuse it with a diagnostic naming the property spelling —
      never a panic. ADR 0002 owns the binding.
- [ ] **The refusal's case** — one `.nvst` under `tests/conformance/error/`
      pinning what the chosen answer prints, in the shape
      `a-finally-runs-when-its-catch-body-throws.nvst` uses.
- [ ] **`python tools/holes.py` re-read** after it lands, to see which item
      the remaining `nvs-ir` panic sites attribute to.

## Backlog
- `[context] modules` names no `nvs-runtime` and no `nvs-diagnostics`
  pattern (docs/agent/loop-goal.toml).
- `array<T> as array<U>` inside a `catch`-reachable position — see the
  playbook's `Core\Csv::format` bullet.
- ADR 0024 § 5's `string as Core\Html\Markup` waits on M7
  (docs/implementation-plan.md, `Open now`).
