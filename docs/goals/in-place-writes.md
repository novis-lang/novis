---
looper: 1
title: "nested writes, appends and reassigned arrays cost what they add, not what they hold"
summary: "Reached when a write into a row of an unshared array happens in place, an append to a property or element grows its string in place, an array passed at its last use is moved, and a retain makes no call, each held by a count guard that fails on faster than linear growth."
context: [crates/nvs-ir/src/lower/stmt.rs, crates/nvs-ir/src/lib.rs, crates/nvs-codegen/tests/common/mod.rs, docs/rules/types/arrays.md]
stages:
  - id: 1
    title: a nested write into an unshared array happens in place
    status: done
    context: [crates/nvs-runtime/src/array.rs, crates/nvs-runtime/src/helpers.rs, crates/nvs-ir/src/ir.rs, crates/nvs-ir/src/lower/mod.rs, crates/nvs-codegen/tests/arrays.rs]
  - id: 2
    title: an append to a property or an element grows its string in place
    status: done
    context: [crates/nvs-runtime/src/string.rs, crates/nvs-runtime/src/helpers.rs, crates/nvs-ir/src/ir.rs, docs/rules/classes/property-observer-pipeline.md]
  - id: 3
    title: an append of an interpolated string writes its parts into the target
    status: done
    context: [crates/nvs-runtime/src/string.rs, crates/nvs-ir/src/ir.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-codegen/src/emit.rs, docs/rules/types/string-holes.md]
  - id: 4
    title: an array or string passed at its last use is moved
    status: done
    context: [crates/nvs-ir/src/lower/call.rs, crates/nvs-ir/src/lower/mod.rs, crates/nvs-ir/src/ty.rs, docs/rules/statements/inout-is-the-by-reference-spelling.md]
  - id: 5
    title: a literal is never retained or released
    status: done
    context: [crates/nvs-codegen/src/emit.rs, crates/nvs-runtime/src/budget.rs, crates/nvs-codegen/tests/probes.rs]
  - id: 6
    title: a retain runs without a call
    status: open
    context: [crates/nvs-codegen/src/emit.rs, crates/nvs-codegen/src/lib.rs, crates/nvs-runtime/src/string.rs, crates/nvs-runtime/src/object.rs, crates/nvs-runtime/src/array.rs]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: nested-writes-in-place
    stage: 1
    name: "a nested append or set into an unshared array allocates in proportion to what it adds, at two sizes"
    run: [cargo, test, --test, in_place_writes, --, a_nested_append_allocates_in_proportion_to_what_it_adds, a_nested_set_allocates_in_proportion_to_what_it_writes, a_nested_write_through_a_property_allocates_in_proportion_to_what_it_adds]
    timeout: 1800
    tests:
      - a_nested_append_allocates_in_proportion_to_what_it_adds
      - a_nested_set_allocates_in_proportion_to_what_it_writes
      - a_nested_write_through_a_property_allocates_in_proportion_to_what_it_adds
  - id: nested-writes-keep-shared-values
    stage: 1
    name: "a nested write still copies a row or an array another holder shares, and a write that throws changes nothing"
    run: [cargo, test, --test, in_place_writes, --, a_nested_write_into_a_shared_row_still_copies_it, a_nested_write_into_a_shared_array_still_copies_it, a_nested_append_that_throws_leaves_the_array_as_it_was]
    timeout: 1800
    tests:
      - a_nested_write_into_a_shared_row_still_copies_it
      - a_nested_write_into_a_shared_array_still_copies_it
      - a_nested_append_that_throws_leaves_the_array_as_it_was
  - id: write-through-separates-by-count
    stage: 1
    name: "the write-through copies exactly the levels whose count is above one, and a nested write lowers to one call of it"
    run: [cargo, test, --lib, --, a_write_through_copies_only_a_level_that_is_shared, a_nested_write_lowers_to_one_write_through]
    timeout: 1800
    tests:
      - a_write_through_copies_only_a_level_that_is_shared
      - a_nested_write_lowers_to_one_write_through
  - id: appends-in-place
    stage: 2
    name: "an append to a property, a static property or an element allocates in proportion to what it adds, at two sizes"
    run: [cargo, test, --test, in_place_writes, --, a_property_append_allocates_in_proportion_to_what_it_adds, a_static_property_append_allocates_in_proportion_to_what_it_adds, an_element_append_allocates_in_proportion_to_what_it_adds]
    timeout: 1800
    tests:
      - a_property_append_allocates_in_proportion_to_what_it_adds
      - a_static_property_append_allocates_in_proportion_to_what_it_adds
      - an_element_append_allocates_in_proportion_to_what_it_adds
  - id: appends-keep-shared-strings
    stage: 2
    name: "an append still copies a string another holder shares, throws on an absent element, and keeps a guard, hook or observer call"
    run: [cargo, test, --test, in_place_writes, --, an_append_to_a_shared_string_still_copies_it, an_append_to_an_absent_element_throws_what_the_read_throws, an_append_to_a_guarded_or_hooked_property_keeps_its_calls]
    timeout: 1800
    tests:
      - an_append_to_a_shared_string_still_copies_it
      - an_append_to_an_absent_element_throws_what_the_read_throws
      - an_append_to_a_guarded_or_hooked_property_keeps_its_calls
  - id: parts-append-in-place
    stage: 3
    name: "an append of an interpolated string allocates only as its target grows, and one that reads its target still copies"
    run: [cargo, test, --test, in_place_writes, --, an_interpolated_append_allocates_only_as_the_target_grows, an_interpolated_append_that_reads_its_target_still_copies_it]
    timeout: 1800
    tests:
      - an_interpolated_append_allocates_only_as_the_target_grows
      - an_interpolated_append_that_reads_its_target_still_copies_it
  - id: parts-append-writes-the-join
    stage: 3
    name: "an append of parts writes the bytes the join of those parts writes, for every part kind"
    run: [cargo, test, --lib, --, an_append_of_parts_writes_what_the_join_writes, a_uint_hole_joins_as_a_part]
    timeout: 1800
    tests:
      - an_append_of_parts_writes_what_the_join_writes
      - a_uint_hole_joins_as_a_part
  - id: last-use-move
    stage: 4
    name: "an array or string passed and reassigned allocates in proportion to what it adds, a shared one is still copied, and a moved one is released once"
    run: [cargo, test, --test, in_place_writes, --, an_array_passed_and_reassigned_allocates_in_proportion_to_what_it_adds, a_string_passed_and_returned_allocates_in_proportion_to_what_it_adds, a_passed_array_another_variable_shares_is_still_copied, a_moved_argument_is_released_once_on_every_way_out]
    timeout: 1800
    tests:
      - an_array_passed_and_reassigned_allocates_in_proportion_to_what_it_adds
      - a_string_passed_and_returned_allocates_in_proportion_to_what_it_adds
      - a_passed_array_another_variable_shares_is_still_copied
      - a_moved_argument_is_released_once_on_every_way_out
  - id: move-conditions
    stage: 4
    name: "a local is moved only where every condition holds, and each broken condition keeps the retain"
    run: [cargo, test, --lib, --, a_local_is_moved_only_where_every_condition_holds]
    timeout: 1800
    tests:
      - a_local_is_moved_only_where_every_condition_holds
  - id: literals-uncounted
    stage: 5
    name: "compiled code names no refcount helper for a literal, and a row of literals enters none"
    run: [cargo, test, --test, refcount_calls, --, a_literal_is_never_retained_or_released, a_row_of_literals_enters_no_refcount_helper]
    timeout: 1800
    tests:
      - a_literal_is_never_retained_or_released
      - a_row_of_literals_enters_no_refcount_helper
  - id: refcount-inline
    stage: 6
    name: "a row that retains enters no retain helper, compiled code names none, and the inline count agrees with the helper's on every kind"
    run: [cargo, test, --test, refcount_calls, --, a_row_that_retains_enters_no_retain_helper, compiled_code_names_no_retain_helper, compiled_code_and_the_runtime_agree_on_the_count_word]
    timeout: 1800
    tests:
      - a_row_that_retains_enters_no_retain_helper
      - compiled_code_names_no_retain_helper
      - compiled_code_and_the_runtime_agree_on_the_count_word
