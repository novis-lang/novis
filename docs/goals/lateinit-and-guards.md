---
looper: 1
title: "A base class initializes `lateinit` properties and guards writes"
summary: "`LateInit`, `isinit` and `PropertyGuard` work on every `lateinit` type, and `PropertyObserver` is told of every element write."
context: [crates/nvs-runtime/src/value.rs, crates/nvs-hir/src/interfaces.rs]
stages:
  - id: 1
    title: The observer is told of element writes
    status: done
    context: [crates/nvs-ir/src/lower/stmt.rs, crates/nvs-ir/src/lower/mod.rs]
    extra:
      context:
        rules: [classes/property-observer, classes/property-observer-pipeline]
        shapes: ["A `.nvst` test case"]
  - id: 2
    title: "`lateinit` takes every type, and `isinit`"
    status: done
    context: [crates/nvs-types/src/signatures.rs, crates/nvs-types/src/lateinit.rs, crates/nvs-syntax/src/parser/expr.rs, crates/nvs-ir/src/lower/expr.rs]
    extra:
      context:
        rules: [classes/lateinit, classes/lateinit-restrictions, classes/an-unwritten-property-read-throws, classes/lateinit-read-before-write]
        shapes: ["A `.nvst` test case", A diagnostic]
  - id: 3
    title: "`LateInit` initializes on the first read"
    status: open
    context: [crates/nvs-ir/src/lower/expr.rs, crates/nvs-runtime/src/object.rs]
    extra:
      context:
        rules: [classes/an-unwritten-property-read-throws, classes/property-observer-pipeline]
        shapes: ["A `.nvst` test case"]
  - id: 4
    title: "`LateInit` across tasks"
    status: open
    context: [crates/nvs-runtime/src/object.rs]
    extra:
      context:
        rules: [concurrency/one-scheduler, concurrency/the-parking-contract]
        shapes: ["A `.nvst` test case"]
  - id: 5
    title: "`PropertyGuard` runs before every write"
    status: open
    context: [crates/nvs-ir/src/lower/stmt.rs, crates/nvs-ir/src/lower/mod.rs]
    extra:
      context:
        rules: [classes/property-observer-pipeline, classes/unset-is-refused-on-a-property, types/arrays]
        shapes: ["A `.nvst` test case"]
  - id: 6
    title: "the rules, the reference and the feature proofs"
    status: open
    context: [docs/reference/lang/50-classes.md, website/src/content/docs/syntax/classes/properties.mdx, website/src/content/docs/syntax/classes/property-hooks.mdx]
    extra:
      context:
        rules: [testing/feature-proofs, core-api/reference-card]
        shapes: [A rule fragment, "Feature proofs — an example, an attack, a bench, a `covers:` marker"]
