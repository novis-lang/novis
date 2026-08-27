# Handoff

## State

**M4 — language completeness**, and **item 25 is closed**. `object` is a declared type wherever a
class name is one — local, parameter, return, property field, `array<object>` element, `?object`,
a union member — and it erases to the same pointer, so identity, `instanceof`, `clone` and `as
ClassName` all still work through it. The representation arms were already on disk (`lower_decl_type`
at `crates/mwl-ir/src/lower/mod.rs:2211`, `erase_checked_ty` at `mod.rs:2372`); what this session owed
was item 25's other half, the check that nothing below reads a class *label* off an erased operand.
Exactly one reader does: an instance **method call**, which had no resolved target to record and
panicked in `mwl-ir`. It is `E0477` at the checker now
(`mwl_types::expr::calls::report_method_on_erased_receiver`, `crates/mwl-types/src/expr/calls.rs:429`),
naming `instanceof` and `as ClassName` as the two narrowings that do resolve. The decision — refuse
rather than dispatch, because a call needs a signature and a return type that an erased receiver has
neither of, and because refusing is the reversible half of the pair — is recorded in
`docs/adr/README.md` § *Decisions taken at project start*.

`python tools/holes.py` still prints item 25 at 2 sites and it is not the `object` work: both are
`lower_decl_type`/`lower_checked_ty`'s **catch-alls**, and the shapes they still refuse are `decimal`,
`never`, `iterable`, `self`/`static`/`parent`, a shape type and an intersection *as a declared type*.
That is a real hole, in item 25's file, that no item names — the plan's `Open now` says so.

Conformance 566, differential 162, `verify.py` 6 of 6 green. No valgrind leg: this session added a
diagnostic and two `.mwlt` cases, and no refcount edge.

## Next group

**All three slices share `crates/mwl-ir/src/lower/stmt.rs` and `crates/mwl-ir/src/lower/mod.rs`**, and
the design below is already done — do not re-derive it. Take them in order.

- [ ] **A nested `$grid[0][1] = v` writes back** — item 22, ADR 0007 § 5's copy-on-write separation.
      `lower_reassignment`'s `Index` arm is `crates/mwl-ir/src/lower/stmt.rs:781`; it lowers the base,
      emits one `ArraySet` and calls `Lowering::write_back_array`
      (`crates/mwl-ir/src/lower/mod.rs:1801`), whose `other =>` panic at `mod.rs:1840` is what a
      nested target hits. **The algorithm, worked out and checked against the ownership protocol in
      `InstKind::ArraySet`'s doc (`crates/mwl-ir/src/ir.rs:1013`) and `lower_index`
      (`crates/mwl-ir/src/lower/expr.rs:3659`):** flatten the target to `root` plus keys
      `k0…kn`, lower `root` once and each key once (each key is used *twice* — borrowed by an
      `ArrayGet`, then stored by an `ArraySet` — so the existing single-level `if key_aliasing
      { retain }` still applies once per key); descend with `ArrayGet`, **retaining every
      intermediate array** because `ArrayGet` borrows and `ArraySet` consumes one reference; then
      `ArraySet` back up, outermost last, and `write_back_array(root, …)` re-points the root's holder
      exactly as it does today. Do **not** recurse inside `write_back_array` by re-lowering the base
      — that evaluates the base and its keys twice, which PHP does not. Two things to say out loud in
      the module doc: the intermediate retain makes every level's refcount ≥ 2 so a nested write
      **always copies the inner row** (correct, and O(inner) per write — name the write-through
      optimization as the follow-up), and `ArrayGet` models only the happy path, so
      **auto-vivification** (`$grid["9"]["0"] = 1` with `"9"` absent) is not lowered by this and needs
      its own answer. This one *does* owe a WSL `leak-check.sh` leg: the intermediate retain is a new
      refcount edge.
- [ ] **An array-element write through a hooked property is a diagnostic** — the assert at
      `crates/mwl-ir/src/lower/mod.rs:1819`, inside `write_back_array`'s `PropertyAccess` arm. The
      decision is already pre-authorized in `loop-goal.md` § *Standing decisions* (PHP raises
      "indirect modification of overloaded property", so a refusal *is* the PHP-compatible answer);
      it takes a new `E04xx` — `E0477` is now used, so claim the next free one from the pack — and it
      belongs in `mwl_types`, not in `mwl-ir`.
- [ ] **The named case** `tests/conformance/array/a-nested-element-write-separates-only-the-inner-array.mwlt`,
      which `python tools/holes.py --item 22` already owes. Assert the separation both ways: a second
      binding taken *before* the nested write must not see it, and the outer array's other rows must
      still be the same rows. `Core\Json::encode` renders a whole nested array in one line and is the
      strongest assertion available (playbook, *Writing a test case*).

## Backlog

- A method call on a **`mixed`, a union or a scalar** receiver still panics `mwl-ir`
  (`crates/mwl-ir/src/lower/expr.rs:2939`) — the *erased* half is `E0477` now, and the rest wants
  either the same refusal or a checker one. Not attributed to any item.
- `lower_decl_type`/`lower_checked_ty`'s catch-alls (`mod.rs:2232`, `mod.rs:2315`) still have no arm
  for `decimal`, `never`, `iterable`, `self`/`static`/`parent`, a shape type or an intersection as a
  declared type. Currently attributed to item 25, which is closed.
- `public ?T $x = null;` is refused as a property default (`E0472`) for every `?T`. See the playbook
  bullet under *Writing MWL itself*; owner is `mwl_types`' property-default folder.
- `Core\Reflect::typeOf` does not exist, though ADR 0007 § 4 names it as what reports a `uint` as its
  own kind. `mwl_stdlib::registry` is the home.
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
- `Core\Json::decodeAs<T>`'s wider codec-reachable set and its two default-bearing rows
  (`mwl_stdlib::json` gaps).
