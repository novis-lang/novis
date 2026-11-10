---
looper: 1
title: "no valid program reads freed memory, leaks at a stop or aborts the process"
summary: "Reached when a borrowed read holds its value across every later operand that can free it, a stop at a function's entry frees what the callee owns, an `inout` loop survives any change to its array, and no program shape reaches a panic inside a runtime primitive."
context: [crates/nvs-ir/src/lower/mod.rs, crates/nvs-runtime/src/abi.rs, docs/rules/errors/helper-abi.md]
stages:
  - id: 1
    title: a borrowed read owns its value across code that can run
    status: open
    context: [crates/nvs-ir/src/lower/call.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/lower/operator.rs, crates/nvs-syntax/src/ast.rs]
  - id: 2
    title: a store keeps its target alive while its value runs code
    status: open
    context: [crates/nvs-ir/src/lower/stmt.rs, crates/nvs-ir/src/lower/mutate.rs, crates/nvs-ir/src/lower/call.rs]
  - id: 3
    title: a stop at a function's entry releases what the callee owns
    status: open
    context: [crates/nvs-codegen/src/emit.rs, crates/nvs-ir/src/ir.rs, crates/nvs-codegen/tests/throwing.rs, docs/rules/errors/stack-depth.md]
  - id: 4
    title: an inout loop walks the array it began with and writes by key
    status: open
    context: [crates/nvs-ir/src/lower/control.rs, crates/nvs-runtime/src/array.rs, docs/rules/statements/inout-is-the-by-reference-spelling.md, docs/rules/iteration/foreach-subjects.md]
  - id: 5
    title: the array primitives hold no panic a program reaches
    status: open
    context: [crates/nvs-runtime/src/array.rs, crates/nvs-runtime/src/identity.rs, crates/nvs-runtime/src/string.rs]
  - id: 6
    title: the string, value and arithmetic primitives hold no panic a program reaches
    status: open
    context: [crates/nvs-runtime/src/string.rs, crates/nvs-runtime/src/value.rs, crates/nvs-runtime/src/arith.rs]
  - id: 7
    title: the object primitives hold no panic a program reaches
    status: open
    context: [crates/nvs-runtime/src/object.rs]
  - id: 8
    title: every other primitive holds no panic a program reaches
    status: open
    context: [crates/nvs-runtime/src/throwable.rs, crates/nvs-runtime/src/callable.rs, crates/nvs-runtime/src/ctx/safepoint.rs, crates/nvs-runtime/src/ctx/frames.rs]
  - id: 9
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: operands-hold-their-reads
    stage: 1
    name: "every operand site keeps a borrowed read alive while a later operand runs code or rebinds it"
    run: [cargo, test, --test, borrowed_reads]
    timeout: 1800
    tests:
      - every_operand_site_keeps_a_borrowed_read_alive_across_later_code
      - a_local_read_survives_a_later_operand_that_rebinds_it
  - id: the-free-path-stays-free
    stage: 1
    name: "a read followed only by literals and bare locals takes no reference, and every expression kind is judged"
    run: [cargo, test, --lib, --, a_read_followed_only_by_literals_and_locals_takes_no_reference, every_borrow_judgment_passes_the_one_operand_lowering]
    timeout: 1800
    tests:
      - a_read_followed_only_by_literals_and_locals_takes_no_reference
      - every_borrow_judgment_passes_the_one_operand_lowering
  - id: stores-hold-their-targets
    stage: 2
    name: "every store form writes the storage it read, whatever its value expression did"
    run: [cargo, test, --test, borrowed_stores]
    timeout: 1800
    tests:
      - every_store_form_keeps_its_target_alive_while_the_value_runs_code
  - id: entry-stop-releases
    stage: 3
    name: "a stop at an entry poll or a stack check leaves nothing allocated"
    run: [cargo, test, --test, throwing, --, a_limit_stop_at_a_functions_entry_releases_its_arguments, a_recursion_error_releases_the_stopped_frames_arguments]
    timeout: 1800
    tests:
      - a_limit_stop_at_a_functions_entry_releases_its_arguments
      - a_recursion_error_releases_the_stopped_frames_arguments
  - id: every-poll-has-an-edge
    stage: 3
    name: "every function kind polls with an error edge, and codegen refuses a poll without one"
    run: [cargo, test, --lib, --, every_function_kind_polls_with_an_error_edge, codegen_refuses_a_poll_with_no_error_edge]
    timeout: 1800
    tests:
      - every_function_kind_polls_with_an_error_edge
      - codegen_refuses_a_poll_with_no_error_edge
  - id: inout-loop-holds
    stage: 4
    name: "an inout loop survives removals, a replaced array and a swept array, and a write to a removed entry throws"
    run: [cargo, test, --test, foreach_inout]
    timeout: 1800
    tests:
      - a_write_to_a_removed_entry_throws_a_logic_error
      - an_inout_loop_survives_removals_that_sweep_the_array
      - an_inout_loop_over_a_replaced_array_never_panics
      - an_inout_loop_that_only_writes_copies_nothing
  - id: array-primitives
    stage: 5
    name: "every array primitive denies a panic in its own body"
    run: [cargo, test, --test, primitives_never_panic, --, the_array_primitives_deny_a_panic_in_their_own_body]
    timeout: 1800
    tests:
      - the_array_primitives_deny_a_panic_in_their_own_body
  - id: string-primitives
    stage: 6
    name: "every string, value and arithmetic primitive denies a panic in its own body"
    run: [cargo, test, --test, primitives_never_panic, --, the_string_and_value_primitives_deny_a_panic_in_their_own_body]
    timeout: 1800
    tests:
      - the_string_and_value_primitives_deny_a_panic_in_their_own_body
  - id: object-primitives
    stage: 7
    name: "every object primitive denies a panic in its own body"
    run: [cargo, test, --test, primitives_never_panic, --, the_object_primitives_deny_a_panic_in_their_own_body]
    timeout: 1800
    tests:
      - the_object_primitives_deny_a_panic_in_their_own_body
  - id: every-primitive
    stage: 8
    name: "every primitive in the runtime's source denies a panic in its own body, read from the source"
    run: [cargo, test, --test, primitives_never_panic]
    timeout: 1800
    tests:
      - every_primitive_denies_a_panic_in_its_own_body
