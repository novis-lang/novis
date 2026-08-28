# Handoff

## State

**M4's frontier is ADR 0014, closed whole.** `PropertyObserver` is a reserved global
interface, seeded, checked, lowered and pinned; ADR 0014 gained the § *Verification*
section M4's acceptance has always named and which did not exist.

- **The mechanism has one home each and is not restated here**: the roster in
  `nvs_hir::interfaces::RESERVED`, the two member signatures in `nvs_types::iter_lib`,
  the compile-time "does this class observe" question in
  `nvs_types::expr::members::observer_calls` (`crates/nvs-types/src/expr/members.rs:596`),
  and the emission in `nvs_ir::lower::expr::Lowering::emit_observer_call`
  (`crates/nvs-ir/src/lower/expr.rs:2757`). ADR 0014 § *Verification* owns the three
  boundaries the sections above left open (own-hook access, `static`, recursion).
- **`holes.py --cases` now names four**, and the next two are **not** case-writing
  slices either — this session's was not, and the probe says the next is not:
  `implements Greets by $inner;` parses and checks and then dies at run time with
  *"a method with no body was called, and no class in the receiver's chain declared
  one"*, so ADR 0043 § 3's forwarding has no lowering at all.
- **`orient.py`'s pack was complete for this item**, but its `[context] modules` still
  has no `nvs-runtime` and no `nvs-diagnostics` entry, and for this item it also had
  no `nvs-hir` one — the reserved-interface roster is where the work started.

## Next group

**ADR 0043 § 3's `implements I by $field;` delegation, which is one hole and then its
case.** The file set is `crates/nvs-types/src/`, `crates/nvs-ir/src/lower/` and
`tests/conformance/class/` — the same three this session had open, and the same
`ExprInfo`/`lower_call` seam.

- [ ] **Find where a delegated interface method resolves and why it lowers to a
      bodiless target.** ADR 0043 § 3. The failure is `nvs_runtime`'s
      `nvs_abstract_method`, reached from `InstKind::CallVirtual`'s `fallback: None`,
      so the checker resolved `Outer::greet` to the *interface*'s bodiless declaration
      — `crates/nvs-types/src/expr/calls.rs`'s method resolution and
      `crates/nvs-ir/src/lower/call.rs:1` are the two ends. `nvs_hir` already parses
      the `by $field` clause (`crates/nvs-syntax/src/parser/decl.rs`).
- [ ] **Lower the forward**: a delegated method is a call on the named field, with the
      receiver replaced and the arguments passed through. Mirror
      `crates/nvs-ir/src/lower/expr.rs:2757`'s shape — one `FieldGet` for the field,
      then one call on it — and decide there whether the forward is synthesized as a
      method body or emitted at the call site.
- [ ] **`tests/conformance/class/a-delegated-interface-forwards-to-the-object-it-names.nvst`**
      — ADR 0043 § 3. `python tools/holes.py --cases` names it.

## Backlog

- `tests/conformance/lang/an-attribute-is-retrieved-by-its-own-type.nvst` — ADR 0046;
  an attribute *parses* on a class today, but nothing was probed about retrieval.
- The two `Core`-shaped named cases (`a-dump-renders-one-record-and-redacts-a-secret`,
  `a-test-attribute-builds-a-table-the-runner-reports`) — `holes.py --cases`.
- A `require` whose path is not a string literal runs nothing, silently, in both forms
  — `nvs_hir::requires`' own known gap.
- `[context] modules` in `docs/agent/loop-goal.toml` has no `nvs-runtime`,
  `nvs-diagnostics` or `nvs-hir` entry.
- ADR 0014 *Revisiting*'s "an interface with no implementations yet" is now only true
  of the stdlib; leave it until something ships one.