checks:
  - id: observer-element-write
    stage: 1
    name: "an element write reaches `onPropertySet` once, with the whole array, and never `onPropertyGet`"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/class/a-property-observer-is-told-of-an-element-write.nvst]
    tests: [tests/conformance/class/a-property-observer-is-told-of-an-element-write.nvst]
  - id: lateinit-every-type
    stage: 2
    name: "`lateinit` takes a scalar, an array and a nullable type, and `null` is a value it holds"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/class/lateinit-takes-every-type.nvst]
    tests: [tests/conformance/class/lateinit-takes-every-type.nvst]
  - id: isinit-construct
    stage: 2
    name: "`isinit` tests a `lateinit` property without reading it, and is a compile error on any other property"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/class/isinit-tests-a-lateinit-without-reading-it.nvst, --case, tests/conformance/class/isinit-is-refused-on-a-property-that-always-has-a-value.nvst]
    tests: [tests/conformance/class/isinit-tests-a-lateinit-without-reading-it.nvst, tests/conformance/class/isinit-is-refused-on-a-property-that-always-has-a-value.nvst]
  - id: late-init-codegen
    stage: 2
    name: "nvs-codegen (the compiled `lateinit` read checks the tag on every type, and `isinit` calls nothing)"
    run: [cargo, test, --test, late_init]
    tests: [a_lateinit_int_reads_its_value_after_the_first_write, a_lateinit_nullable_holds_null_as_a_value, a_lateinit_read_before_the_first_write_throws_on_every_type, isinit_reads_no_property_and_calls_nothing]
  - id: late-init-first-read
    stage: 3
    name: "`initProperty` runs on the first read, its initializing writes are plain, and a read with nothing initialized throws"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/class/late-init-initializes-on-the-first-read.nvst, --case, tests/conformance/class/late-init-batch-writes-are-plain.nvst, --case, tests/conformance/class/late-init-that-initializes-nothing-throws.nvst]
    tests: [tests/conformance/class/late-init-initializes-on-the-first-read.nvst, tests/conformance/class/late-init-batch-writes-are-plain.nvst, tests/conformance/class/late-init-that-initializes-nothing-throws.nvst]
  - id: late-init-runtime
    stage: 3
    name: "nvs-codegen (`initProperty` runs once per property, a read inside it throws, an element write initializes first)"
    run: [cargo, test, --test, late_init]
    tests: [init_property_runs_once_per_property, a_read_inside_its_own_init_property_throws, an_element_write_on_an_uninitialized_property_initializes_it_first, an_ordinary_write_inside_init_property_runs_guard_and_observer]
  - id: late-init-tasks
    stage: 4
    name: "a second task waits for a running `initProperty`, and a failed one is retried by the next read"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/late-init-a-second-task-waits-for-the-first.nvst, --case, tests/conformance/core/late-init-a-failed-init-is-retried-by-the-next-read.nvst]
    tests: [tests/conformance/core/late-init-a-second-task-waits-for-the-first.nvst, tests/conformance/core/late-init-a-failed-init-is-retried-by-the-next-read.nvst]
  - id: property-guard
    stage: 5
    name: "`PropertyGuard` stops a write and keeps the old value, gets the key path of an element write, and sees an unset first"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/class/property-guard-stops-a-write-and-keeps-the-old-value.nvst, --case, tests/conformance/class/property-guard-gets-the-key-path-of-an-element-write.nvst, --case, tests/conformance/class/property-guard-sees-an-unset-before-it-happens.nvst]
    tests: [tests/conformance/class/property-guard-stops-a-write-and-keeps-the-old-value.nvst, tests/conformance/class/property-guard-gets-the-key-path-of-an-element-write.nvst, tests/conformance/class/property-guard-sees-an-unset-before-it-happens.nvst]
  - id: the-rules
    stage: 6
    name: "rulebook: the `LateInit` and `isinit` fragments exist"
    run: [bun, nv, brief, --where, lateinit]
    output_contains: ["rule:classes/late-init-initializes-on-the-first-read", "rule:classes/isinit-reads-nothing"]
  - id: the-guard-rule
    stage: 6
    name: "rulebook: the guard fragment exists"
    run: [bun, nv, brief, --where, guard]
    output_contains: ["rule:classes/property-guard-runs-before-the-write"]
  - id: proofs-late-init
    stage: 6
    name: "`LateInit` owes no feature proof"
    run: [bun, nv, proofs, --id, "lang:classes/lateinit"]
    stdout_contains: [complete.]
  - id: proofs-property-guard
    stage: 6
    name: "`PropertyGuard` owes no feature proof"
    run: [bun, nv, proofs, --id, "lang:classes/propertyguard"]
    stdout_contains: [complete.]
  - id: proofs-property-observer
    stage: 6
    name: "`PropertyObserver` still owes no feature proof"
    run: [bun, nv, proofs, --id, "lang:classes/propertyobserver"]
    stdout_contains: [complete.]
---

# A base class initializes `lateinit` properties and guards writes

## Why

A `get` / `set` hook replaces a property's storage, so an element write through a hooked property is
`E0478` (`crates/nvs-types/src/expr/assign.rs:958-964`), and that stays. Two common needs do not
replace storage at all: giving a property its value on its first read, and listening to writes
without changing them. This goal gives both to a base class, declared once and inherited, while the
property stays plain storage, so `$post->tags[] = "x"` works.