---

# no valid program reads freed memory, leaks at a stop or aborts the process

## Why

Four defects in the compiler's lowering and the runtime's primitives. Any Novis source reaches each
one, so a package with an empty grant line does too, and a remote client drives the second.

**A read is used after a later operand freed it.** A read of `$obj->prop`, `$obj->arr[k]`,
`Class::$prop` or a local gives a pointer with no reference of its own
(`crates/nvs-ir/src/lower/mod.rs:5249`, `crates/nvs-ir/src/lower/mod.rs:4182`). Several consumers
lower every operand first and use the pointer afterwards. A later operand that runs code can
replace the property, and one that rebinds the local frees its old value. Seen on the release
binary, with a method `clobber` that assigns the property a new string:

- `echo $h->name . T::clobber($h);` prints another string's bytes
  (`crates/nvs-ir/src/lower/expr.rs:3207`, the late retain at `crates/nvs-ir/src/lower/expr.rs:3251`).
- `$h->name == T::same($h)` is `false` for equal text, and `match ($h->name)` takes `default`
  (`crates/nvs-ir/src/lower/operator.rs:859`, `crates/nvs-ir/src/lower/expr.rs:2039`).
- A `Core` member reads a freed argument: `(ArgOwnership::Borrowed, true) => {}` takes no reference
  (`crates/nvs-ir/src/lower/call.rs:497`). A callable's arguments pass the same arm
  (`crates/nvs-ir/src/lower/call.rs:1134`).
- `$h->items[T::key($h)]` throws "undefined array key" for a key that exists: the base array was
  freed while the key ran (`crates/nvs-ir/src/lower/expr.rs:6713`).
- `$s . T::change(inout $s) . T::fill('K')` prints the third operand's bytes in place of `$s`. So
  does `$v . ($v = T::fill('G')) . T::fill('K')`, and an element of a local array passed `inout`.
- The store forms `$o->s .= f()`, `$o->arr[] = f()`, `$o->arr[k] = f()`, `$m[$o->key] = f()` and
  `$a->b->c = f()` write into the old target after `f()` freed it
  (`crates/nvs-ir/src/lower/stmt.rs:1319`). Two of them abort the process.

The freed block returns to a per-thread cache that serves other values of the same size. A stale
pointer then reads another value's bytes, on a thread that also serves other requests.

**A stop at a function's entry leaks what the callee owns.** A callee owns its receiver and its
arguments from the call instruction on (`crates/nvs-ir/src/lower/mod.rs:231`). Its first instruction
is the entry poll, emitted before any parameter is bound and with no error edge
(`crates/nvs-ir/src/lower/mod.rs:3423`). Codegen returns the status directly there and at the stack
check in front of it (`crates/nvs-codegen/src/emit.rs:1456`, `crates/nvs-codegen/src/emit.rs:1404`).
One caught `RecursionError` leaks the innermost frame's arguments, and so does a memory, CPU-time,
wall-time or cancel stop that lands on an entry poll. Under `nvs serve` the bytes stay for the life
of the process, so the growth is O(requests served). Commit `b0c50da2a8` closed the same leak at the
loop back-edge poll and left the entry poll, "because nothing is bound there yet".