---

# nested writes, appends and reassigned arrays cost what they add, not what they hold

## Why

Three ordinary statements copy everything they hold each time they run. Each count below is
`nvs run --count`'s `bytes`, at 2,000 and then 4,000 iterations. Twice the input allocates four times
the bytes.

- **`$groups[$key][] = $row`.** 96,388,931 bytes, then 384,532,931. The array built is 64 KB. The
  lowering descends with `Helper::ArrayRowForWrite` (`crates/nvs-ir/src/lower/stmt.rs:1987`). That
  helper returns the row retained (`crates/nvs-runtime/src/helpers.rs:402`) while the outer array
  keeps its own reference. The row reaches its write at a count of two, so copy-on-write copies it.
  The module doc records it as an open follow-up (`crates/nvs-ir/src/lib.rs:127`).
- **`$this->out .= $piece`.** 8,328,383 bytes, then 32,412,384, for a 16 KB string. Only a plain
  `string` local takes the in-place path (`crates/nvs-ir/src/lower/stmt.rs:564`). A property, a
  static property and an element take the `$t = $t . e` rewrite
  (`crates/nvs-ir/src/lower/stmt.rs:665`). The slot keeps its reference while the join runs, so the
  join copies the whole string. `6fdcc01e1a` fixed this shape inside `Core\Xml\Writer` alone.
- **`$xs = Run::push($xs, $i)`.** 96,388,527 bytes, then 384,532,527. A variable passed by value is
  retained before the call (`crates/nvs-ir/src/lower/call.rs:499`). The callee sees a count of two
  and its first write copies the array. The caller's variable is dead the moment the call returns.

Who reaches it: a remote client. A handler that groups a JSON body or a query result by a key runs
the first shape on a list the client sizes. A page built in a property runs the second. The peak
stays near twice the row, because each copy is freed at once. So the memory ceiling never stops the
request, and only `cpu_time` does.

Three smaller costs sit on the same paths, and none grows faster than its input.

- `$out .= "<td>{$i}</td>"` joins its parts into a fresh string and then copies that string into the
  target (`crates/nvs-ir/src/lower/stmt.rs:1047`). A `uint` hole is first rendered into a string of
  its own.