```nvs
abstract class Entity implements LateInit, PropertyGuard, PropertyObserver {
    public function initProperty(string $name): void {
        $this->session->initBatch($this, $name);       // may initialize the property on many objects
    }
    public function beforePropertySet(string $name, mixed $value, array<string> $keys): void {
        $this->session->validate($this, $name, $value, $keys);   // a throw stops the write
    }
    public function beforePropertyUnset(string $name, array<string> $keys): void {}
    public function onPropertySet(string $name, mixed $value): void {
        $this->session->markDirty($this, $name);
    }
    public function onPropertyGet(string $name, mixed $value): void {}
}

class Post extends Entity {
    public int $authorId;
    public lateinit ?User $author;
    public lateinit array<string> $tags;
}

echo $posts[0]->author?->name;   // initProperty("author"): one query for the whole batch
$posts[0]->tags[] = "news";      // initializes tags, then guard, in-place append, observer
if (!isinit($posts[5]->tags)) { /* not initialized, and this test did not initialize it */ }
```

The user settled every decision below in one interactive session. It also fixes a bug found while
designing it: today `$o->rows[] = "a"` on an observed property reaches `onPropertyGet` and never
`onPropertySet`, which is exactly the flaw Java gave for deprecating `java.util.Observable`.

It is a side goal: it shares no file with goal `pdf` or goal `spreadsheet`.

## Standing decisions

**Names.** Everything says "init", after the keyword: `lateinit`, `isinit`, the interface `LateInit`
and its method `initProperty(string $name): void`. The guard is `PropertyGuard` with
`beforePropertySet(string $name, mixed $value, array<string> $keys): void` and
`beforePropertyUnset(string $name, array<string> $keys): void`. `PropertyObserver` keeps its names.
Three interfaces, never one, so a class pays only for what it implements. Home: the new fragments.

**Hooks do not change.** `rule:classes/property-hooks` and `E0478` stay as they are.

**`lateinit` takes every type** — scalars, arrays, `?T` — and keeps refusing `readonly` (`E0427`)
and a promoted parameter (`E0426`). `E0424` and `E0425` are retired as
[conventions.md](../agent/conventions.md) § *A diagnostic* says. The compiled read checks the tag
byte of `Tag::Unset` (`crates/nvs-runtime/src/value.rs:118`) instead of the null payload
(`crates/nvs-ir/src/lower/expr.rs:4963-4997`); `null` and "not initialized" are two tags. Memory:
none, the tag already exists. Fallback if the tag load costs a measurable read on a non-`lateinit`
property: it may not; only `lateinit` reads carry it. Home: `rule:classes/lateinit-restrictions`.