**A write through an `inout` loop variable aborts the process.** The loop keeps a position in the
live array (`crates/nvs-ir/src/lower/control.rs:1130`). A write to `$v` asks for the key at that
position (`crates/nvs-ir/src/lower/control.rs:1384`), and `nvs_array_key_at` holds
`.expect("a foreach cursor only names live entries")` (`crates/nvs-runtime/src/array.rs:2439`).
Positions do not survive the body's own changes. `unset` leaves a hole, and then sweeps every hole
once they outnumber the live entries (`crates/nvs-runtime/src/array.rs:892`). A copy-on-write copy
drops the holes too (`crates/nvs-runtime/src/array.rs:946`). So each of these aborts with exit 127:
`unset($a[$k]); $v = 'w';`, `$a = []; $v = 'w';`, and a loop that removes the entry before the
current one and then writes `$v`. No `catch` sees it.

**The class behind that abort.** `rule:errors/helper-abi` says every helper contains a panic to one
request. That holds for a helper written through `nvs_helper!` (`crates/nvs-runtime/src/abi.rs:853`).
The runtime also has about sixty primitives declared as a bare `extern "C" fn`, with no context and
no `catch_unwind` (`crates/nvs-runtime/src/string.rs:1272`). A panic in one cannot unwind, so it
aborts the process and every in-flight request on every core.

## Standing decisions

**A borrowed read gets its own reference when a later operand can free it, at one place.** One
function in `crates/nvs-ir/src/lower/mod.rs` lowers an operand. It takes the expressions that are
lowered after it and before its consumer uses it. When the read is borrowed and refcounted, and a
later expression can free it, the function emits the retain and stages the value with
`own_temporary` (`crates/nvs-ir/src/lower/call.rs:1153`). It then reports the value as one this
frame owns, so the consumer's existing sweep and every landing block release it. `aliasing_read`
becomes private to that function, so no consumer can judge a read without naming what follows it.
**What it spends:** one non-atomic increment and one decrement per held operand, per evaluation.
Nothing in memory. Declined: retaining every property read, which pays the pair on every
`$this->x` operand. Declined: a fix written consumer by consumer, which leaves the next consumer
open.

**The test for "can free it" is syntactic, and it has two halves.** A read through a property or a
static property is shared storage. It is held when any later operand is something other than a
literal or a bare local read. A read of a bare local, or an element chain rooted at a local with no
property in it, is storage only this frame can write. It is held only when a later operand writes
that local: an assignment, a compound assignment, `++` or `--`, a destructuring target, or an
`inout` argument. Both halves are one exhaustive `match` over `ExprKind` with no wildcard arm
(`crates/nvs-syntax/src/ast.rs:914`), so a new expression kind does not compile until it is judged.
So `$this->a . $this->b` and `$s . f($x)` cost nothing more. Safe default: if a form writes a local
in a way the scan cannot see, a local is held like shared storage, at one more pair per operand.

**Every operand site is named, and the proof reads the names.** The one function takes a value of a
public enum `OperandSite` with one variant per consumer and an `ALL` list. The matrix test builds a
program per variant from an exhaustive `match`. It fails naming a site whose first operand changed.
A second test reads the lowering's source and fails on a call of `aliasing_read` outside the one
function. Trade-off: one enum and one argument per call site, paid in simplicity, for a consumer
list a test can read.

**A store evaluates its value before it reads its target's storage.** For `$o->s .= e`,
`$o->arr[] = e`, `$o->arr[k] = e`, `$m[key] = e` and `$a->b->c = e`, the receiver chain and the key
are lowered as held operands, then `e`, and only then the slot is read and written. The observable
order of side effects does not change. **What it spends:** the same pair per held operand, and only
when `e` can run code. Safe default: where a form cannot re-read its slot, it holds the receiver.

**A `Core` member that suspends is covered by the same hold.** The caller's hold lasts until the
member returns, so another task of the request cannot free a borrowed argument during the wait. No
member needs a hold of its own for an argument.

**A stop at the entry poll or the stack check takes the frame's landing block.** Each entry binds
its parameters first, then emits the poll through the emitter the back-edge poll uses
(`crates/nvs-ir/src/lower/mod.rs:3409`). The edge-less emitter is deleted. Codegen sends the stack
check's status and the poll's status through `emit_status_check`
(`crates/nvs-codegen/src/emit.rs:4402`), which already refuses a missing edge. So a poll with no
error edge does not compile, and a later poll site cannot miss the release. **What it spends:** one
cold landing block per compiled function, in code size. The path where nothing is set keeps its one
load and one branch. Declined: a sweep at request end, which does not exist and would hide the
next leak.