- Every retain and release is a call (`crates/nvs-codegen/src/emit.rs:4338`). A string literal is an
  immortal header (`crates/nvs-runtime/src/string.rs:367`), and each use still ends in a call that
  tests the sentinel and returns.
- A retain of a shared value is a load, a compare, an add and a store. Compiled code calls a helper
  for it and saves every caller-saved register around the call.

Nothing here is unsafe today. Every copy is a correct answer to copy-on-write
(`rule:types/arrays`). The goal removes the copies nobody can observe.

## Standing decisions

**These are fixes of faster than linear growth in ordinary web code, not an optimising tier.** The
shapes are grouping rows, building a string in a property, and passing an array through a function.
No inline cache is added, no second compile tier and no new helper ABI. Each new runtime entry is
declared with `nvs_helper!` (`crates/nvs-runtime/src/abi.rs:853`) or sits beside the string
primitives with their signature. **What it spends:** no memory per request in any stage.

**`values-stay-alive` is this goal's gate.** That goal changes when a read of shared storage owns its
value. It also makes a store lower its value before it reads its target's slot. Every in-place path
here builds on that order. The gate holds when `docs/goals/values-stay-alive.md` is gone and
`crates/nvs-codegen/tests/borrowed_stores.rs` holds
`every_store_form_keeps_its_target_alive_while_the_value_runs_code`. No stage starts before it holds.

**A write is in place only when the count says nothing else holds the value.** Security and request
isolation come first. The runtime decides at run time, by a reference count of exactly one, in the
function that already decides it (`crates/nvs-runtime/src/array.rs:1534`,
`crates/nvs-runtime/src/string.rs:1837`). The compiler never proves uniqueness. It only stops adding
a second reference of its own. A value another variable, a `foreach`, a closure or a second
property holds has a count above one and is copied as today. Each in-place path has a test in which
the value is shared and the copy still happens.

**The slot is read after the last operand that can run code, and the write consumes the slot's own
reference.** The receiver and the keys are evaluated once, left to right, as now. Between the load
of the slot and the store back, the lowering emits only the write itself. No release runs program
code, because there are no destructors (`rule:classes/no-destructors`). Safe default: where a form
cannot read its slot late, it keeps the copy, and its guard test is not written for that form.

**A nested write is one write-through call.** `Helper::ArrayWriteThrough` replaces
`Helper::ArrayRowForWrite` and the climbing `ArraySet` chain. Its arguments are the root, the inner
keys, the last key and the value. It walks the chain under each parent's `RefCell` borrow and calls
`make_unique` on a level only where that level's count is above one. It builds an absent row and
inserts it with the write that needed it. **What it spends:** one call where a write of depth two
made three, and no copy of an unshared row. Trade-offs: the lowering loses its descent and climb,
and the runtime gains one recursive function. Usability does not change.

**The write-through keeps the ownership rules `ArraySet` has.** It consumes one reference to the root
and returns the one reference to store back, so `write_back_array` is unchanged
(`crates/nvs-ir/src/lower/mod.rs:3789`). It consumes the value. Keys are borrowed, and the helper
retains a key it stores. On a failure the root is exactly as it arrived, the value is released, and
no row was inserted. An append can throw (`crates/nvs-runtime/src/array.rs:2184`), so this is tested.

**The inner keys travel without an allocation at depth two.** The second argument is `null` for no
inner level, and a `string` or `int` for one keyed level. A deeper chain or an inner append passes
an array of keys. An array is never a key, so its tag tells the forms apart. **What it spends:** one
small array per write at depth three or more, freed by the same statement.

**Declined: `ArrayRowForWrite` moving the row out of an unshared outer array.** The outer array would
hold an empty slot between two instructions. An append that throws would then lose the row unless
the lowering put it back on the error edge. That is an ownership rule in the lowering, where the
write-through keeps the whole decision in one Rust function.

**An append to a plain property slot is load, append, store.** A plain slot is a `string` property
with no hook, no `PropertyGuard` and no observer, on a class known while compiling
(`rule:classes/property-observer-pipeline`). The lowering emits `FieldGet`, `StrAppend`, `FieldSet`
with no retain and no release, as `write_back_array` does for `$this->list[] = $v`. A static
`string` property takes the same three steps. Every other property keeps the rewrite, so a guard,
hook or observer is called exactly as now. **What it spends:** two calls fewer per append.

**A suffix read from shared storage is retained across the append.** `$this->out .= $this->out` and
`$a[$k] .= $a[$k]` then see a count of two and copy. Safe default: any suffix the lowering cannot
judge is retained.

**An append to an element is a second write-through leaf.** `Helper::ArrayStrAppendThrough` takes the
root, the inner keys, the last key and the suffix. It never builds a row. An absent key at any level
throws what the read throws today (`rule:types/absent-storage-is-never-a-zero-value`). It applies
when the element type is `string`. A `?string` or `mixed` element keeps the rewrite.

**An element append under a guarded or observed root property keeps the rewrite.** A
`PropertyGuard` is given the new element and an observer the whole new array, so each needs the
appended string as a value of its own before or after the write. The in-place call never builds
it, so keeping the guard in front of the helper would mean building the copy the helper exists to
avoid. Security first: the guard and observer calls stay exactly as they are, and only the plain
roots (a local, a static property, a property with neither interface) take the helper. **What it
spends:** nothing; a guarded root costs what it costs today.

