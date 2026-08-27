# Handoff

## State

**M4 — language completeness.** `true` and `false` are `bool`'s literal types: placed at a
position that names one, widened to `bool` everywhere else, erased to `bool`'s
representation, and tested for membership by an `as` exactly as ADR 0047 § 1's string and
int atoms are. `closed_literal_set` builds its set in one fallible pass, so the catch-all
that was on the worklist no longer exists. `convert` gained the widening row a
heterogeneous set needs (`$s as 1|"a"`, `$s as mixed`) — one `InstKind::Tag`.
`python tools/holes.py` is at **24 sites, 6 items**.

`verify.py` 6 of 6 green — conformance **600**, differential **167**, 1631 unit tests.
`tools/leak-check.sh` green over a fixture exercising all three new edges: the untag after
a proven tag, the widening row over a borrowed operand in a loop and over a fresh one, and
a throwing miss with a refcounted `mixed` live across it.

Facts recorded where they belong rather than here: `mwl_types::ty::Ty::True` owns what the
two `bool` singletons are; `Lowering::convert`'s doc comment owns the four shapes of row it
lowers, the widening one included; `closed_literal_set`'s own comment owns why the target's
atom list is walked once.

## Next group

**The remaining aborts in `crates/mwl-ir/src/lower/expr.rs`**, the file this session had
open, with `crates/mwl-runtime/src/helpers.rs` (where a new `Helper` is implemented),
`crates/mwl-ir/src/ir.rs` (the `Helper` enum) and `crates/mwl-codegen/src/emit.rs` (its
`Signatures` row). The first two are the same missing helper seen from two sides.

- [ ] **A `mixed` in a condition, and `$m as bool`** — `crates/mwl-ir/src/lower/expr.rs:1081`,
      `truthy_convert`'s catch-all. `mixed $m = "a"; if ($m) { … }` aborts the process
      today, which is ADR 0035's whole subject matter over ADR 0007 § 2's one unchecked
      position. One runtime helper applying § 1's table to a tagged value, dispatching on
      the tag the way `Helper::Identical` already does; `convert`'s `(_, Ty::Bool)` arm at
      `expr.rs:1000` then reaches it for free. Not on `holes.py`'s list — it reads
      `expr.rs` and attributes only the dispatch catch-all — so say so in the handoff if
      the tool still misses it after.
- [ ] **`as ?T` over a literal or enum target** — `crates/mwl-ir/src/lower/expr.rs:981`,
      `convert_or_null`'s catch-all, whose message already names this as ADR 0066 § 1's
      available form. `$x as ?"a"` aborts. It is the membership chain
      `lower_conversion` now emits for every other target, answering `null` at the far end
      instead of `Helper::LiteralMismatch` — `lower_literal_membership` at `expr.rs:4290`
      is the thing to parameterize, not to copy.
- [ ] **The `lower_expr` dispatch catch-all** — `crates/mwl-ir/src/lower/expr.rs:258`
      (`holes.py` item 6). Its message lists what is lowered; measure which `ExprKind`
      still reaches it with a scratch file before deciding, per the new playbook bullet.
- [ ] **A `Class::CONST` on a user-declared class** — `crates/mwl-ir/src/lower/expr.rs:247`.
      The value is unmodeled in `mwl_types`, so this is a checker slice before it is a
      lowering one; `E0498` is the next free code if it turns out to be a refusal.

## Backlog

- A `Ty::Tagged` operand converted to `bytes` — no runtime-tag helper; `convert`'s own
  panic at `crates/mwl-ir/src/lower/expr.rs:921` names it.
- `array<T> as array<U>` — ADR 0007 § 2's last unbuilt conversion row, and the reason
  several `Core` refusals cannot be reached from source (`docs/agent/playbook.md`).
- `holes.py` item 25's two catch-alls: `decimal`/`never`/`iterable`/`self`/a shape/an
  intersection as a *declared* type (`crates/mwl-ir/src/lower/mod.rs:2319`).
- `Class::method(...)`, the first-class callable spelling — `mwl-ir` gap 1.
- The two unattributed sites in `crates/mwl-codegen/src/ty.rs:116` and `:121`.
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
