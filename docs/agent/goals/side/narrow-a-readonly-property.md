# Side goal — a null test narrows a `readonly` property, and `readonly` is written once

When this goal is green, `if ($this->p != null) { $this->p->x; }` compiles when `p` is `readonly`, through
`$this` and through any variable, with all four narrowing spellings. The narrowed read keeps a run-time
null check, so a write the checker missed is an error and never an unchecked read. `readonly` is really
written once: only through `$this`, at most once on each path of the declaring constructor, and never on
a hooked property. A test of a mutable property still narrows nothing, `rule:types/narrowing` says why,
and E0459's help says that a test does not narrow a property and shows the two ways that work.

## Why a side goal

A user report asked for it, and the user placed all four items in one side goal on 2026-10-05. Nothing
on the chain waits for it. It touches `crates/nvs-types` and `crates/nvs-ir`, and the chain's live goal
works in `crates/nvs-codegen`, so the two runs share no file.

## What is on disk today, measured

All four probes ran on `main` at 13449b191 with the release binary.

- **A property never narrows.** `rule:types/narrowing` (`docs/rules/types/narrowing.md:29-31`) makes the
  subject of every spelling a variable. The narrowing state is `LocalScope::narrowed`, a map from a
  variable's name to its proved type (`crates/nvs-types/src/locals.rs:153`), filled by `narrow`
  (`locals.rs:451`) and dropped by `overwrite` and `drop_narrowing` (`locals.rs:291`, `locals.rs:309`).
  A narrowed read is recorded as `ExprInfo::NarrowedRead` (`crates/nvs-types/src/expr_table.rs:784`,
  written at `crates/nvs-types/src/expr/mod.rs:308`) and `nvs-ir` discharges it with an **unchecked**
  `Untag` (`crates/nvs-ir/src/lower/expr.rs:1365-1383`; why it is unchecked: `locals.rs:45-50`).
- **E0459** is reported in `strip_nullsafe_receiver` (`crates/nvs-types/src/expr/members.rs:1173-1209`),
  called from `members.rs:1131` and `crates/nvs-types/src/expr/calls.rs:73`, which both hold the receiver
  expression. Its help always says "test it first", also when the receiver is a property the line above
  tested. The card is `crates/nvs-diagnostics/src/lib.rs:1698`.
- **`readonly` is not written once.** The only check is `reject_readonly_write`
  (`crates/nvs-types/src/expr/assign.rs:1114-1147`), and it asks only whether the write is inside the
  declaring class's constructor (`assign.rs:1130`). Three programs show it:
  - a constructor writes `$other->p = null` on another, already built instance, and the other
    instance's `p` changes from a `Point` to `null`;
  - a constructor writes `$this->p` twice, and the second value is kept;
  - `public readonly ?Point $p { get => null; }` compiles, and every read returns `null`. PHP 8.4
    refuses a hook on a `readonly` property, and `rule:classes/property-hooks` keeps that interaction.
- **Writers the checker does not see.** A write through an erased (shape) receiver names no class, so
  `reject_readonly_write` cannot ask it anything (`assign.rs:1107-1111`). Whether a class instance can be
  written through a shape view, by reflection, by filling an object from a database row or JSON, or by a
  `clone` that changes values, is *not checked*.
- **The constructor pass.** `crates/nvs-types/src/ctor_init.rs` is the flow pass over a constructor body
  that proves every property is assigned on every path (`rule:classes/definite-property-initialization`).
  "At most once on each path" is the same walk with the opposite question.
- **Cases today.** `tests/conformance/reject/readonly-is-written-once.nvst` pins E0782 for a write from a
  method and from outside the class. `tests/conformance/reject/a-nullable-receiver-needs-the-nullsafe-arrow.nvst`
  pins E0459. The feature `lang:classes/nullable-objects-and` is complete (`bun nv proofs --id`).

## Stage 1 — the floor

Main's carried floor, which a side run is always checked against. Never traded.

## Stage 2 — `readonly` is written once

**Does:** Refuses every second write of a `readonly` property, and a hook on one, so a value read from a
built object never changes.

File set: `crates/nvs-types/src/expr/assign.rs`, `crates/nvs-types/src/ctor_init.rs`,
`crates/nvs-types/src/check.rs`, `crates/nvs-diagnostics/src/lib.rs`, `tests/conformance/reject/`.

- **Only through `$this`.** `reject_readonly_write` also refuses a write whose receiver is not `$this`,
  inside the declaring constructor too. Same code, E0782, with its own headline.
- **At most once on each path.** `ctor_init.rs` refuses a write on a path where the property may already
  be assigned, including a promoted parameter's own assignment and a write inside a loop. Same code.
- **No hook on `readonly`.** A `readonly` property that declares a `get` or `set` hook does not compile.
  One new diagnostic code, taken right before it is written.