**An `inout` loop walks the array it began with, and a write to `$v` goes to the subject by key.**
No rule states what a write through `$v` does once its entry is gone, so this goal decides it. The
loop retains its subject when it starts and reads positions from that array alone, as the by-value
loop does (`crates/nvs-ir/src/lower/control.rs:1045`). Nothing can change that array's positions, so
the two `expect`s are unreachable and each gets a comment that says so. A write to `$v` calls one
status-returning helper with the subject, the walked array, the position and the value:

- The subject is still the walked array, and only the binding and the loop hold it: the value is
  replaced in place at the position.
- Otherwise the helper writes the subject's entry that has the same key.
- The subject has no entry with that key: the helper throws a `LogicError` that says the entry was
  removed during the loop. The program can catch it. Nothing panics.

The loop visits the entries the array had when it began, as the by-value loop does. **What it
spends:** one retain and release per loop, one helper call per write in place of a key allocation
and a hashed lookup, and one cold landing block per write. A body that adds or removes entries
copies the array once, as it does in a by-value loop: that is O(entries) per loop, and twice the
array's bytes while the loop runs. A loop that only writes `$v` copies nothing. Safe default: if the
in-place branch proves unsound, every write goes by key and the first write copies the array once.
Declined: a compile error for changing the subject inside the loop, which removes a working idiom.
Declined: a position that follows a sweep, which needs a counter on every array.

**A panic in a primitive keeps aborting the process.** A primitive takes no context, so it cannot
report a `FATAL`, and after a panic the runtime's state is unknown. Going on would risk request
isolation, which is not traded. So the fix is to remove every panic a program shape can reach. Each
such site becomes an error the program can catch, or keeps its panic with a reason that says why no
program reaches it. Where the operation must report an error, it moves to a helper written through
`nvs_helper!`, with a landing block. **What it spends:** one cold landing block per moved site.
Declined: `catch_unwind` in every primitive, and a new helper ABI. A helper written through
`nvs_helper!` is out of this sweep: `rule:errors/helper-abi` already contains its panic to one
request.

**The sweep is enforced by lints, and a test reads the source.** Each primitive carries
`#[deny(clippy::expect_used, clippy::unwrap_used, clippy::panic, clippy::unreachable,
clippy::indexing_slicing, clippy::arithmetic_side_effects)]`. So does each function in
`nvs-runtime` that the stage's list shows a primitive calls. A site that stays is written
`#[expect(<lint>, reason = "no program reaches this: ...")]`. The test finds every bare
`extern "C" fn` in `crates/nvs-runtime/src/` and fails naming one without the attribute.
Trade-off: an attribute per function, for a rule the build holds.

**Lanes:** stages 1 and 2 share the lowering files and run in order. Stage 3 shares `mod.rs` with
stage 1 and `emit.rs` with stage 4, so stages 1 to 4 run in order. Stage 5 shares `array.rs` with
stage 4 and runs after it. Stages 6, 7 and 8 touch disjoint runtime files and may run side by side
once stage 5 has landed the test file.

## Stages

### 1. a borrowed read owns its value across code that can run

**Does:** add the one operand lowering with its two-part test, and move every read-side consumer to
it. Then prove every site with a matrix that reads the site list.

**Split:** before the first edit, an `Explore` subagent lists every call of `aliasing_read` and
`is_aliasing_read` under `crates/nvs-ir/src/lower/`, as `file.rs:NN` and the consumer's function.
The session writes the list under this stage as checkpoints.

- [ ] Find the rule for operand order with `bun nv brief --where operand` and `--where evaluation`.
  Extend it, or write a new fragment with the cost line and the Declined list above
  (`docs/rules/expressions/an-operand-keeps-its-value-while-later-operands-run.md:1`).
- [ ] Add `OperandSite` with `ALL`, and the one function that lowers an operand given what follows
  it (`docs/rules/expressions/an-operand-keeps-its-value-while-later-operands-run.md:1`,
  `crates/nvs-ir/src/lower/mod.rs:4182`, `crates/nvs-ir/src/lower/call.rs:1153`).
- [ ] Write the exhaustive `ExprKind` judgment: "can run code" and "writes this local"
  (`crates/nvs-ir/src/lower/mod.rs:5249`, `crates/nvs-syntax/src/ast.rs:914`).