**`StrAppend` takes parts.** Its suffix becomes a list of `ConcatPart`. One `Str` part calls
`nvs_str_append` as now. Any other list calls a new `nvs_str_append_parts`, which measures the
parts, grows the target once and writes each part into the tail. A `Uint` part joins the `Int`
part, so a `uint` hole allocates no string. The runtime copies when a part is the target itself.
That check is one pointer compare per part, and it keeps the rule in one place. **What it spends:**
one call per append where there were two, and no temporary string.

**A local passed at its last use is moved, and the compiler alone decides it.** Nothing changes for
a Novis programmer. The statement is `$x = call(…, $x, …)` or `return call(…, $x, …)`. All of
these hold, and one function in the lowering asks them:

- `$x` is a local whose representation is `Ty::Array` or `Ty::Str`, and not an `inout` parameter.
- `$x` is written exactly once in the statement, anonymous function bodies included.
- That one place is a written argument of the statement's own call, passed by value to a parameter
  the callee owns. The call is not a nullsafe call.
- The statement is not inside the body of a `try` in this function.
- The function is not a generator.

The lowering then emits no retain, and takes `$x` out of `Env` before the next argument is lowered.
So no error edge and no scope end releases it. A failing later argument releases it once, as a
staged transferred value. **What it spends:** one scan of the statement for the name while
compiling, and one retain and one release fewer per call. Safe default: any doubt keeps the retain.

**This is not the `inout` move, and the `inout` copy stays.** An `inout` argument's holder is live
after the call by definition. A callee that throws must leave it unchanged
(`rule:statements/inout-is-the-by-reference-spelling`), and a property holder is reachable through
the object while the callee runs. So `RefSlot` retains what it stages
(`crates/nvs-ir/src/ty.rs:286`), and the callee's first write copies. Moving there would lend out a
live holder's only reference. The by-value move transfers the reference of a variable that syntax
proves dead. It does add one fact to the lowering: a moved local is absent from `Env`. The evidence
for it is the accumulating loop in *Why*, a real program shape. That evidence does not cover
`inout`, and the rule fragment gains the declined entry.

**A literal's retain and release are dropped in codegen.** `emit_refcount` is the one place every
count operation passes. It emits nothing when the operand's defining instruction is `ConstStr`,
`ConstBytes`, `ConstMarkup` or `ConstDuration`. `ConstDuration` joins the three because its
object is immortal in the data section on exactly `ConstMarkup`'s terms. One predicate on
`InstKind`, `is_immortal_literal`, names the four. A value that reaches a block parameter keeps its
calls. **What it spends:** one set entry per literal while compiling.
Declined: dropping them in the lowering, which has many emit sites and would rewrite most snapshots.

**`nvs_codegen::clif` names each import's symbol.** Cranelift prints an import as `fn3 = u0:17`,
so a `;; fn3 = nvs_str_retain` line follows each function. A test then reads which runtime entries
a function calls. **What it spends:** nothing outside `clif`, which only tests call.

**The count update is inline, and every free stays in Rust.** For `Ty::Str`, `Ty::Object` and
`Ty::Array`, a retain is a null test, a load, a sentinel compare and a store. A release stores the
count minus one when the count is above one, and calls today's helper when it is exactly one.
`Ty::Tagged` stays a call. **What it spends:** several instructions and one or two blocks per site,
where a call was one instruction. So the JIT output and the compile time per function both grow.
The stage measures `benches/compare/cases/html-page/main.nvs` and the `json-api` case beside it,
and writes the figures into the module doc. Safe default: if their instruction count grows by more
than a tenth, only the retain is inline.

**The safe default applied: only the retain is inline, and every release is a call.** The six
entries have no effect beyond the count write in a non-test build, so no kind had to stay a call
for that reason. The figures, from `nvs run --dump-asm`: `html-page` 2,247 before, 3,946 with both
inline (76%), 2,755 with the release a call in error landing blocks only (23%), and 2,342 with the
retain alone (4.2%). `json-api` 3,875, 6,043, 4,825 and 4,051 (4.5%). The growth is mostly
Cranelift's edge moves around the extra blocks. So the stage guards count retains: a debug-only
`budget::retain_calls()` beside `refcount_calls()`. The stage, the summary and check
`refcount-inline` now say "a retain". **What it spends:** 95 and 176 instructions in the two
cases, and nothing per request.

**The inline form adds one invariant, asserted in one place.** Compiled code now relies on four
facts. The count is the first word of each header, at the published offset. It is one machine word
and is never touched from another thread. A string or object count equal to `IMMORTAL_REFCOUNT` is
never written, and an array has no such sentinel. A decrement that leaves the count above zero does
nothing else. A `const` block beside `emit_refcount` asserts the offsets, the width and the
sentinel. `compiled_code_and_the_runtime_agree_on_the_count_word` runs both paths on each kind.

**Every guard counts, and none reads a clock.** A growth guard runs one program at 2,000 and 4,000
iterations and reads `nvs_runtime::budget::allocated_bytes()` around the run
(`rule:testing/bench-counters`). It fails when the larger run allocates three times the smaller or
more. The helper-call guards read a counter of refcount helper entries that exists only under
`debug_assertions`, beside `allocations()` (`crates/nvs-runtime/src/budget.rs:1182`). Release
builds pay nothing for it.