- **The other writers.** Check each writer the checker does not see (the list above). Each one either
  refuses a `readonly` property or is shown not to reach one, with a `file:line`. One that writes it is
  fixed in this stage. One that cannot be fixed here becomes a gap record under `data/gaps/`, and the
  checked read of stage 3 keeps it from being a memory hole.
- **Cases**, one per rule above: a constructor writing another instance, a second write on one path, a
  write in a constructor loop, a hooked `readonly` property.

## Stage 3 — a null test narrows a `readonly` property

**Does:** Lets the four narrowing spellings narrow `$v->p` when `p` is `readonly`, and keeps a run-time
null check at the narrowed read.

File set: `crates/nvs-types/src/locals.rs`, `crates/nvs-types/src/expr/members.rs`,
`crates/nvs-types/src/expr/mod.rs`, `crates/nvs-types/src/expr_table.rs`,
`crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-types/tests/narrowing.rs`,
`crates/nvs-ir/tests/narrowed_property_reads.rs` (new), `tests/conformance/class/`.

- **The subject** is a path that starts at a variable (`$this` included) and continues through one or
  more `readonly` properties without hooks: `$this->p`, `$v->p`, `$v->p->q`. A path with a mutable or
  hooked link, a static property, an element, or a computed name is not a subject.
- **When it is dropped.** A write to the root variable drops every path under it, through
  `LocalScope::overwrite` and `drop_narrowing`. A loop body drops it as it drops a variable's. Nothing
  else drops it, because nothing else can change the value.
- **The checked read.** The narrowed property read records its own `ExprInfo` entry, never
  `NarrowedRead`, and `nvs-ir` lowers it to a tag test that throws if the value is `null`, followed by the
  `Untag`. A narrowed variable keeps today's unchecked `Untag`. The IR test proves the check is emitted.
- **Cases**: `$this->p` in an `if`, in a ternary and after a guard clause; `$v->p` on a parameter; a
  two-link path; `is` on a `readonly` property; a write to the root variable that widens the path again.

## Stage 4 — the help, the rule and the proofs

**Does:** Makes E0459's help right for a property, writes the rule and its record, and brings the
feature's proofs and pages up to date.

File set: `crates/nvs-types/src/expr/members.rs`, `crates/nvs-types/src/expr/calls.rs`,
`crates/nvs-diagnostics/src/lib.rs`, `docs/rules/types/narrowing.md`, one new record under
`docs/decisions/`, `docs/examples/lang/classes/nullable-objects-and/`,
`tests/hostile/lang/classes/nullable-objects-and/`, `website/src/content/docs/syntax/values/nullable.mdx`,
`website/src/content/docs/syntax/classes/properties.mdx`, `tests/conformance/reject/`.

- **The help.** `strip_nullsafe_receiver` takes the receiver expression. For a property that is not a
  narrowing subject, or an element, the help says that a test does not narrow it, and shows a copy into a
  local variable and `?->`. For a variable the help stays as it is. The card says the same.
- **The rule.** `docs/rules/types/narrowing.md:29-31` is rewritten: the subject is a variable, or a path
  of `readonly` properties from one, and the narrowed read of a path is checked at run time. It says why a
  mutable property is never narrowed: a call the code does not show can write it between the test and
  the read (a `PropertyObserver` already runs one on the test's own read), and so can a write through
  another variable, a `yield` or an `await`. A copy into a local variable or `?->` is the way.
- **One new record** for the decision, and no other number.
- **The proofs.** `about.md` and one example show a `readonly` property narrowed. One attack tries to
  change a `readonly` property after it was tested, and every attempt must fail. The bench and the Rust
  tests carry over. The two website pages say the same, held to AGENTS.md § *Text an end user reads*.
- **The case** for a mutable property: E0459 with the new help, pinned.

## Standing decisions

These are the user's calls, made on 2026-10-05, unless marked as mine. No session re-decides one.

- **A null test narrows a `readonly` property**, once `readonly` is written once.
- **The narrowed read of a property keeps a run-time null check**, so a write path that was missed is a
  run-time error and never memory unsafety. A narrowed variable stays unchecked.
- **A mutable property is never narrowed**, and the rule says why, so the next report of it is answered
  by `bun nv brief --where narrow`.
- **One side goal for all of it**, so the chain run is not disturbed.
- **Mine, 2026-10-05:** "written once" is checked when the program compiles, not at run time; a second
  write and a write through another receiver keep E0782; a hook on `readonly` gets one new code; a
  subject may be a path of several `readonly` links; all four spellings narrow it, on the same edges as a
  variable; the checked read throws an `Error` that names the property, through the existing run-time
  error path closest to it, and the record says which.
- **The tradeoffs, stated once.** Speed: one compare and branch per narrowed property read, which
  replaces the copy into a local that programs write today. Memory: none. Usability: those copies go
  away for `readonly` properties, and E0459 shows the fix for the others. Simplicity: the rule gains one
  kind of subject, and `readonly` gains the write-once guarantee it already promised. A constructor that
  writes a `readonly` property twice, or through another instance, stops compiling.