- [ ] Move `.` concatenation and string holes to it, and delete the late retain
  (`crates/nvs-ir/src/lower/expr.rs:3207`, `crates/nvs-ir/src/lower/expr.rs:3251`,
  `crates/nvs-ir/src/lower/expr.rs:1020`).
- [ ] Move `==`, `!=`, the tagged comparisons and the `match` subject to it
  (`crates/nvs-ir/src/lower/operator.rs:859`, `crates/nvs-ir/src/lower/expr.rs:2039`).
- [ ] Move `Core` member arguments and callable arguments to it, so the `(Borrowed, true)` arm holds
  (`crates/nvs-ir/src/lower/call.rs:497`, `crates/nvs-ir/src/lower/call.rs:1134`).
- [ ] Move an index's base and key, and a property chain's base, to it
  (`crates/nvs-ir/src/lower/expr.rs:6713`, `crates/nvs-ir/src/lower/expr.rs:5689`).
- [ ] Move every other consumer the list names, then make `aliasing_read` private to the one function
  (`crates/nvs-ir/src/lower/mod.rs:4182`).
- [ ] Test `every_operand_site_keeps_a_borrowed_read_alive_across_later_code` in
  `crates/nvs-codegen/tests/borrowed_reads.rs:1`. For each `OperandSite::ALL` entry and each read
  form (property, element of a property array, static property), it runs a program whose later
  operand replaces the storage and allocates blocks of the same size. It fails naming the site.
- [ ] Test `a_local_read_survives_a_later_operand_that_rebinds_it` in the same file: a local and a
  local array's element, each followed by an `inout` argument and by an assignment operand
  (`crates/nvs-codegen/tests/borrowed_reads.rs:1`).
- [ ] Tests `a_read_followed_only_by_literals_and_locals_takes_no_reference` and
  `every_borrow_judgment_passes_the_one_operand_lowering` in the lowering tests. The first counts
  `Retain` instructions for `$this->a . $this->b` and for `$s . f($x)` at two operand counts. The
  second reads the lowering's source files (`crates/nvs-ir/src/lower/tests.rs:1`).
- [ ] Attack `tests/hostile/lang/expressions/string-operators/02-an-operand-that-frees-the-one-before-it.nvs`:
  each consumer once, with a later operand that replaces the property
  (`rule:testing/feature-proofs`, `tests/hostile/lang/expressions/string-operators/02-an-operand-that-frees-the-one-before-it.nvs:1`).

### 2. a store keeps its target alive while its value runs code

**Does:** lower the value of every store form before the target's slot is read, with the receiver
chain and key held by stage 1's function.

- [ ] `lower_store` holds the receiver chain and the key, lowers the value, then reads the slot
  (`docs/rules/expressions/an-operand-keeps-its-value-while-later-operands-run.md:1`,
  `crates/nvs-ir/src/lower/stmt.rs:1319`).
- [ ] `lower_read_modify_write` does the same for `.=` and every compound form
  (`crates/nvs-ir/src/lower/stmt.rs:665`).
- [ ] The element stores `$o->arr[] = e`, `$o->arr[k] = e` and `$m[$o->key] = e` do the same
  (`crates/nvs-ir/src/lower/mutate.rs:1`, `crates/nvs-ir/src/lower/stmt.rs:1319`).
- [ ] Add each store form to `OperandSite`, so stage 1's source test covers it
  (`crates/nvs-ir/src/lower/mod.rs:4182`).
- [ ] Test `every_store_form_keeps_its_target_alive_while_the_value_runs_code` in
  `crates/nvs-codegen/tests/borrowed_stores.rs:1`. It runs the five forms from *Why*, each with a
  value that replaces the target, and reads the storage back. It fails naming the form.
- [ ] Attack `tests/hostile/lang/expressions/assignment/02-a-value-that-replaces-its-own-target.nvs`
  (`rule:testing/feature-proofs`, `tests/hostile/lang/expressions/assignment/02-a-value-that-replaces-its-own-target.nvs:1`).

### 3. a stop at a function's entry releases what the callee owns

**Does:** bind the parameters before the entry poll, give that poll and the stack check the frame's
landing block, and make codegen refuse a poll with no error edge.

- [ ] Delete the edge-less emitter. Every entry emits its poll through the back-edge emitter, after
  its last `Param` and before any other instruction (`rule:errors/stack-depth`,
  `crates/nvs-ir/src/lower/mod.rs:3423`, `crates/nvs-ir/src/lower/mod.rs:3409`).
