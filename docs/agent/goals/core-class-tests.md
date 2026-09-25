---
milestone: post-parity
---
# Loop goal 63 — a `Core` class is a name a type test can walk

`$v instanceof Core\Time\Date` and `$v as Core\Time\Date` answer at run time instead of being refused
where they are written, so a `mixed` holding a `Core` instance narrows to the class it actually holds.
A value that arrives as a `mixed` — decoded back from an isolate's answer, handed over by `Core\Debug`,
taken by any member declaring `mixed` — becomes usable by naming its class, the same way a value of a
class the program declares already does. Afterwards `crates/nvs-runtime/src/graph.rs` owns no gap.

## Why here

Directly after goal `worker-placement` and in front of goal `gap-zero`, because both halves of the
answer are already built and only the checker's roster is missing. The descriptor a test would walk is
published: `nvs_stdlib::instance::class_descriptors` (`crates/nvs-stdlib/src/instance.rs:488`) hands the
backend a `(name, *const ClassDesc)` pair for every `Core` class, which is how a folded `` html`…` ``
constant already reaches one. The run-time side is built too — `nvs_runtime::Ctx::class_desc`
(`crates/nvs-runtime/src/ctx/error.rs:261`) asks the `Core` resolver after the program's own table, which
is why a decoded `Core\Time\Date` arrives back under its own descriptor with its slots intact. What
refuses the program is one roster in `nvs-types`, in two places: `infer_instanceof`
(`crates/nvs-types/src/expr/members.rs:298`) reports `E0496` for a `Core` class because it "has no
descriptor laid out for the test to walk", and `reject_unconvertible`
(`crates/nvs-types/src/expr/operators.rs:1765`) reports `E0711` for the same class as a conversion
target with no class to test against.

Goal `gap-zero` is what needs it: that goal's gate is that no register item names a goal, and
`crates/nvs-runtime/src/graph.rs` gap 1 names this one. It could not have been written earlier: the
crossing that hands a program a decoded `Core` instance is what made the question reachable, and it is
that crate's own walk (`crates/nvs-runtime/src/graph.rs`).

## Stage 0 — the catch-up

The sentences on disk that go wrong the day this goal is green, each with the file that holds them:

- `crates/nvs-types/src/expr/members.rs:44-49` — *a `Core` class has no descriptor laid out for the test
  to walk, so all three are `E0496`*. Three refusals share that code and only this one stops being true;
  the dynamic form and the enum stay exactly as they are.
- `crates/nvs-ir/src/lib.rs:558-562` — *Every other object target names no class to test against — plain
  `object`, a shape, a `callable`, a `Core` class*. The roster loses its last member and keeps the rest.
- `crates/nvs-runtime/src/graph.rs:72-86` — the `# Known gaps` block this goal closes, rewritten as what
  the round trip then does rather than as what it cannot do.
- `crates/nvs-diagnostics/src/lib.rs`'s docs on `E_INSTANCEOF_NOT_A_CLASS` and
  `E_UNTESTABLE_CONVERSION_TARGET`, wherever they name a `Core` class as a member of either roster.

## Stage 1 — the floor

Goal `worker-placement`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the keystone: a `Core` class is a testable class name

The one question `nvs-types` asks — *does this name resolve to a class whose descriptor a test can walk?*
— answers yes for a `Core` class. `infer_instanceof` stops reporting `E0496` for one and records the
resolved name in `ExprInfo::InstanceOf` the way it already records a declared class, so `nvs-ir` reaches
the descriptor address `class_descriptors` publishes and the run-time test is the walk it already does.
Nothing about the descriptor set moves, and `nvs-types` gains no second table: what it needs is the
class's *identity*, which the registry already answers for every other purpose.

Everything after this is mechanical, which is why it is the keystone: once a `Core` name is testable, the
conversion target below is the same question asked through a different operator.

## Stage 3 — the conversion target, and the lift that is not one

`mixed as Core\Time\Date` becomes `rule:types/unions-and-mixed`'s checked downcast over the descriptor
stage 2 made reachable — `nvs_ir::lower::Lowering::lower_checked_downcast`
(`crates/nvs-ir/src/lower/convert.rs:1171`) is the lowering, unchanged in shape. `reject_unconvertible`'s
roster keeps `object`, a shape and a `callable`, each of which still names no class.

`string as Core\Html\Markup` does not move. `rule:core-classes/html-auto-escape` makes it a **lift**
rather than a test — the operand is a source literal or it is `E0417` — and it stays
`Lowering::lower_markup_lift`'s own arm, never reached through the downcast this stage opens.

## Stage 4 — the three spellings agree

`rule:types/type-test` owns `is`, and whatever it already answers for a `Core` class is what the two
operators above must agree with: one question, three spellings, and no program that can ask it two ways
and get two answers. The conformance cases are written over the same class from all three, and
`crates/nvs-runtime/src/graph.rs`'s module doc is rewritten as what the round trip now does.

## Standing decisions

- **The descriptor set does not move, and no second table is built.**
  `nvs_stdlib::instance::class_descriptors` stays the one publisher of a `Core` class's descriptor
  address, and `nvs_runtime::Ctx::class_desc` stays the one lookup. A session that finds the checker
  needs the identity in a different shape derives it from the registry rather than declaring a copy.
- **A `Core` class is final for the test.** The walk answers the class itself and whatever its descriptor
  already declares; this goal adds no inheritance edge, no interface and no member to any `Core` class.
- **`string as Core\Html\Markup` stays a lift**, per `rule:core-classes/html-auto-escape`. A session that
  finds the downcast reaching it has widened the wrong roster.
- **Not this goal**: `new Core\X()` and a static call through a value, which `reject_dynamic_class_name`
  refuses for the unrelated no-computed-names reason; what the `Core` registry contains; and the
  crossing itself, which `rule:security/isolate-values-cross-by-copy` owns.
- **What it spends** is written per module, per `rule:programs/memory-priority`: the descriptor table is
  per process and already built, and a test that answers costs the walk it already costs for a declared
  class.
- **ADR slots**: one record, taking the next free number, and only if the shape the checker needs for a
  `Core` class's identity is a design choice rather than a lookup. Reading the registry the way every
  other pass reads it opens none.
