---
looper: 1
title: A union of classes reads a member every class declares
summary: "A property read and a method call through a union of classes compile when every class declares that member with the same type and it is visible, each lowers to a class test and a fixed-offset access per class, the rules agree, the feature has its proofs, and the checks below pass."
context: [crates/nvs-types/src/expr/members.rs, crates/nvs-types/src/expr_table.rs, docs/rules/types/unions-and-mixed.md, docs/rules/types/a-declared-type-answers-before-the-program-runs.md]
stages:
  - id: 1
    title: "the scope: what a union already allows, and the rule"
    status: done
    context: [crates/nvs-types/src/expr/calls.rs, crates/nvs-diagnostics/src/code/types.rs, docs/rules/types/erased-member-access.md, tests/conformance/lang/a-property-through-a-receiver-that-holds-no-object-is-a-compile-error.nvst]
    extra:
      context:
        rules: [types/unions-and-mixed, types/a-declared-type-answers-before-the-program-runs, types/erased-member-access]
  - id: 2
    title: "the checker accepts a shared member, and records each class"
    status: done
    context: [crates/nvs-types/src/expr/calls.rs, crates/nvs-diagnostics/src/code/types.rs]
    extra:
      context:
        rules: [types/unions-and-mixed, types/a-declared-type-answers-before-the-program-runs]
  - id: 3
    title: "a shared member lowers to a class test and a fixed-offset access per class"
    status: open
    context: [crates/nvs-ir/src/ir.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/lower/call.rs, tools/nv/cmd/soundness.ts]
    extra:
      context:
        rules: [types/unions-and-mixed, types/erased-member-access]
  - id: 4
    title: "the feature has its proofs and its place on the website"
    status: open
    context: [docs/rules/testing/feature-proofs.md, docs/examples/lang/types/chapter.md, docs/examples/lang/types/nullable-union-single-value-and-enum-case-types/about.md, website/src/content/docs/syntax/values/union-types.mdx]
    extra:
      context:
        rules: [testing/feature-proofs, types/unions-and-mixed]
checks:
  - id: union-member-proofs
    stage: 4
    name: the shared-member feature owes no proof
    run: [bun, nv, proofs, --gate, --only, "lang:types/a-member-every-class-of-a-union-declares"]
    reach: true
    floor: false
  - id: rules-check
    stage: 4
    name: the rulebook validates
    run: [bun, nv, rules, --check]
    reach: true
    floor: false
---

# A union of classes reads a member every class declares

## Why

A program that holds a `Box|Circle` and wants the `name` both classes declare cannot read it.
`$either->name` is `E0495` (`crates/nvs-diagnostics/src/code/types.rs:936`, emitted at
`crates/nvs-types/src/expr/members.rs:2304`), and the conformance case
`tests/conformance/lang/a-property-through-a-receiver-that-holds-no-object-is-a-compile-error.nvst:52`
pins that. What a program writes instead is an `if` with `is` for each class, and the same read in
each branch.

The rules disagree with each other. `rule:types/unions-and-mixed` (`docs/rules/types.md:1394`) says a
union permits the operations valid for every member. `rule:types/a-declared-type-answers-before-the-program-runs`
(`docs/rules/types.md:2827`) refuses this case, and `rule:types/erased-member-access`
(`docs/rules/types.md:2476`) repeats the refusal. This goal makes the first rule true and rewrites the
other two to agree.

## Standing decisions

**What is allowed.** A property read and a method call through a receiver whose type is a union of
classes compile when every class in the union declares or inherits that member, with the same type
for a property and the same signature for a method, and the member is visible at the call site. The
same signature means the same parameters, types, `inout`, defaults, variadic and return type, and the
same `static`. The same type means identical, nullability and the `tainted` and `secret` qualifiers
included. A nullable union (`?Box|Circle`) follows the nullable rules a single class follows: `?->`
reads it. Why: the result has one type, so nothing after the read changes.

**An interface in the union keeps today's error.** Every member of the union must be a class. Why: an
interface gives no fixed offset, and a fixed-offset read is the point of the lowering below. A later
goal may add interface arms through the dispatch a call on an interface already uses.

**Writes.** A write through the union is allowed only when the property is writable at the call site
in every class, with the same type, no `readonly` and the same `set` visibility. Otherwise it keeps an
error. Why: the write then checks the value against one declared type, as a write through each class
would. `rule:types/a-member-every-class-of-a-union-declares` records it.

**A refusal gets a code that says what is wrong.** A union where one class lacks the member, where
the types or signatures differ, or where the member is not visible in one class, gets one new code,
`E0881`, the next free one in `crates/nvs-diagnostics/src/code/types_e08.rs`. Its message names the class that
lacks the member, or the two types that differ. `E0495` stays for a receiver that can hold no object
at all. Every message and help line follows AGENTS.md § *Text an end user reads*.