- [ ] The three method entries (`crates/nvs-ir/src/lower/mod.rs:1469`,
  `crates/nvs-ir/src/lower/mod.rs:1904`, `crates/nvs-ir/src/lower/mod.rs:2076`).
- [ ] The two anonymous-function entries (`crates/nvs-ir/src/lower/anon_fn.rs:196`,
  `crates/nvs-ir/src/lower/anon_fn.rs:715`).
- [ ] The four generator entries (`crates/nvs-ir/src/lower/generator.rs:661`,
  `crates/nvs-ir/src/lower/generator.rs:813`, `crates/nvs-ir/src/lower/generator.rs:985`,
  `crates/nvs-ir/src/lower/generator.rs:1152`).
- [ ] The two hand-built functions, whose landing block releases the receiver and each argument
  (`crates/nvs-ir/src/lower/call.rs:1491`, `crates/nvs-ir/src/lower/exception.rs:1191`).
- [ ] Codegen: the poll and the stack check send their status through `emit_status_check`, and the
  direct return is deleted (`crates/nvs-codegen/src/emit.rs:1451`,
  `crates/nvs-codegen/src/emit.rs:1400`, `crates/nvs-codegen/src/emit.rs:4402`).
- [ ] Rewrite the two doc comments that say nothing is bound at entry
  (`crates/nvs-ir/src/ir.rs:735`, `crates/nvs-codegen/src/emit.rs:1423`).
- [ ] Tests `a_limit_stop_at_a_functions_entry_releases_its_arguments` and
  `a_recursion_error_releases_the_stopped_frames_arguments`, beside the back-edge guard. Each calls
  a method that holds a large string argument and compares live bytes before and after
  (`crates/nvs-codegen/tests/throwing.rs:398`).
- [ ] Tests `every_function_kind_polls_with_an_error_edge` in the lowering tests and
  `codegen_refuses_a_poll_with_no_error_edge` in `nvs-codegen`. The first lowers one fixture with
  every function kind above and walks every `Safepoint`
  (`crates/nvs-ir/src/lower/tests.rs:1`, `crates/nvs-codegen/src/emit.rs:1427`).
- [ ] `rule:errors/stack-depth` gains one sentence: a stop at entry releases the frame's arguments
  (`docs/rules/errors/stack-depth.md:5`).
- [ ] Update the IR snapshots the moved poll changes, with `cargo test --lib` run once for
  `nvs-ir`'s snapshot tests (`crates/nvs-ir/src/lower/tests.rs:1`).

### 4. an inout loop walks the array it began with and writes by key

**Does:** make the by-reference loop retain its subject and read positions from it, and replace the
write-through with one status-returning helper that writes in place or by key.

- [ ] Write the rule fragment for an `inout` loop from the standing decision,
  with its cost and Declined list, and point `rule:statements/inout-is-the-by-reference-spelling`
  at it (`docs/rules/iteration/an-inout-loop-walks-the-array-it-began-with.md:1`,
  `docs/rules/statements/inout-is-the-by-reference-spelling.md:16`).
- [ ] The loop retains the subject at its start and releases it on every exit, as the by-value loop
  does. The header reads positions from the walked array
  (`crates/nvs-ir/src/lower/control.rs:1130`, `crates/nvs-ir/src/lower/control.rs:1045`).
- [ ] Add the helper through `nvs_helper!`: subject, walked array, position and value in, the
  subject out. It consumes the value on every path (`crates/nvs-runtime/src/array.rs:2425`,
  `crates/nvs-ir/src/ir.rs:2488`).
- [ ] `write_through_element` emits that helper as a fallible instruction and stores the returned
  subject. The `ArrayKeyAt` and `ArraySet` pair goes (`crates/nvs-ir/src/lower/control.rs:1363`).
- [ ] The two position `expect`s keep their panic, each with the reason: the walked array is
  retained, so nothing changes it (`crates/nvs-runtime/src/array.rs:2439`,
  `crates/nvs-runtime/src/array.rs:2473`).
- [ ] Tests in `crates/nvs-codegen/tests/foreach_inout.rs:1`:
  `a_write_to_a_removed_entry_throws_a_logic_error`,
  `an_inout_loop_survives_removals_that_sweep_the_array` (forty entries, each visit removes the one
  before it and writes `$v`), `an_inout_loop_over_a_replaced_array_never_panics` (the subject
  assigned, passed `inout`, and the nested loop form), and
  `an_inout_loop_that_only_writes_copies_nothing` (allocations counted at two array sizes).
