# Handoff

## State

**Spec § 10's exception surface is whole except one read.** The constructor takes ADR 0063 R2's options
bag — `new RuntimeError("…", {previous: $e})` — and the chain is stored, read back and released; the bag
is optional by construction, so every existing one-argument `new` is unchanged. `mwl_types::error_lib`'s
module doc is the home for the seeded shape, for why `location` is the *throw* site (and why a rethrow
moves it), and for the one gap left. The flattening at the call site was already generic over any
signature carrying a `Ty::Options` parameter; what this needed was `lower_options_arg` widening each
option into the slot its declared type erases to, since a compiled MWL function's parameter is typed
where a `Core` helper's is a whole `Value`.

Verify is green (1536 tests, clippy and fmt clean). Conformance is **428**, differential 89. Valgrind is
clean over the new refcount edge — a chained throw/catch in a 200-iteration loop, plus exceptions built
with and without the bag and never thrown. `examples/collect.mwl` still exits 1 at `Core\Out::capture`,
which is the known frontier.

## Next group — a shape receiver's property read, which is one file set

**Shared file set:** `crates/mwl-types/src/expr/members.rs` (the receiver's type, and the
`ExprInfo::Property` it does *not* record) and `crates/mwl-ir/src/lower/expr.rs`
(`lower_property_access`, which panics without one). ADR 0036 § 4 is what specifies the receiver;
ADR 0071 § 5 is what wants it.

- [ ] **1. `$issue->path` lowers.** The checker already *types* a shape field read —
      `members.rs:361` finds it in `Ty::Shape(fields)` and answers with the field's type — but records no
      `ExprInfo::Property` for it (`members.rs:435` is the class-receiver arm that does), so
      `lower_property_access` (`expr.rs:2822`) panics naming the missing entry. A shape value is an
      anonymous methodless instance whose slot order is the interner's **sorted** field order
      (`error_lib::issue_shape`), and `mwl_stdlib::issue` is what builds one — `issue.rs:45` is its
      `SHAPE` label, `issue.rs:49` its `FIELDS` in slot order.
- [ ] **2. `ParseError::issues` becomes readable, and ADR 0071 § 5 gets its case.** With slice 1 landed,
      a `.mwlt` case can walk the list a failed decode throws and print every offending field's dotted
      path — the one-throw-lists-every-bad-field rule that ADR is built around.
      `tests/conformance/error/a-parse-error-carries-an-issue-list.mwlt` is the case that counts them
      today and is the one to extend rather than duplicate.
- [ ] **3. An anonymous shape literal is readable the same way.** ADR 0036 § 4's own receiver, not just
      the `Core\Issue` one — same two anchors, and worth pinning in `tests/conformance/lang/` so the
      rule is not recorded only through `Core`.

## Backlog

- `Core\Out::capture` — § 12's last member, and `examples/collect.mwl`'s first failure
  (`docs/implementation-plan.md` *Open now*; lands with M4S's sink work).
- `Core\Json::decodeAs<T>` — § 6's last member (`mwl_stdlib::json` gap 2).
- A `Core` collection is still not assignable to an `Iterable<T>` **parameter**, so
  `Core\Arr::from($set)` does not type-check: `expr::assign::class_satisfied` asks
  `mwl_hir::hierarchy::implements_interface`, which knows only the user `ClassGraph`. The runtime half
  already works (`mwl_runtime::sequence::drain` drives the same three names).
- `compareTo` on a `Core` instance is still invisible to `Core\Heap`'s ordering — the mechanism is now
  there (`instance`'s dispatch roster), so it is a row per class plus the transfer wrapper
  (`mwl_stdlib::heap`'s own known gap).
- `$e->previous->message` needs the value bound and narrowed first; a property access straight off a
  `Ty::Tagged` receiver is that type's own known gap (`mwl_ir::ty`).
- Stage 4's counts are their own work: conformance 428 of 600, differential 89 of 150.
- `docs/spec/02-php-migration.md` is 31% classified, one pass per PHP domain
  (`python tools/check-migration.py`).