**The lowering is a class test per class and the access that class alone would make.** The checker
records a new `ExprInfo` variant that lists each class of the union with its member
(`crates/nvs-types/src/expr_table.rs:654` is `ExprInfo::Property`, the one-class form). Lowering
branches on `ClassTest` (`crates/nvs-ir/src/ir.rs:1353`) for each class but the last. Each branch
lowers exactly as the same access through that one class does: a plain property is a fixed-offset
`FieldGet` (`crates/nvs-ir/src/ir.rs:1031`), a hooked one calls its hook, and a method call is the
call that class makes. The last class is the fall-through, because the checker proved the receiver is
one of them. There is no name lookup and no throw edge on the request path.

**The name-keyed `SlotGet` is declined** (`crates/nvs-ir/src/ir.rs:1168`). Why: it looks the name up on
every read, and it adds an error edge for a case the checker already ruled out. The Declined entry is
in `rule:types/a-member-every-class-of-a-union-declares`.

**The fall-through is covered by the soundness matrix.** If `FieldGet` does not check its receiver's
class at run time, stage 3 adds a union receiver row to `bun nv soundness` (`tools/nv/cmd/soundness.ts`),
so a hole in the type system that lets a third class reach the read fails a test.

**The feature id is `lang:types/a-member-every-class-of-a-union-declares`.** Its chapter heading is
"A member every class of a union declares" in `docs/examples/lang/types/chapter.md`, after "Nullable,
union, single-value and enum-case types". Its examples go under
`docs/examples/lang/types/a-member-every-class-of-a-union-declares/`. Its help in the binary is its
reference section, which `bun nv render` writes into `docs/reference/lang/20-types.md` and
`nvs agent show` embeds (`crates/nvs-cli/src/agent.rs:352`). The website's union page
(`website/src/content/docs/syntax/values/union-types.mdx`) gains one section.

**The case that pins today's refusal is rewritten, not frozen.** The `Box|Circle` read in
`tests/conformance/lang/a-property-through-a-receiver-that-holds-no-object-is-a-compile-error.nvst:52`
becomes a union whose classes do not share the member, so it still pins a refusal. The commit says the
rule changed, which is why the expected output changed.

**Gate.** The goal starts when `one-error-per-mistake` is reached: `docs/goals/one-error-per-mistake.md`
no longer exists. Both change `nvs-types`, and this one also changes `nvs-ir`, which `compiler-holes`
changes. It sits right behind `one-error-per-mistake` on the chain, so the gate holds when it starts.

**Lanes.** One lane. Stages 1 to 3 share the checker and the lowering and run in order. Stage 4's
examples print what stage 3 lowers.

**Trade-offs.** Security: each class's branch reads exactly what a read through that class alone
reads, and the soundness matrix covers the fall-through. Latency: a read through a union of `n`
classes costs `n - 1` class tests and one fixed-offset load, with no name lookup. A read through one
class does not change. Memory: one list of classes per access site in the checker's table, per
compiled program and never per request. Simplicity: the language loses one refusal, and a union now
allows what every member allows, as its rule already says. Usability: the `is` ladder that programs
write today becomes one line.

## Stages

### 1. the scope: what a union already allows, and the rule

**Does:** the session finds out what compiles today, settles writes, and rewrites the three rules
before any code.

**Split:** one `Explore` subagent answers, as `file.rs:NN` and one line each: whether a method call
through `A|B` compiles today and where that is decided, whether `A|B` with a common parent that
declares the member is already read through the parent, and every conformance case that pins a member
access through a union.

- [x] The subagent's answers, and the stage items below narrowed to what does not work yet. `crates/nvs-types/src/expr/members.rs:2304`, `crates/nvs-types/src/expr/calls.rs:1819`
- [x] Writes decided, and why (`rule:types/a-member-every-class-of-a-union-declares`). `docs/rules/types/a-member-every-class-of-a-union-declares.md:1`
- [x] The shared member, the same-type test, visibility, writes, the interface arm, and the Declined `SlotGet` lowering are stated, in a `designed` rule of their own that `rule:types/unions-and-mixed` points at. `docs/rules/types/a-member-every-class-of-a-union-declares.md:1`, `docs/rules/types/unions-and-mixed.md:9`
- [x] `rule:types/a-declared-type-answers-before-the-program-runs` no longer refuses a union of classes. `docs/rules/types/a-declared-type-answers-before-the-program-runs.md:1`
- [x] `rule:types/erased-member-access` no longer repeats the refusal. `docs/rules/types/erased-member-access.md:1`

### 2. the checker accepts a shared member, and records each class

**Does:** `$p->x` and `$p->m()` through `A|B` type-check as the rule says, and the checker records each
class for the lowering.