- [ ] Conformance cases for the rule's observable results: the visit count, the thrown class and its
  message (`docs/rules/iteration/an-inout-loop-walks-the-array-it-began-with.md:1`,
  `tests/hostile/lang/statements/foreach/01-a-million-elements-walked-written-and-changed-under-the-loop.nvs:1`).
  Find the `.nvst` home with `bun nv brief --where conformance`.
- [ ] Attack `tests/hostile/lang/statements/foreach/02-an-inout-loop-whose-array-changes-under-it.nvs`:
  the three aborting shapes from *Why*, each caught, so the next step runs
  (`rule:testing/feature-proofs`, `tests/hostile/lang/statements/foreach/02-an-inout-loop-whose-array-changes-under-it.nvs:1`).
- [ ] The `foreach` example's `about.md` says what a write does after the entry was removed. Find it
  with `bun nv proofs --id 'lang:statements/foreach'` (`docs/examples/lang/statements/foreach/about.md:1`).

### 5. the array primitives hold no panic a program reaches

**Does:** list every panic site reachable from a primitive, write the test that reads the source,
and clear the array group.

**Split:** before the first edit, an `Explore` subagent lists every `expect`, `unwrap`, index,
arithmetic that can overflow, `borrow_mut`, `panic!` and `unreachable!` reachable from a bare
`extern "C" fn` in `crates/nvs-runtime/src/`. The list is grouped by file, as `file.rs:NN`, the
primitive and one line on what input reaches it. It also names every bare `extern "C" fn` in
`crates/nvs-stdlib/src/` and `crates/nvs-host/src/` whose body runs outside `run_helper`. The
session writes each group under its stage as checkpoints, so stages 6 to 8 start from the list.

- [ ] Correct `rule:errors/helper-abi`: a primitive has no wrapper, a panic in one aborts the
  process, and no program shape may reach one (`docs/rules/errors/helper-abi.md:1`). Correct the
  same sentence in the runtime's docs (`crates/nvs-runtime/src/lib.rs:23`,
  `crates/nvs-runtime/src/abi.rs:13`).
- [ ] Write `crates/nvs-runtime/tests/primitives_never_panic.rs:1`. One function finds every bare
  `extern "C" fn` in a source file and checks for the `deny` attribute. The four tests call it over
  their file groups, and `every_primitive_denies_a_panic_in_its_own_body` walks the whole directory.
- [ ] The fifteen primitives in `array.rs` and `nvs_array_eq` get the attribute. Each site the list
  names becomes an error or an `#[expect]` with its reason (`rule:errors/helper-abi`,
  `crates/nvs-runtime/src/array.rs:1863`, `crates/nvs-runtime/src/identity.rs:180`).
- [ ] The table methods those primitives call get the same treatment: `remove`, `rename`, `compact`,
  `make_unique` and the position reads (`crates/nvs-runtime/src/array.rs:884`,
  `crates/nvs-runtime/src/array.rs:900`, `crates/nvs-runtime/src/array.rs:914`,
  `crates/nvs-runtime/src/array.rs:1534`).
- [ ] Attack `tests/hostile/lang/expressions/arrays-in-expressions/02-every-array-operation-at-its-edge.nvs`:
  one step per site the list marked reachable
  (`rule:testing/feature-proofs`, `tests/hostile/lang/expressions/arrays-in-expressions/02-every-array-operation-at-its-edge.nvs:1`).

### 6. the string, value and arithmetic primitives hold no panic a program reaches

**Does:** clear the string group from stage 5's list.

- [ ] The eight primitives in `string.rs` get the attribute, and each listed site is resolved. The
  subtraction in the append path is one of them (`rule:errors/helper-abi`,
  `crates/nvs-runtime/src/string.rs:1307`, `crates/nvs-runtime/src/string.rs:1231`).
- [ ] `nvs_value_retain`, `nvs_value_release` and `nvs_float_pow` get the attribute
  (`crates/nvs-runtime/src/value.rs:895`, `crates/nvs-runtime/src/value.rs:928`,
  `crates/nvs-runtime/src/arith.rs:32`).
- [ ] The comment above the string primitives says how the rule is held
  (`crates/nvs-runtime/src/string.rs:1289`).

### 7. the object primitives hold no panic a program reaches

**Does:** clear the object group from stage 5's list.

- [ ] The seventeen primitives in `object.rs` get the attribute, and each listed site is resolved
  (`rule:errors/helper-abi`, `crates/nvs-runtime/src/object.rs:4342`,
  `crates/nvs-runtime/src/object.rs:6251`).
- [ ] The release path they share with arrays gets the same treatment
  (`crates/nvs-runtime/src/release.rs:57`).