**Perf figures are recorded again at the goal end.** These stages move code under most `lang:`
benches. Stage 6's last items run `bun nv proofs --gate` and record each stale figure.

**Lanes:** one lane. Stages 1 to 4 all edit `crates/nvs-ir/src/lower/stmt.rs` and share
`crates/nvs-codegen/tests/in_place_writes.rs`, so they run in order. Stages 5 and 6 edit
`crates/nvs-codegen/src/emit.rs`, which stages 1 and 3 also touch, so they follow.

## Stages

### 1. a nested write into an unshared array happens in place

**Does:** replace the row descent and the `ArraySet` climb with one write-through call that copies
a level only when its count is above one.

- [x] Check the gate: `docs/goals/values-stay-alive.md` is gone, and
  `crates/nvs-codegen/tests/borrowed_stores.rs:1` holds
  `every_store_form_keeps_its_target_alive_while_the_value_runs_code`. Read where the `Index` arm
  now loads its root (`crates/nvs-ir/src/lower/stmt.rs:1902`).
- [x] `NvsArray` gains the write-through walk: a recursive function over the key chain that makes a
  level unique only above a count of one, builds an absent row, and fails before it inserts
  (`rule:types/arrays`, `crates/nvs-runtime/src/array.rs:1534`,
  `crates/nvs-runtime/src/array.rs:2080`, `crates/nvs-runtime/src/array.rs:2184`).
- [x] Test `a_write_through_copies_only_a_level_that_is_shared` beside the array tests: every mix of
  shared and unshared root and row, an absent row, an inner append, and a refused append
  (`crates/nvs-runtime/src/array.rs:2639`).
- [x] `nvs_array_write_through` replaces `nvs_array_row_for_write`, with the ownership rules of the
  standing decision (`rule:errors/propagation`, `crates/nvs-runtime/src/helpers.rs:370`).
- [x] `Helper::ArrayWriteThrough` replaces `Helper::ArrayRowForWrite` in the enum, its doc, the
  printer and the symbol table (`crates/nvs-ir/src/ir.rs:2613`, `crates/nvs-ir/src/ir.rs:2483`,
  `crates/nvs-ir/src/print.rs:596`, `crates/nvs-codegen/src/emit.rs:4962`).
- [x] The `Index` arm emits one write-through for a nested target. Keys are borrowed, so the key
  retains go and a fresh key is an owned temporary. A single-level write is unchanged
  (`crates/nvs-ir/src/lower/stmt.rs:1958`, `crates/nvs-ir/src/lower/stmt.rs:1979`,
  `crates/nvs-ir/src/lower/stmt.rs:2011`).
- [x] Rewrite the comments that describe the descent, as wholes
  (`crates/nvs-ir/src/lower/stmt.rs:1841`, `crates/nvs-ir/src/lower/stmt.rs:2287`,
  `crates/nvs-runtime/src/helpers.rs:482`).
- [x] Replace `writing_through_a_nested_subscript_separates_every_level` and its snapshot with
  `a_nested_write_lowers_to_one_write_through` (`crates/nvs-ir/src/lower/tests.rs:2004`).
- [x] Create `crates/nvs-codegen/tests/in_place_writes.rs:1` with a `bytes_of_run` helper modelled
  on `allocations_of_run` (`crates/nvs-codegen/tests/arrays.rs:130`), and the three growth guards
  of check `nested-writes-in-place`. The property guard takes its value from a method call.
- [x] Tests `a_nested_write_into_a_shared_row_still_copies_it` and
  `a_nested_write_into_a_shared_array_still_copies_it`: a second variable, a `foreach` over the
  array, and a closure each keep the contents they had
  (`crates/nvs-codegen/tests/in_place_writes.rs:1`).
- [x] Test `a_nested_append_that_throws_leaves_the_array_as_it_was`: an append into a row whose
  next index is taken throws, the catch reads the array unchanged, and live bytes balance
  (`crates/nvs-codegen/tests/in_place_writes.rs:1`).
- [x] Conformance case `tests/conformance/lang/a-nested-write-leaves-a-shared-row-alone.nvst:1`, so
  the shared and unshared paths run under ASAN and Miri (`rule:types/arrays`).
- [x] Rewrite the module doc's nested-write bullet as a whole: what the write-through does and
  what it spends (`crates/nvs-ir/src/lib.rs:127`).
- [x] `rule:types/arrays` gains one bullet: a write copies a level only when another holder shares
  it, at every depth. Then run `bun nv rules --render` (`docs/rules/types/arrays.md:1`).

### 2. an append to a property or an element grows its string in place

**Does:** give `.=` on a plain property slot, a static property and a `string` element the in-place
append a local already has.

- [x] `lower_compound_assignment` gains the plain-slot arm: receiver held as `values-stay-alive`
  holds it, suffix lowered, then `FieldGet`, `StrAppend`, `FieldSet` back to back
  (`rule:types/arrays`, `crates/nvs-ir/src/lower/stmt.rs:554`,
  `crates/nvs-ir/src/lower/stmt.rs:1038`). The plain-slot test is the one `lower_store`'s property
  arm uses to emit a bare `FieldSet` (`crates/nvs-ir/src/lower/stmt.rs:1387`).
