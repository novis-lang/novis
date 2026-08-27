# Handoff

## State

**M4 — language completeness.** The property-access half of item 17's shared file is closed. A
`mixed` receiver is ADR 0036 § 4's fourth erased shape — read and write alike are the name-keyed
fetch — and every receiver whose declared type can hold no object at all is **E0495** where it is
written (ADR 0007 § 7 **row 13**, new). `python tools/holes.py` is at **24 sites, 7 items**.

The one fact worth carrying: `mwl_object_slot_get`/`mwl_object_slot_set` now take the receiver as a
whole `Value` **by address** rather than as an `ObjHeader` pointer, and check its tag where they
already check the name. `ReceiverProof` in `crates/mwl-ir/src/lower/expr.rs` is what selects that —
`Erased` emits no `Untag`, `Proven` still does — and the playbook bullet above says why an
unchecked untag over a `mixed` is a segfault rather than a panic.

`verify.py` 6 of 6 green — conformance **595**, differential **166**, 1630 unit tests.
`tools/leak-check.sh` green over fixtures exercising both new throw edges with a freshly-built
receiver live.

Facts recorded where they belong rather than here: ADR 0007 § 7 row 13 owns the divergence; ADR
0036 § 4 owns the `mixed` trigger; `mwl_runtime::mwl_object_slot_get`'s own doc comment owns the
by-address receiver; `mwl-ir`'s crate doc (`crates/mwl-ir/src/lib.rs`) says all four § 4 receivers
lower; `E0495`'s reasoning is its own `Code::new` doc comment.

## Next group

**The last two refusals in `crates/mwl-ir/src/lower/expr.rs`** — the file this session already had
open, with `crates/mwl-types/src/expr/members.rs` (where `instanceof` is checked and recorded) and
`crates/mwl-diagnostics/src/lib.rs` (next free code is **E0496**).

- [ ] **An `instanceof` whose right-hand side resolved to no class** —
      `crates/mwl-ir/src/lower/expr.rs:3898`, the `ExprInfo::InstanceOf` lookup, against
      `crates/mwl-types/src/expr/members.rs:154-180`, which records one only for a written class
      name. The remaining route is the dynamic `$x instanceof $name` form; MWL has no dynamic class
      names (`$$var` and `eval` are already rejected, ADR 0007 § 2), so this is almost certainly a
      diagnostic naming that rule rather than a lowering — measure which spellings reach it first,
      the way this session's second playbook bullet says to.
- [ ] **An `as` whose target `closed_literal_set` cannot build** —
      `crates/mwl-ir/src/lower/expr.rs:4146`, the `other =>` arm of the match inside
      `closed_literal_set` (`:4092`). ADR 0047 names three atoms; what a union may also contain is
      what decides between widening the set and refusing the conversion target by name.

## Backlog

- Item 1's 11 sites are catch-alls only (`mwl-codegen`'s mismatched-representation refusal) —
  `docs/implementation-plan.md` § *Open now*.
- The 2 unattributed sites in `crates/mwl-codegen/src/ty.rs` belong to no item —
  `python tools/holes.py`.
- Item 25's 2 sites are `lower_decl_type`/`lower_checked_ty` catch-alls for `decimal`, `never`,
  `iterable`, `self`/`static`/`parent` as *declared* types — `docs/implementation-plan.md`.
- 17 named `.mwlt` cases still to write — `python tools/holes.py --cases`.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
