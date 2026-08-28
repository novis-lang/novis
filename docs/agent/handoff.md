# Handoff

## State

**M4's frontier is ADR 0043 § 4, which now runs.** `implements I by $field;`
synthesizes a real forwarding method per required member, reachable through the
class and through the interface alike, with a `.nvst` case and a valgrind-clean
fixture behind it.

- **The mechanism has one home each and is not restated here**: the resolution in
  `nvs_types::conformance::resolve_delegations`
  (`crates/nvs-types/src/conformance.rs:150`), the record it rides on in
  `nvs_types::expr_table::Delegation` (`crates/nvs-types/src/expr_table.rs:577`),
  the emission in `nvs_ir::lower::call::delegation_forward`
  (`crates/nvs-ir/src/lower/call.rs:911`), and the method-table row in
  `lower_program` (`crates/nvs-ir/src/lower/mod.rs:673`). Why it is a method and
  not a call-site rewrite is `Delegation`'s own doc comment.
- **Three member shapes get no forward** — `static`, variadic, and any `inout`
  parameter — and each still reaches `nvs_abstract_method` if called. The
  whole-class conformance exemption stays with them, because
  `E_DELEGATE_TYPE_MISMATCH` does not exist and without it a member no forward
  covers cannot be told from one whose field cannot answer it.
- **`orient.py`'s pack was complete for this item**; `[context] modules` still has
  no `nvs-runtime`, `nvs-diagnostics` or `nvs-hir` entry, and `nvs-hir`'s
  `hierarchy` module doc was the one file outside the pack this session had to
  correct.

## Next group

**ADR 0043 § 4's three remaining edges, which are all in the files this session had
open.** The file set is `crates/nvs-types/src/` (`conformance.rs`, `layout.rs`),
`crates/nvs-diagnostics/src/lib.rs` and `crates/nvs-ir/src/lower/call.rs`.

- [ ] **`E_DELEGATE_TYPE_MISMATCH`, and the conformance check made per-member
      instead of whole-class.** ADR 0043 § 4 bullet 1: `$field` must be a declared
      property of a non-nullable class/interface type that satisfies the delegated
      interface. `nvs_hir::implements_interface` answers it; the exemption to
      narrow is `crates/nvs-types/src/conformance.rs:59`, the resolution to check
      inside is `:150`, and the next free code is `E0720`.
- [ ] **A `static`, variadic or `inout` member of a delegated interface gets a
      forward or a diagnostic naming the rule** — never the `nvs_abstract_method`
      `FATAL` it reaches today. The skip is `crates/nvs-types/src/conformance.rs:195`
      and the shape that would have to widen is
      `crates/nvs-ir/src/lower/call.rs:911`.
- [ ] **A promoted constructor parameter is not a property**, so § 4's own worked
      example does not compile. `crates/nvs-types/src/layout.rs:271`'s
      `own_properties` and `:334`'s `flatten_methods` are the two ends; the
      playbook bullet added this session is the symptom.

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
