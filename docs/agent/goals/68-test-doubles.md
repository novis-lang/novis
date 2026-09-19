---
milestone: post-parity
---
# Loop goal 68 — `Core\Test`'s double half — a double is a shape checked against an interface, and a call is asserted after the fact

`Core\Test::double<T>` and `partial<T>` exist, and a double **is** a `T`: a shape of closures the checker
compares to the interface's method set at the call site, refused with a diagnostic for a method the
interface does not declare and for one the double leaves unimplemented, and accepted wherever a `T` is
taken. `assertCalled` and `assertNeverCalled` read the calls the double recorded, against a
compile-checked method reference. `assertCompletes` runs a body under the test's own clock and fails
when it has not finished within the duration named. `rule:testing/doubles` and
`rule:testing/interaction-after-the-fact` read `shipped`, and the five `§13 Test::…` keys are struck from
`crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt`.

## Why here

The user's call of 2026-09-18, made while goal `gap-zero` held the run: the five members were the last
`Core\Test` rows with no class behind them, tagged `unowned` because no goal built them and no
milestone's plan stated them, and `gap-zero` may not invent an owner. They cannot be a milestone's
work, because nothing later is needed to build them — every piece they stand on is landed:
`rule:types/shape-type`'s structural check, `rule:types/closure-literal`'s closure objects,
`nvs_runtime::dispatch`'s by-name method call, `nvs_types::generics`' binding of a `Core` member's `T`
and the call-site `<T>` that `Core\ObjectSet<Tag>` already writes, and `Core\Test::advance`'s virtual
clock. So this is a goal, and it sits directly in front of `gap-zero` because that goal's ratchet half —
`unowned` stops being an owner for a key — needs every key to name a goal that has walked or a milestone
still ahead. Goal `bigint` owns the sixth key and runs between the two; its file set is disjoint.

## Stage 0 — the catch-up

One sentence on disk says the half does not exist: `crates/nvs-stdlib/src/test.rs`'s module doc names
the assertion roster, the request and the outbound answers and nothing of doubles. It gains a section on
the double half, which is where the design calls stages 2 and 4 leave open are recorded once made. The
ratchet header at `crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt:19` and the gap
index already name this goal; nothing to do there.

## Stage 1 — the floor

Goal `decided-closures`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. It holds
every closure goal's checks and goal `one-type-test`'s. Never traded.

## Stage 2 — the keystone: a double exists at runtime and is a `T`