- [ ] Attack `tests/hostile/lang/classes/properties/02-every-slot-read-and-write-at-its-edge.nvs`:
  one step per site the list marked reachable
  (`rule:testing/feature-proofs`, `tests/hostile/lang/classes/properties/02-every-slot-read-and-write-at-its-edge.nvs:1`).

### 8. every other primitive holds no panic a program reaches

**Does:** clear the remaining files, and the primitives the list found in other crates.

- [ ] `throwable.rs`, `callable.rs`, `extension.rs`, `deferred.rs` and `lib.rs`
  (`rule:errors/helper-abi`, `crates/nvs-runtime/src/throwable.rs:1025`,
  `crates/nvs-runtime/src/callable.rs:345`, `crates/nvs-runtime/src/extension.rs:101`).
- [ ] The files under `ctx/`: `safepoint.rs`, `frames.rs`, `trace.rs`, `hooks.rs`, `isolate.rs` and
  `deprecated.rs` (`crates/nvs-runtime/src/ctx/safepoint.rs:1173`,
  `crates/nvs-runtime/src/ctx/frames.rs:444`, `crates/nvs-runtime/src/ctx/trace.rs:767`,
  `crates/nvs-runtime/src/ctx/deprecated.rs:89`).
- [ ] Each function the list named in `nvs-stdlib` and `nvs-host` runs its body inside `run_helper`,
  or gets the attribute (`crates/nvs-runtime/src/abi.rs:518`).
- [ ] `every_primitive_denies_a_panic_in_its_own_body` is green over the whole directory
  (`crates/nvs-runtime/tests/primitives_never_panic.rs:1`).

### 9. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Nothing is built yet. The tree holds every defect in *Why*: the late retain in `lower_concat`, the
empty `(Borrowed, true)` arm, the edge-less entry poll, and the position `expect` in
`nvs_array_key_at`. No test named in the checks exists.

### Next

Stage 1, over `crates/nvs-ir/src/lower/mod.rs`, `call.rs`, `expr.rs` and `operator.rs`. An `Explore`
subagent first lists every call of `aliasing_read` and `is_aliasing_read` under
`crates/nvs-ir/src/lower/`.

- [ ] `docs/rules/expressions/an-operand-keeps-its-value-while-later-operands-run.md:1`: find the
  rule for operand order with `bun nv brief --where operand` and `--where evaluation`. Extend it, or
  write that fragment.
- [ ] `crates/nvs-ir/src/lower/mod.rs:4182`: add `OperandSite` with `ALL`, and the one function that
  lowers an operand given what follows it (`crates/nvs-ir/src/lower/call.rs:1153`).
- [ ] `crates/nvs-ir/src/lower/mod.rs:5249`: write the exhaustive `ExprKind` judgment, "can run
  code" and "writes this local" (`crates/nvs-syntax/src/ast.rs:914`).
- [ ] `crates/nvs-ir/src/lower/expr.rs:3207`: move `.` concatenation and string holes to it, and
  delete the late retain (`crates/nvs-ir/src/lower/expr.rs:3251`,
  `crates/nvs-ir/src/lower/expr.rs:1020`).
- [ ] `crates/nvs-ir/src/lower/operator.rs:859`: move `==`, `!=`, the tagged comparisons and the
  `match` subject to it (`crates/nvs-ir/src/lower/expr.rs:2039`).
- [ ] `crates/nvs-ir/src/lower/call.rs:497`: move `Core` member arguments and callable arguments to
  it (`crates/nvs-ir/src/lower/call.rs:1134`).
- [ ] `crates/nvs-ir/src/lower/expr.rs:6713`: move an index's base and key, and a property chain's
  base, to it (`crates/nvs-ir/src/lower/expr.rs:5689`).
- [ ] `crates/nvs-ir/src/lower/mod.rs:4182`: move every other consumer the list names, then make
  `aliasing_read` private to the one function.
- [ ] `crates/nvs-codegen/tests/borrowed_reads.rs:1`: tests
  `every_operand_site_keeps_a_borrowed_read_alive_across_later_code` and
  `a_local_read_survives_a_later_operand_that_rebinds_it`.
- [ ] `crates/nvs-ir/src/lower/tests.rs:1`: tests
  `a_read_followed_only_by_literals_and_locals_takes_no_reference` and
  `every_borrow_judgment_passes_the_one_operand_lowering`.
- [ ] `tests/hostile/lang/expressions/string-operators/02-an-operand-that-frees-the-one-before-it.nvs:1`:
  the attack, each consumer once, with a later operand that replaces the property.

Expected red until stage 3 lands: none. Stage 3 changes IR snapshots under
`crates/nvs-ir/src/lower/snapshots/`.

### Backlog