- [x] A static `string` property takes the same arm with its own load and store. If its store runs
  anything else, it keeps the rewrite and *Standing decisions* says why
  (`crates/nvs-ir/src/lower/stmt.rs:554`).
- [x] A suffix read from a property, a static property or an element is retained across the append
  and released after it (`crates/nvs-ir/src/lower/expr.rs:1020`).
- [x] The write-through walk gains the string-append leaf, which builds no row and throws on an
  absent key (`rule:types/absent-storage-is-never-a-zero-value`,
  `crates/nvs-runtime/src/array.rs:1534`, `crates/nvs-runtime/src/string.rs:1837`).
- [x] `nvs_array_str_append_through` and `Helper::ArrayStrAppendThrough`, in the enum, the printer
  and the symbol table (`crates/nvs-runtime/src/helpers.rs:370`, `crates/nvs-ir/src/ir.rs:2613`,
  `crates/nvs-ir/src/print.rs:596`, `crates/nvs-codegen/src/emit.rs:4962`).
- [x] `lower_compound_assignment` sends `.=` on an element of type `string` to that helper, at any
  depth, with the `PropertyGuard` call kept in front (`crates/nvs-ir/src/lower/stmt.rs:554`,
  `crates/nvs-ir/src/lower/stmt.rs:2049`).
- [x] Rewrite the docs that say only a local appends in place
  (`crates/nvs-ir/src/lower/stmt.rs:534`, `crates/nvs-ir/src/ir.rs:1680`).
- [x] The three growth guards of check `appends-in-place`. Each appends a piece a method returns,
  so the late read is under test (`crates/nvs-codegen/tests/in_place_writes.rs:1`).
- [x] Test `an_append_to_a_shared_string_still_copies_it`: a second variable keeps the old text for
  a property, a static property and an element, and so does a suffix that is the target
  (`crates/nvs-codegen/tests/in_place_writes.rs:1`).
- [x] Tests `an_append_to_an_absent_element_throws_what_the_read_throws` and
  `an_append_to_a_guarded_or_hooked_property_keeps_its_calls`
  (`rule:classes/property-guard-runs-before-the-write`,
  `crates/nvs-codegen/tests/in_place_writes.rs:1`).
- [x] Conformance case `tests/conformance/lang/an-append-leaves-a-shared-string-alone.nvst:1`
  (`rule:types/arrays`).

### 3. an append of an interpolated string writes its parts into the target

**Does:** remove the temporary string a `.=` builds from an interpolation or a `.` chain, and the
string a `uint` hole renders into.

- [x] `nvs_str_append_parts` beside `nvs_str_append`: measure, grow once, write each part into the
  tail, and copy when the target is shared or a part is the target (`rule:types/string-holes`,
  `crates/nvs-runtime/src/string.rs:1837`, `crates/nvs-runtime/src/string.rs:1745`).
- [x] Test `an_append_of_parts_writes_what_the_join_writes`: every part kind, alone and mixed,
  against a join followed by a plain append (`crates/nvs-runtime/src/string.rs:2559`).
- [x] `InstKind::StrAppend` takes `Vec<ConcatPart>` as its suffix, in the IR, its doc and the
  printer (`crates/nvs-ir/src/ir.rs:1684`, `crates/nvs-ir/src/print.rs:341`).
- [x] Codegen emits `nvs_str_append` for one `Str` part and `nvs_str_append_parts` for any other
  list, with the parts on the stack as the join passes them
  (`crates/nvs-codegen/src/emit.rs:1062`, `crates/nvs-codegen/src/emit.rs:4021`,
  `crates/nvs-codegen/src/emit.rs:4079`).
- [x] `lower_string_append` lowers an interpolation or a `.` chain to parts. Stage 2's property arm
  calls the same function (`crates/nvs-ir/src/lower/stmt.rs:1038`,
  `crates/nvs-ir/src/lower/expr.rs:2278`, `crates/nvs-ir/src/lower/expr.rs:3286`).
- [x] `PartKind::Uint` beside `PartKind::Int`, in the IR, the printer, the runtime's part kinds and
  the hole lowering (`crates/nvs-ir/src/ir.rs:2325`, `crates/nvs-ir/src/print.rs:334`,
  `crates/nvs-runtime/src/string.rs:1574`, `crates/nvs-ir/src/lower/expr.rs:2344`).
- [x] Test `a_uint_hole_joins_as_a_part`: the IR of an interpolation over a `uint` has no
  `helper.uint_to_string` (`crates/nvs-ir/src/lower/tests.rs:2004`).
- [x] Tests `an_interpolated_append_allocates_only_as_the_target_grows` and
  `an_interpolated_append_that_reads_its_target_still_copies_it`. The first reads
  `budget::allocations()`: 2,000 more appends make fewer than 200 more requests
  (`crates/nvs-codegen/tests/in_place_writes.rs:1`).

### 4. an array or string passed at its last use is moved

**Does:** stop retaining a local around the call in `$x = call(…, $x, …)` and
`return call(…, $x, …)`, under the conditions of the standing decision.

**Split:** before the first edit, an `Explore` subagent lists every reader of a frame's locals as
`file.rs:NN` and one line each: the landing block, the scope-end sweep, generator state, debug
probes and the collector.

