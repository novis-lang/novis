# Handoff

## State

**Goal 1 of the parity program has just started; nothing of it has landed yet.** The previous goal —
M4, language completeness — reached its whole acceptance list, and that list is now this goal's Stage 1
floor. A failure there is a real regression and never a scope question: nothing in this goal lowers
anything, so `nvs-ir` and `nvs-codegen` should not change at all.

M4S Part I is **registered** — `crates/nvs-stdlib/tests/spec-members-outstanding.txt` holds no keys — and
registered is not finished. Three tools are the worklist and no session re-derives one:
`python tools/gaps.py` (depth per class, and the unasserted error paths),
`python tools/check-migration.py --report` (the PHP names with no row), and `python tools/loop.py --list`
(the acceptance list itself).

## Next group

**ADR 0061 § 3's program enumeration.** It is Stage 0 and it is first because three later items are the
same scan — `#[Route]`'s table, `#[Command]`'s table and the OpenAPI emitter all filter this enumeration
(ADR 0077 § 5 says so outright), so writing any of them first means writing the walk three times.

One file set: `crates/nvs-hir/src/autoload.rs`, `crates/nvs-hir/src/hierarchy.rs`,
`crates/nvs-types/src/expr/calls.rs`.

- [ ] **The program walk answers "which non-abstract classes implement `T`".** `AutoloadMap::build` at
      `crates/nvs-hir/src/autoload.rs:162` already turns a program's `autoload` sites into prefix → roots
      and `resolve` turns a `QName` into the file that declares it; what is missing is the other
      direction — enumerate every class the program declares, filter by `implements_interface`
      (`crates/nvs-hir/src/hierarchy.rs`, already there), and **sort by fully-qualified name** so the
      order never depends on filesystem enumeration. ADR 0061 § 3.
- [ ] **`Core\Program::implementing<T>()` expands at its call site** to an array literal of `new`
      expressions, one per enumerated class — so the instances are per-request like every other object
      and nothing crosses an isolate boundary. The expansion belongs beside the other generic call
      handling in `crates/nvs-types/src/expr/calls.rs`. A `T` that is not an interface type is a
      diagnostic, and so is an enumerated class with no no-argument constructor: ADR 0061 § 3 names both,
      and the second is the one that is easy to leave out because it only fires on a real program.
- [ ] **`examples/program.nvs` and its case.** Three classes implementing one interface across two files
      reached by `autoload`, printed in name order. The fixture is what proves the walk sees a class it
      was never `require`d to see, which no unit test over a synthetic module does.

## Backlog

- `nvs_types::derive::ATTRIBUTES` holds five names where ADRs 0071, 0077, 0085, 0086 and 0102 name
  twelve. Items 2–6 are the rest, and they are one group after this one.
- `python tools/gaps.py`'s *unasserted error paths* list stood at 68 sites, 65 of them `Fault::fatal`.
  Judge before writing: a `fatal` may be an invariant no program reaches, and the answer there is a
  comment at the site, not a case.
- ADR 0057's folding pass does not exist; three modules mention the ADR in a doc comment and nothing
  reads any of them.