- [x] A property read through a union of classes that all declare it with the same type and make it visible (`rule:types/a-member-every-class-of-a-union-declares`). Today `class_qname_of` has no `Ty::Union` arm, so the read takes the `None` arm's `E0495`. `crates/nvs-types/src/expr/members.rs:733`, `crates/nvs-types/src/expr/members.rs:2308`
- [x] A method call through such a union, with the same signature in every class. Today it is `E0477`: resolution runs only when `class_qname_of` is `Some`. `crates/nvs-types/src/expr/calls.rs:83`, `crates/nvs-types/src/expr/calls.rs:113`
- [x] A write, as stage 1 decided: the same type, no `readonly` and the same `set` visibility in every class. Reads and writes share `check_property_member`. `crates/nvs-types/src/expr/assign.rs:991`, `crates/nvs-types/src/expr/members.rs:2029`
- [x] The new `ExprInfo` variant lists each class with its member. `crates/nvs-types/src/expr_table.rs:654`
- [x] The new code `E0881` for a member not shared, with a message that names the class or the two types. `E04xx` is full, so it is the next free code of `types_e08.rs`. `crates/nvs-diagnostics/src/code/types_e08.rs:1045`
- [x] Unit tests in the checker for each refusal: a class lacks it, the types differ, it is private in one class, an interface is in the union. `crates/nvs-types/tests/narrowing.rs:1`

### 3. a shared member lowers to a class test and a fixed-offset access per class

**Does:** the lowering reads the new `ExprInfo` and emits one branch per class.

**Split:** after the lowering lands, one writing subagent writes the `.nvst` cases below.

- [x] A property read lowers to `ClassTest` per class but the last and one `FieldGet` per branch. `crates/nvs-ir/src/lower/expr.rs:5262`, `crates/nvs-ir/src/ir.rs:1353`, `crates/nvs-ir/src/ir.rs:1031`
- [x] A hooked property's branch calls its hook, as a read through that class does. `crates/nvs-ir/src/lower/expr.rs:5262`
- [x] A method call lowers to one branch per class, each the call that class makes. `crates/nvs-ir/src/lower/call.rs:1299`
- [x] A write, as stage 1 decided. `crates/nvs-ir/src/lower/stmt.rs:1`
- [x] A union receiver row in the soundness matrix, if `FieldGet` does not check its receiver's class. `tools/nv/cmd/soundness.ts:1`
- [x] The `Box|Circle` case rewritten: it pins `E0495` with `Box|int`, and the new `E0881` case pins the union whose classes do not share the member. `tests/conformance/lang/a-property-through-a-receiver-that-holds-no-object-is-a-compile-error.nvst:52`
- [x] The `Dog|Cat` call case rewritten the same way: it pins `E0477` with `Dog|int`. `tests/conformance/reject/a-method-call-through-a-receiver-that-names-no-class-is-refused.nvst:34`
- [x] Cases: a shared property read through each class, a shared method called through each class, each refusal with its code. `tests/conformance/lang/a-property-every-class-of-a-union-declares-is-read.nvst:1`, `tests/conformance/lang/a-method-every-class-of-a-union-declares-is-called.nvst:1`, `tests/conformance/lang/a-member-not-shared-by-every-class-of-a-union-is-a-compile-error.nvst:1`

### 4. the feature has its proofs and its place on the website

**Does:** `bun nv proofs --id 'lang:types/a-member-every-class-of-a-union-declares'` prints nothing
owed, and the website's union page shows the read.

**Split:** writing subagents, one after another: the chapter heading, `about.md` and the examples
first, then the attack, then the bench, then the website section. The session checks each against
AGENTS.md § *Text an end user reads*.

- [ ] The chapter heading after the union feature. `docs/examples/lang/types/chapter.md:1`
- [ ] `about.md`: what a union of classes may read and call, and when it does not compile. `docs/examples/lang/types/a-member-every-class-of-a-union-declares/about.md:1`
- [ ] Three examples: a property both classes declare, a method both classes declare, and a member one class lacks. `docs/examples/lang/types/a-member-every-class-of-a-union-declares/01-a-property-both-classes-declare.nvs:1`
- [ ] The attack: a third class reaches the union through `mixed` and `as`, a subclass redeclares the property, and a private member in one class is read. Each fails to compile or throws where it enters (`rule:testing/hostile-case-contract`). `tests/hostile/lang/types/a-member-every-class-of-a-union-declares/01-a-third-class-tries-to-reach-the-read.nvs:1`
- [ ] The bench: a read through a union of three classes, with `// bench: complexity constant`. `benches/members/lang/types/a-member-every-class-of-a-union-declares.nvs:1`
- [ ] `bun nv render` writes the reference section, and `nvs agent show` prints it. `docs/reference/lang/20-types.md:1112`, `crates/nvs-cli/src/agent.rs:352`
- [ ] The website's union page has a section on the shared member, with one example. `website/src/content/docs/syntax/values/union-types.mdx:1`
- [ ] `rule:types/a-member-every-class-of-a-union-declares` names the new cases in `guardedBy`, and its `status` is `shipped` with no `goal`. `data/rules/types/a-member-every-class-of-a-union-declares.json:1`