- [x] One function decides the move for a statement and returns the local's name.
  `lower_reassignment` and the lowering of `return` call it. `release_all_locals` already skips a
  returned local through `except` (`rule:types/arrays`, `crates/nvs-ir/src/lower/stmt.rs:1112`,
  `crates/nvs-ir/src/lower/mod.rs:4249`).
- [x] The argument lowering passes a moved local with no retain, stages it as transferred, and takes
  it out of `Env`. The change sits in the one operand function `values-stay-alive` left, found by
  `OperandSite` (`crates/nvs-ir/src/lower/call.rs:238`, `crates/nvs-ir/src/lower/call.rs:485`).
- [x] The reassignment binds the result as a fresh binding at the local's declared type, with no
  release of an old value (`crates/nvs-ir/src/lower/stmt.rs:1366`).
- [x] Every reader the subagent listed is checked against a moved local. Each is static from `Env`,
  or its function kind joins the conditions (`crates/nvs-ir/src/lower/mod.rs:3279`).
- [x] Test `a_local_is_moved_only_where_every_condition_holds`: one program per condition, each with
  that one condition broken, and each keeps its `retain` (`crates/nvs-ir/src/lower/tests.rs:2004`).
- [x] Tests `an_array_passed_and_reassigned_allocates_in_proportion_to_what_it_adds` and
  `a_string_passed_and_returned_allocates_in_proportion_to_what_it_adds`. The second is a recursive
  walk that returns its accumulator (`crates/nvs-codegen/tests/in_place_writes.rs:1`).
- [x] Test `a_passed_array_another_variable_shares_is_still_copied`: `$ys = $xs` before the call,
  and `$ys` keeps its contents (`crates/nvs-codegen/tests/in_place_writes.rs:1`).
- [x] Test `a_moved_argument_is_released_once_on_every_way_out`: the callee throws, a later argument
  throws, and the call returns. Live bytes balance each time
  (`crates/nvs-codegen/tests/in_place_writes.rs:1`).
- [x] Conformance case `tests/conformance/lang/a-passed-array-a-second-variable-shares-is-copied.nvst:1`
  (`rule:types/arrays`).
- [x] The module doc gains the invariant: a moved local is absent from `Env`, and which function
  decides it (`crates/nvs-ir/src/lib.rs:127`).
- [x] `rule:statements/inout-is-the-by-reference-spelling` gains a Declined entry: moving the
  holder's reference into the cell, with the two reasons. Then run `bun nv rules --render`
  (`docs/rules/statements/inout-is-the-by-reference-spelling.md:39`).

### 5. a literal is never retained or released

**Does:** emit nothing for a retain or release whose operand a literal instruction defines.

- [x] `InstKind` gains one predicate that is true for `ConstStr`, `ConstBytes` and `ConstMarkup`
  (`crates/nvs-ir/src/ir.rs:916`, `crates/nvs-ir/src/ir.rs:932`).
- [x] Codegen records each value such an instruction defines, and the `Retain` and `Release` arms
  skip those operands (`crates/nvs-codegen/src/emit.rs:826`, `crates/nvs-codegen/src/emit.rs:1334`,
  `crates/nvs-codegen/src/emit.rs:4338`).
- [x] A counter of refcount helper entries under `debug_assertions`, bumped in the six typed entries
  and read by `budget::refcount_calls()` (`rule:testing/bench-counters`,
  `crates/nvs-runtime/src/budget.rs:1182`, `crates/nvs-runtime/src/string.rs:1979`,
  `crates/nvs-runtime/src/object.rs:4481`, `crates/nvs-runtime/src/array.rs:1906`).
- [x] Create `crates/nvs-codegen/tests/refcount_calls.rs:1`. Test
  `a_literal_is_never_retained_or_released` reads `nvs_codegen::clif` for a function whose only
  counted values are the three literal kinds, and finds no refcount symbol
  (`crates/nvs-codegen/tests/probes.rs:70`).
- [x] Test `a_row_of_literals_enters_no_refcount_helper`: a loop that echoes literals enters the
  helpers equally often at 2,000 and 4,000 rows (`crates/nvs-codegen/tests/refcount_calls.rs:1`).
- [x] Rewrite the literal's doc to say its count operations are not emitted
  (`crates/nvs-codegen/src/emit.rs:1748`).

### 6. a retain runs without a call

**Does:** emit the retain inline for strings, objects and arrays. The release stays a call, by the
safe default (*Standing decisions*).

**Split:** before the first edit, an `Explore` subagent lists every effect of the six typed retain
and release entries beyond the count write, in every build kind, as `file.rs:NN` and one line each.

- [x] If the list names an effect in a non-test build, that kind stays a call and *Standing
  decisions* says why (`crates/nvs-runtime/src/release.rs:57`).
- [x] Count the instructions `nvs run --dump-asm` prints for the `html-page` and `json-api` cases,
  before any change (`benches/compare/cases/html-page/main.nvs:1`,
  `crates/nvs-codegen/src/lib.rs:116`).
- [x] `emit_refcount` emits the inline retain, and the release stays a call by the safe default, for `Ty::Str`, `Ty::Bytes`, `Ty::Object` and `Ty::Array` (`rule:programs/memory-priority`,
  `crates/nvs-codegen/src/emit.rs:4338`).
