# Handoff

## State

**M4 — language completeness.** Item 19 is closed as **two refusals**, not a lowering: a
`&$x` parameter is a contract between the two ends of one call, and the two
declarations that break that ordering are refused where they are written. A generator
declaring one is **E0492**, a closure declaring one is **E0493**. The third site needed
no rule — only `lower_method`'s parameter loop ever binds a `Ty::Ref`, so with E0492
standing no `&$x` binding can be live across a `yield`, and `lower_yield`'s assert is an
internal-consistency check now. `python tools/holes.py` is at **29 sites, 7 items**.

`verify.py` 6 of 6 green — conformance **592**, differential **165**, 1629 unit tests.
No new refcount edge, so no `leak-check.sh` run was owed: both slices only report.

Three facts recorded where they belong rather than here: the frame-lifetime rule and why
it is not a lowering we declined to write are `docs/adr/README.md` § *Decisions taken at
project start*; each code's own reasoning is its `Code::new` doc comment in
`mwl-diagnostics`; and the three asserts that are now internal-consistency checks say so
in `lower_generator`, `lower_yield` and `lower_closure`'s own `# Panics` sections.

**What item 19 still owes is a lowering**: a closure *capturing* an enclosing `&$x`
parameter. ADR 0031 § 2's capture is by value, so it is a snapshot of the cell at the
literal — one `RefLoad`, not a refusal. It is the next group's first slice.

**Gap in the pack**: `[context] adrs` printed ADR 0031 § 2 but not § 4 (`callable` is
opaque) or ADR 0053 § 4 (calling a generator runs no user code), which are the two
sections both refusals actually rest on. Both were reachable second-hand from
`check_fn_literal`'s and `check.rs`'s own doc comments, but the manifest should name
them.

## Next group

**The `lower/expr.rs` refusals, which `holes.py` files under item 17 because they share
the file.** One file set: `crates/mwl-ir/src/lower/expr.rs`, with
`crates/mwl-ir/src/lower/closure.rs` and `Lowering::pointee_of`
(`crates/mwl-ir/src/lower/mod.rs:1941`) for the first slice.

- [ ] **A closure capturing an enclosing `&$x` parameter** —
      `crates/mwl-ir/src/lower/expr.rs:2839`, the assert inside `lower_closure_literal`'s
      capture loop (`:2828`). This one is a **lowering**, not a refusal: ADR 0031 § 2
      captures by value, so the field takes a snapshot of the cell's current value. The
      move already exists one arm away — a `Ty::Ref` variable read is a `RefLoad` at
      `pointee_of(name)` (`expr.rs:139`), and the capture is that read plus the retain
      the loop already emits for a refcounted value. `crates/mwl-ir/src/lib.rs`'s gap 9
      states what is owed. Closes item 19 outright.
- [ ] **An object literal writing one field name twice** —
      `crates/mwl-ir/src/lower/expr.rs:3474`. ADR 0036 § 2; the panic says `mwl_types`
      records the shape with the later value, so the answer is a diagnostic where it is
      written, in the same shape as this session's two.
- [ ] **A property access with no resolved declaring class** —
      `crates/mwl-ir/src/lower/expr.rs:3345`. Judge it first: E0477 and E0480 already
      refuse an erased receiver by name, so this may be an internal-consistency reword
      like `lower_yield`'s rather than a new rule.

## Backlog

- Item 7's five sites are one coherent group in `crates/mwl-ir/src/lower/stmt.rs` — the
  nullsafe property assignment target at `:664` is the headline (`python tools/holes.py --item 7`).
- The 2 unattributed sites, `crates/mwl-codegen/src/ty.rs:116` and `:121` — no item in
  `docs/agent/loop-goal.md` names them.
- `Class::method(...)`, the first-class callable spelling — `mwl-ir` gap 1, `crates/mwl-ir/src/lib.rs`.
- 17 of the 32 named `.mwlt` cases are still to write — `python tools/holes.py --cases`.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