**The two rows and the runtime land in one group**, because a registered row forces its helper:
`crates/nvs-stdlib/src/test.rs:2968`'s `every_row_names_a_symbol_this_module_claims` and
`crates/nvs-stdlib/src/lib.rs:518`'s `every_registered_member_has_an_implementation_address` each sweep
every row in `registry::CLASSES` for an address, and
`crates/nvs-stdlib/tests/conformance_coverage.rs:52` wants a case writing `Core\Test::double<`. So
`crates/nvs-stdlib/src/test.rs:321`'s `CLASS` gains `double` and `partial` — a written `T`
(`crates/nvs-stdlib/src/registry.rs:554`'s `CoreTy::Written`) and a return type of that `T` — with the
card, the `address()` arm, the helper and the case that make the row honest, and the two lines struck
from `crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt:29-30`. `double` alone is a
group's worth, and `partial`'s ratchet line may be struck a session later.

The checker's only work here is the plumbing that hands the helper what it needs — `T`'s own descriptor,
which `crates/nvs-stdlib/src/registry.rs:3030`'s `WRITTEN_CLASS_MEMBERS` and
`crates/nvs-types/src/expr/args.rs:1671`'s `written_class_of` already carry for a written *class*. An
interface has a descriptor for the same reason a class does: `nvs_ir::ir::Class::conforms` names it and
`crates/nvs-codegen/src/lib.rs:1483` defines every entry it names. Stage 3 is the refusals.

The runtime's half, `nvs-stdlib` and `nvs-runtime`. A double is a `Core`-owned instance under
`crates/nvs-stdlib/src/instance.rs`'s first decision — an ordinary object whose slots hold Novis
values: one slot per interface method holding the closure or `null`, one slot for `$real` (`partial`)
or `null`, one slot for the call record, an `array` of `{method: string, args: array<mixed>}` shapes
appended per call. Its `ClassDesc` names `T` in the slice of every other class and interface an instance
also is (`crates/nvs-runtime/src/object.rs:318`), which is what a `$x is Clock` and an interface-typed
parameter read, and its method table (`:324`) has a `MethodRow` per interface method whose `code` is a
native trampoline: append to the record, then reach the slot's closure through
`nvs_runtime::call_closure` (`crates/nvs-runtime/src/closure.rs`), or delegate through
`nvs_runtime::dispatch::call_method` on `$real`.

**One descriptor per `(interface, call site)`.** Either `nvs-ir` builds it while lowering the call, the
way `crates/nvs-ir/src/lower/expr.rs:2885`'s `lower_new` already lowers `new` on a `Core` class to that
class's own helper, or the runtime builds it on first call from the interface's method list — the
session picks whichever `MethodRow::code`'s calling convention makes cheaper, and records the pick and
its reason in `test.rs`'s module doc. From the caller's side a call on a double is
`InstKind::CallVirtual` on the receiver's descriptor — the same by-name lookup an interface call makes
today (`crates/nvs-ir/src/ir.rs:822`) — so nothing on a non-test path changes.
`MethodRow::code` is a bare address with no data word beside it
(`crates/nvs-runtime/src/object.rs:628`), so a trampoline carries its slot in its own *identity* — one
native function monomorphized per slot, against a ceiling the double's method count is checked at — or
the pick is the other one.

## Stage 3 — the refusals: `T` is an interface, and the shape answers its method set

The checker's half, in `nvs-types`, over the rows Stage 2 landed. `T` arrives at
`crates/nvs-types/src/expr/calls.rs:126`'s `check_written_type_args`, the path
`new Core\ObjectSet<Tag>()` already takes:

1. **`T` must resolve to an interface.** A class, a scalar, a shape or a `Core` class is refused naming
   the member; the reserved global interfaces (`crates/nvs-hir/src/interfaces.rs`) are admitted like any
   other.
2. **`$shape` is a shape literal whose every field is a closure.** A field named like no method of `T`
   is refused — *`Clock` declares no method `tomorrow`* — and, for `double`, a method of `T` with no
   field is refused — *`Clock::now` is not implemented by this double*. For `partial` a missing field is
   the delegated case, and `$real` must be assignable to `T`.
3. **Each closure's parameters and return are checked against the method's signature** by the ordinary
   assignability check, off the closure literal's declared types — `callable` carries none
   (`rule:types/callable-is-a-closure`).
4. **The result has type `T`**, so `new Session($clock)` type-checks under no further rule.

Diagnostics: two new codes in `crates/nvs-diagnostics/src/lib.rs`, one per refusal in item 2; item 1
draws the argument-mismatch code the row already carries. A method with a default body
(`rule:classes/interface-default-methods`) is not required of a double — a default is an
implementation — and a private interface method (`rule:classes/interface-private-methods`) is not
nameable in one.

## Stage 4 — the two assertions read the record

`assertCalled($double, T::method, {times?: uint, with?: array<mixed>})` and
`assertNeverCalled($double, T::method)`. **The method reference**: `T::method`, written bare in these
two argument positions, is a compile-checked reference — the checker resolves it against `T`'s declared
methods and refuses a name `T` lacks — and it lowers to the method's name as a `string`. It is admitted
only where a registry row's parameter is a new `CoreTy` variant for a method reference, and nowhere
else, so no general "method as a value" expression enters the language. `times` compares the count
exactly; `with` compares each recorded argument under `assertEquals`'s comparison
(`rule:testing/assertions-are-typed`'s rows), and a call written without it matches on the name alone.
A failure is a `Core\Test\Failure` whose message names the method, the count expected and every call
the record holds.

## Stage 5 — `assertCompletes` under the test's clock

`assertCompletes(callable $body, {within: Duration} $options)`: the body runs as a child task under the
test's task (`rule:testing/task-tree-and-virtual-clock`), the virtual clock advances by `within`
exactly as `Core\Test::advance` advances it, and a body still running then fails the test naming the
duration. Wall-clock time is never read, so two runs are byte-identical. Where no test fixed a clock
it refuses with the `LogicError` `advance` refuses with — a `.nvst` case is top-level statements and
is never inside a `#[Test]`, so that refusal is the only side of the member a case can observe; the
accepted side is a runner-observed test in `crates/nvs-cli`, beside `a_test_at_a_fixed_clock_reads_that_clock`.

## Stage 6 — the proofs and the rulebook

ADR 0079 § *Verification*'s § 10 bullets as conformance cases: a double missing a method, and one
declaring a method the interface lacks, each an `--EXPECTF-ERROR--` case under
`tests/conformance/reject/`; a double satisfying an interface passed to a parameter of that type under
`tests/conformance/core/`; `assertCalled` with `times` and `with`, `assertNeverCalled`, and a `partial`
delegating what it does not override; `assertCompletes` refusing without a clock as a case, and on a
body that finishes and one that does not as two `nvs-cli` runner tests.
`rule:testing/doubles` and `rule:testing/interaction-after-the-fact` flipped to `shipped` in
`docs/rules/testing.json`, then `python tools/rules.py --render`. The five keys struck, in the same
edit as the rows that register them.

## Standing decisions

- **No new syntax.** A double is a shape literal of closures, the method reference is admitted in two
  argument positions only, and `assertCompletes` takes a closure. If bare `T::method` cannot be read as
  an expression in argument position without a grammar change, the fallback is the first-class-callable
  spelling `T::method(...)`, which `crates/nvs-syntax/src/ast.rs:450` already parses; record the choice
  in the two rules' fragments and in `test.rs`.
- **A double is an ordinary object.** `instance.rs`'s decision binds: no new heap shape, no new `Tag`.
  Its descriptor is never a name a program can spell, and its frames appear in a backtrace as the
  closure's own, which the author wrote (`rule:testing/doubles`).
- **Strict, always.** No default answer for an unimplemented method: the checker refuses the double.
  There is no loose mode to add, because nothing legal could be returned.
- **What it spends**: one object per double plus one array append per recorded call, charged to the
  test's request and released with it. Nothing on a served request — `Core\Test` is unreachable from a
  build (`rule:testing/tests-never-reach-a-build`).
- **ADR slots**: none. ADR 0079 §§ 10, 11 and 16 decided this; a design call the session must make is
  recorded in `test.rs`'s module doc, never in a new record.
- **Not this goal**: `Core\BigInt` (goal `bigint`); the feature proofs beyond ADR 0079's own bullets (goal
  `dossier`); mutation testing, which is M10's.