- [x] A `const` block beside it asserts the three offsets, the word width and the sentinel
  (`crates/nvs-runtime/src/string.rs:216`, `crates/nvs-runtime/src/object.rs:3582`,
  `crates/nvs-runtime/src/array.rs:986`, `crates/nvs-runtime/src/string.rs:367`).
- [x] Each runtime entry's doc says compiled code repeats its fast path, and what must change with it
  (`crates/nvs-runtime/src/string.rs:1979`, `crates/nvs-runtime/src/string.rs:2019`,
  `crates/nvs-runtime/src/object.rs:4144`, `crates/nvs-runtime/src/array.rs:1703`).
- [x] Tests `a_row_that_retains_enters_no_retain_helper`, at 2,000 and 4,000 rows, and
  `compiled_code_names_no_retain_helper`
  (`crates/nvs-codegen/tests/refcount_calls.rs:1`).
- [x] Test `compiled_code_and_the_runtime_agree_on_the_count_word`: for each kind, a program
  retains and releases a value, and the count read from Rust matches at each step. A literal's
  header is never written (`crates/nvs-codegen/tests/refcount_calls.rs:1`).
- [x] Count the instructions again, apply the safe default if the growth is above a tenth, and write
  both figures and the invariant into the module doc (`crates/nvs-codegen/src/lib.rs:116`).
- [ ] Run `bun nv proofs --gate`, then `bun nv proofs --record-perf --force --id '<feature>'` for
  each stale figure. Lower a bench's `// bench: allocations N` line where its count dropped
  (`rule:testing/member-perf-ledger`, `benches/members/README.md:1`).

### 7. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 5 are done. Stage 5: `InstKind::is_immortal_literal`
(`crates/nvs-ir/src/ir.rs:2361`) is true for `ConstStr`, `ConstBytes`, `ConstMarkup` and
`ConstDuration`. `Emitter::define` records each such value in `Emitter::immortal_literals`, and
the `Retain` and `Release` arms (`crates/nvs-codegen/src/emit.rs:1350`) skip it.
`budget::refcount_calls()` (`crates/nvs-runtime/src/budget.rs:1278`) reads a debug-only thread
local that `count_refcount_call` bumps at the top of the six typed entries. `nvs_codegen::clif`
now prints a `;; fnN = <symbol>` line per import (`crates/nvs-codegen/src/lib.rs:2650`), which is
how `a_literal_is_never_retained_or_released` finds refcount symbols. Both tests of check
`literals-uncounted` pass, and both went red with the skip disabled. Build, clippy and rustfmt are
green; no verify ran.

Expect red in stage 7: IR snapshots under `crates/nvs-ir/src/lower/snapshots/` that print a
property, static property or element `.=` (stage 2: `str_append` or `array_str_append_through` in
place of `concat` and `array_set`), any snapshot of a `.=` whose suffix is an interpolation or a
`.` chain (now `str.append vN, <parts>` with no `concat`), any snapshot of an interpolation or a
`Core\Str::format` call over a `uint` (now `uint vN`, no `uint_to_string`), any snapshot of
`$x = f($x)` or `return f($x)` over an array or string local passed to a compiled function (stage
4: no `retain` of the local, and no `release` of its old value), and any test that pinned the old
read order of a `.=` whose suffix writes its own target. Any test that pinned `clif` text now
sees the `;;` import lines. The sweep items are stale: no `env-secret-proofs` check exists, and
nightly 43201099245 predates `7568aaa0fb`, which skips those Miri tests.

### Next

Stage 6. Start with its `**Split:**` line: the `Explore` list of every effect of the six typed
entries. The debug-only `count_refcount_call` is one, and it is a test counter, not a reason to
keep a call. Once the update is inline, compiled code reaches the counter only on the call path,
which is what the stage 6 guards count.

- [ ] Count `nvs run --dump-asm` instructions for `benches/compare/cases/html-page/main.nvs:1` and
  `json-api` before any change.
- [ ] `crates/nvs-codegen/src/emit.rs:4443`: `emit_refcount` emits the inline retain and release,
  calling the helper only at a count of one.
- [ ] `crates/nvs-runtime/src/string.rs:216`: the `const` block of offsets, width and sentinel.
- [ ] `crates/nvs-codegen/tests/refcount_calls.rs:1`: the three tests of check `refcount-inline`.

### Backlog

- `$this->p = $this->p . e`, the `=` spelling of a property or element append, keeps the rewrite.
- `.=` of an interpolation onto an element joins first, because the element helper takes one `Str`
  suffix (`crates/nvs-ir/src/lower/stmt.rs:728`).
- A parameter read once and stored into a property is still retained and released.
- A `foreach` element is retained and released each step, though the loop holds the array
  (`crates/nvs-ir/src/lower/control.rs:1234`).
- A `foreach` step is two helper calls, one for the slot and one for the value.
- `.=` through an `inout` string parameter keeps the rewrite (`crates/nvs-ir/src/lower/stmt.rs:568`).
- A move into a `$fn(...)` call, a variadic tail or a parameter that widens the argument (`?T`,
  `mixed`) keeps the retain; `lower_fixed_arg` is the only place that moves.
- `a_local_is_moved_only_where_every_condition_holds` has no generator or nullsafe program; both
  conditions are in `last_use_move`.