**`isinit($obj->prop)` is a construct**, parsed like `isset`
(`crates/nvs-syntax/src/parser/expr.rs:2266`), returning `bool`. It never reads the property, never
calls `initProperty` and compiles to the slot load and tag compare. It is a compile error on a
property that is not `lateinit` ("`isinit` needs a `lateinit` property. `$title` always has a value.")
and on a receiver whose class is not known statically ("`isinit` needs an object of a known class. Test
the class first with `is`."). Home: `rule:classes/isinit-reads-nothing`.

**`LateInit`.** A read or an element write that finds a `lateinit` property uninitialized, in a class
implementing `LateInit`, calls `initProperty($name)` and then reads again. Still uninitialized: it
throws "`initProperty` returned, but `$author` is still not initialized." Without `LateInit` the read
throws "`Post`'s property `$author` is read before it is initialized." — the existing message in
`crates/nvs-ir/src/lower/expr.rs:4995` and `crates/nvs-runtime/src/object.rs:4977`, reworded to
"initialized". `rule:classes/lateinit-read-before-write`'s static check (`E0428`) does not fire in a
class implementing `LateInit`, because there a read before a write is the trigger. Home: the new
fragment `classes/late-init-initializes-on-the-first-read`.

**Plain writes.** While `initProperty` runs in a task, a write in that task that *initializes* an
uninitialized `lateinit` property, on any object and through any write form including
`Core\Reflect::set`, is plain: no guard, no `set` hook, no observer. Every other write is ordinary.
A hook's write to its own property stays plain as it is today
(`docs/rules/classes/property-observer-pipeline.md:12-14`). The broad form, every write inside
`initProperty` plain, is declined: it lets any validation be skipped by writing inside an
`initProperty`, and security ranks first.

**Across tasks.** Child tasks share the parent's objects. A read of the property in the same task
during its own `initProperty` throws "`$author` is read while `initProperty` initializes it." A read
from another task parks until that `initProperty` returns, and then reads; it never calls
`initProperty` a second time. A throw or a cancellation leaves the property uninitialized, and the
next read, a parked one included, calls `initProperty` again. The in-progress state is a
request-local table of initializations in flight, keyed by object and property: memory O(in-flight
initializations), never a field per slot. Coordinating a batch across *different* objects is the
program's own job, with a `Core\Task\Channel`. Home: the `LateInit` fragment.

**`PropertyGuard`** runs before the write, in the constructor too, and on a hooked property before
the `set` hook, with the caller's value. A throw stops the write: nothing is stored and no observer
runs. It returns `void`, so it never changes a value. It gets the change, never the resulting array:
`$keys` is `[]` for a whole write, the append's next index for `$p->tags[] = $v`, the path
`["0", "1"]` for `$p->cells["0"]["1"] = $v`, and `$value` is the new element. `unset` of an element
calls `beforePropertyUnset` with at least one key. A `static` property reaches no guard, as it reaches
no observer. Cost: one small key array per element write in a guarded class, and nothing anywhere
else. Home: the new fragment `classes/property-guard-runs-before-the-write`.

**`PropertyObserver` and element writes.** An element write — `=`, a compound assignment, an increment
— reports one `onPropertySet` with the whole array after it is stored, and no `onPropertyGet`. An
element `unset` reports the same. Passing the array copies nothing. Home:
`rule:classes/property-observer-pipeline`.

**The order**, the home being the pipeline fragment:

```
Read:   not initialized? → initProperty → get hook or storage → onPropertyGet
Write:  beforePropertySet → set hook or storage                → onPropertySet
```

**The "nothing is lazy" sentence** in `docs/rules/programs/framework-refusals.md:1-4` is rewritten:
a `get` hook already made it untrue, and `LateInit` is the declared way to initialize later. The data
mapper stays the framework's default.

**Every fragment a stage's work makes untrue is rewritten whole by the session that lands that work**,
not before: `lateinit-restrictions` and `an-unwritten-property-read-throws` by stage 2, the pipeline
by stages 1 and 5, `lateinit-read-before-write` by stage 3.

## Stages

### 1. The observer is told of element writes

**Does:** an element write on an observed property reports `onPropertySet` with the whole array.

- [x] The element write's read of the root property goes without `onPropertyGet`, and the write-back emits `onPropertySet`, for `=`, compound assignment, increment and string append. `crates/nvs-ir/src/lower/mod.rs:3056`
- [x] The same for `unset` of an element. `crates/nvs-ir/src/lower/stmt.rs:1700`
- [x] The conformance case, and the pipeline fragment rewritten. `tests/conformance/class/a-property-observer-is-told-of-an-element-write.nvst:1`

### 2. `lateinit` takes every type, and `isinit`

**Does:** `lateinit` is allowed on every type, its read checks the tag, and `isinit` exists.

- [x] Drop `E0424` and `E0425`. `crates/nvs-types/src/signatures.rs:1444`
- [x] The compiled read checks the tag byte on every `lateinit` type. `crates/nvs-ir/src/lower/expr.rs:4986`
- [x] `isinit` in the lexer, parser and AST, beside `isset`. `crates/nvs-syntax/src/parser/expr.rs:2266`
- [x] `isinit` type-checked, with its two compile errors. `crates/nvs-types/src/expr/mod.rs:956`
- [x] `isinit` lowered to the slot load and tag compare. `crates/nvs-ir/src/lower/expr.rs:611`
- [x] The Rust tests and the conformance cases. `crates/nvs-codegen/tests/late_init.rs:1`

### 3. `LateInit` initializes on the first read

**Does:** a read of an uninitialized `lateinit` property in a `LateInit` class calls `initProperty`.

- [ ] `LateInit` registered as a global interface beside `PropertyObserver`. `crates/nvs-hir/src/interfaces.rs:48`
- [ ] The compiled read's uninitialized branch calls `initProperty`, then reads again. `crates/nvs-ir/src/lower/expr.rs:4898`
- [ ] The erased read does the same. `crates/nvs-runtime/src/object.rs:4973`
- [ ] Initializing writes inside `initProperty` are plain. `crates/nvs-ir/src/lower/stmt.rs:1090`
- [ ] `E0428` skips a class implementing `LateInit`. `crates/nvs-types/src/lateinit.rs:80`

### 4. `LateInit` across tasks

**Does:** a second task parks on a running `initProperty`, and a failed one is retried.

- [ ] The request-local table of initializations in flight, with recursion and parking. `crates/nvs-runtime/src/object.rs:4973`

### 5. `PropertyGuard` runs before every write

**Does:** `beforePropertySet` and `beforePropertyUnset` run before every write and may stop it.

- [ ] `PropertyGuard` registered as a global interface. `crates/nvs-hir/src/interfaces.rs:48`
- [ ] The guard call before a whole write and before the `set` hook. `crates/nvs-ir/src/lower/stmt.rs:1247`
- [ ] The guard call with the key path before an element write. `crates/nvs-ir/src/lower/mod.rs:3056`
- [ ] The guard call before an element `unset`. `crates/nvs-ir/src/lower/stmt.rs:1700`

### 6. the rules, the reference and the feature proofs

**Does:** the fragments, the reference chapter, the website and every feature proof are written.

- [ ] The three new fragments and the rewritten `framework-refusals`. `docs/rules/programs/framework-refusals.md:1`
- [ ] A top-level `LateInit` section and a `PropertyGuard` section in the reference chapter, and `lateinit` widened there. `docs/reference/lang/50-classes.md:170`
- [ ] The website pages. `website/src/content/docs/syntax/classes/properties.mdx:1`
- [ ] The feature proofs of `lang:classes/lateinit` and `lang:classes/propertyguard`, and `propertyobserver`'s updated. `docs/examples/lang/classes/propertyobserver/about.md:1`

## Handoff

### State

Stages 1 and 2 are done. `lateinit` takes every type, and `E0424`/`E0425` are retired. The compiled
`lateinit` read tests the slot's tag through the new `InstKind::FieldIsInit`
(`crates/nvs-ir/src/ir.rs`), emitted by `emit_never_written_guard`
(`crates/nvs-ir/src/lower/expr.rs:4985`). `isinit` is a keyword, `ExprKind::IsInit`, checked by
`presence::check_isinit_operand` (`E0853`, `E0854`) and lowered by `lower_isinit`
(`crates/nvs-ir/src/lower/expr.rs:5011`) to that same instruction. `rule:classes/isinit-reads-nothing`
exists. The unwritten-read message still says "written"; the rewording to "initialized" is stage 3's,
in `crates/nvs-runtime/src/object.rs:4978`, `crates/nvs-ir/src/lower/call.rs:1478`, the guard, and
the cases and docs that pin the text.

### Next

**Stage 3: `LateInit` initializes on the first read**, one file set: `crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-runtime/src/object.rs`, `crates/nvs-hir/src/interfaces.rs`.

- [ ] `LateInit` registered beside `PropertyObserver` (§ *Standing decisions*, `LateInit`). `crates/nvs-hir/src/interfaces.rs:48`
- [ ] The compiled read's unset branch calls `initProperty`, tests again, and throws the two new messages; the guard is `emit_never_written_guard`. `crates/nvs-ir/src/lower/expr.rs:4985`
- [ ] The erased read does the same, and every "read before it is written" becomes "initialized". `crates/nvs-runtime/src/object.rs:4978`
- [ ] `E0428` skips a class implementing `LateInit`, and `lateinit-read-before-write` is rewritten whole. `crates/nvs-types/src/lateinit.rs:80`

### Backlog

- Whether `PropertyGuard` and `PropertyObserver` give their methods empty default bodies, since interfaces may have default methods (`docs/reference/lang/50-classes.md:381`); ask the user.
- `isinit` on a hooked `lateinit` property reports `E0853`; decide in stage 6 whether it should test the slot (`crates/nvs-types/src/expr/presence.rs`).