## Handoff

### State

Stages 1 and 2 are done and committed. **Stage 3's work is written and UNCOMMITTED, and its verify
has not run.** The session hit the hand-off budget, and `looper verify` was not a subcommand of the
installed `looper` binary (`bun tools/looper/main.ts verify` or `bun nv affected --run` is the gate).
Every new `.nvst` case and the soundness probe pass on the debug build, and `cargo build` is clean.

What landed in the working tree:

- **The lowering.** `Lowering::lower_union_member` (`crates/nvs-ir/src/lower/expr.rs:5269`) lowers
  the receiver once and stages it under its span. For each class it emits a `ClassTest` and a branch,
  except the last class, which is the fall-through. Each arm then runs the *one-class* lowering, and
  `Lowering::union_arm` / `member_record` (`crates/nvs-ir/src/lower/mod.rs:3672`) answer that class's
  record for the access span. Read, call, `$u->m(...)`, write (`stmt.rs` `lower_store`) and the
  element write-back (`mod.rs` `write_back_array`) all go through it. `aliasing_read` reports a union
  read with a `get`-hook arm as fresh, and a plain arm then retains its borrowed value.
- **`inout $u->p`** is `RefHolder::UnionField` and `Lowering::branch_on_union_class`
  (`mod.rs:2044`, `mod.rs:3513`). `write_back_holder` and `flush_ref_writebacks` now take
  `cur: &mut BlockId`.
- **The checker** refuses `inout` and an element write through a union where one class hooks the
  property, with the one-class codes `E0439`/`E0478`: `ExprTypeTable::hooked_property`
  (`crates/nvs-types/src/expr_table.rs:1668`), used in `args::check_inout_arg` and
  `assign`'s element-write root check.
- **Two leaks that were already there are fixed**, and the cause was in the one-class code:
  `lower_read_modify_write` never released a fresh `get`-hook read (`$t->hooked .= "x"`), and a
  plain write through a fresh receiver (`make()->p = v`) never released the receiver (`stmt.rs`).
  `tests/conformance/lang/a-write-through-a-fresh-receiver-or-a-get-hook-holds-no-memory-after-it.nvst`
  pins both, and it prints `grew` without the fix.
- **Soundness:** `PROPERTY_PATHS` gained `union-tested` and `union-fallthrough`, with an `Other<n>`
  class. `--write` was run, and `--probe` with `NVS_BIN=E:/mwl/target/debug/nvs.exe` gives 0 of 407
  failing. The default `covws` binary is stale and reports the old panic.
- **Cases:** `a-property-every-class-of-a-union-declares-is-read.nvst`,
  `a-method-every-class-of-a-union-declares-is-called.nvst`, the extended
  `a-member-not-shared-by-every-class-of-a-union-is-a-compile-error.nvst` (6 errors), and
  `a-union-where-one-class-hooks-the-property-is-no-inout-argument-and-no-element-write-root.nvst`.
  There is also a lowering unit test in `crates/nvs-ir/src/lower/tests.rs` that counts the class tests.

### Next

**First, finish stage 3:** run the gate, fix what it names, then commit and set stage 3 `done`.

- [ ] `bun nv affected --run` over the working tree, and fix every failure in one pass. `crates/nvs-ir/src/lower/expr.rs:5269`
- [ ] Commit in three slices: `fix(ir)` for the two leak fixes and their case (`stmt.rs` `lower_read_modify_write`/`lower_store`'s plain arm), `feat(ir)` for the union lowering, the checker refusals, the cases and `tests.rs`, and `test(soundness)` for `tools/nv/cmd/soundness.ts` and `tests/conformance/soundness/`. The rule fragment's new sentence goes with `feat(ir)`. `tools/nv/cmd/soundness.ts:1`
- [ ] Then stage 4 as written. The examples print what stage 3 lowers. `docs/examples/lang/types/chapter.md:1`

### Backlog

- `isinit($u->p)` through a union is `E0854` ("needs an object of a known class"). The rule says
  nothing about it, so it is a choice for stage 4's `about.md` or a later goal.
  `crates/nvs-types/src/expr/presence.rs`
- The write refusals for a different `(set)` visibility and for a `get` hook with no `set` hook have
  no case yet. `crates/nvs-types/src/expr/assign.rs:1297`
