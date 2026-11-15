---
looper: 1
title: "compiled code calls a helper only where it needs one, and an array key allocates only what it keeps"
summary: "Reached when a scalar test or widening is inline, a release that frees nothing is inline, an integer key on a hashed array allocates no text, and each change is pinned by a count."
context: [crates/nvs-codegen/src/emit.rs, crates/nvs-runtime/src/array.rs, crates/nvs-runtime/src/helpers.rs, docs/rules/programs/memory-priority.md, docs/rules/testing/perf-two-mechanisms.md]
stages:
  - id: 1
    title: a scalar test, a widening and a string comparison are inline
    status: done
    context: [crates/nvs-codegen/src/emit.rs, crates/nvs-ir/src/lower/convert.rs, crates/nvs-runtime/src/helpers.rs]
  - id: 2
    title: a release that frees nothing is inline
    status: open
    context: [crates/nvs-codegen/src/emit.rs, crates/nvs-runtime/src/string.rs, crates/nvs-runtime/src/object.rs, crates/nvs-runtime/src/array.rs, crates/nvs-runtime/src/budget.rs]
  - id: 3
    title: an array key allocates only what the array keeps
    status: open
    context: [crates/nvs-runtime/src/array.rs, crates/nvs-runtime/src/helpers.rs]
  - id: 4
    title: a spawned task and a freed container allocate once
    status: open
    context: [crates/nvs-runtime/src/ctx/wiring.rs, crates/nvs-runtime/src/ctx/isolate.rs, crates/nvs-runtime/src/release.rs]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: scalars-inline
    stage: 1
    name: "a truthiness test and a widening of a known scalar compile to no helper call"
    run: [cargo, test, --lib, --, a_known_scalar_test_and_widening_call_no_helper]
    timeout: 1800
    tests:
      - a_known_scalar_test_and_widening_call_no_helper
  - id: release-inline
    stage: 2
    name: "a release that leaves the count above zero calls no helper, and every count still balances"
    run: [cargo, test, --lib, --, a_release_that_frees_nothing_calls_no_helper]
    timeout: 1800
    tests:
      - a_release_that_frees_nothing_calls_no_helper
  - id: int-key-no-text
    stage: 3
    name: "an integer key read on a hashed array allocates nothing, and a write allocates the key alone"
    run: [cargo, test, --lib, --, an_integer_key_on_a_hashed_array_allocates_no_text]
    timeout: 1800
    tests:
      - an_integer_key_on_a_hashed_array_allocates_no_text
---

# compiled code calls a helper only where it needs one, and an array key allocates only what it keeps

## Why

`crates/nvs-codegen/src/lib.rs:140` says a statically known scalar gets a native instruction. Four
places do not follow it. Each was read in the code, and none was timed.

- **Truthiness and widening are helper calls.** `if ($n)`, `!$n`, `while ($f)`, `$int + $float` and
  `$n as float` become `IntTruthy`, `FloatTruthy`, `IntToFloat` and their siblings
  (`crates/nvs-ir/src/lower/convert.rs:519`, `:283`). Each call writes its argument to a stack slot,
  calls through the generic helper signature, tests a status and loads the result
  (`crates/nvs-codegen/src/emit.rs:892`, `:2913`). The helpers are one compare or one range test
  (`crates/nvs-runtime/src/helpers.rs:263`, `:1793`).
- **Every release is a call.** Retain is inline (`crates/nvs-codegen/src/emit.rs:4532`). Release
  calls `nvs_str_release`, `nvs_object_release` or `nvs_array_release` even when the value is null,
  immortal, or stays alive (`crates/nvs-runtime/src/string.rs:2359`, `object.rs:5258`,
  `array.rs:2390`).
- **An integer key on a hashed array builds a `String`.** `get_index` and `set_index` call
  `index.to_string()` (`crates/nvs-runtime/src/array.rs:589`, `:606`), and so do `:516`, `:649`,
  `:688`, `:789` and `:1129`. A read allocates once and a write twice.
- **A `mixed` key is copied.** `nvs_value_to_array_key` copies a string key into a new string, which
  also loses its cached hash (`crates/nvs-runtime/src/helpers.rs:772`).

Smaller costs sit beside them: string `==` always calls (`crates/nvs-codegen/src/emit.rs:2033`),
every `echo` re-validates UTF-8 (`crates/nvs-runtime/src/helpers.rs:3024`), a task spawn discards
two allocations and two random draws (`crates/nvs-runtime/src/ctx/wiring.rs:131`), and freeing a
hashed array regrows a vector (`crates/nvs-runtime/src/array.rs:872`).

## Standing decisions

**Limits: what stops this goal.**

- **Every inline path keeps its helper as the slow path.** The helper stays the only place an error
  is made or memory is freed. An item that would need a second copy of that logic in emitted code is
  declined.
- **A count pins each change, not a clock.** The checks count helper calls in emitted code and
  allocations in the runtime, with the debug-only counters `rule:testing/perf-two-mechanisms` names.
  No bench run decides anything in this goal.
- **Stage 2 stops at the first imbalance.** If the refcount balance tests show one leak or one double
  free that is not fixed inside the stage's two sessions, the inline release is taken out whole and
  recorded in `### Backlog`. Half of it is never kept.
- **An item gets one attempt, and a stage two sessions,** counted in `### State`. At the ceiling the
  open items move to `### Backlog` as declined.
- **No new `unsafe` block.** An item that needs one is declined.
- **No item is added.**

**The out-of-range widening still calls the helper.** The inline code tests the range and converts.
The other branch calls the existing helper, so the error text and code are the same.

**Inline release covers three cases: null, immortal, and a count above one.** Anything else calls
the existing helper, which sees the original count. Safe default if objects or arrays do more at a
decrement than strings do: inline strings and bytes only.

**The debug refcount counter counts an inline release too.** `count_refcount_call`
(`crates/nvs-runtime/src/budget.rs:1268`) feeds a guard, so the emitted code bumps it in debug
builds. **What it spends:** nothing in a release binary.

**The Cranelift verifier stays on in every build.** It turns an emitter bug into a compile error,
and a warm start never pays it. Declined: turning it off in release.

**A literal key's hash is not written at compile time.** Compiled units are cached across
processes, and the hash key is drawn per process. Declined.

**No pass removes retain and release pairs.** A wrong pair is a memory-safety bug, and nothing
measured asks for it. Declined until a program shows the cost.

**Lanes:** stages 1 and 2 share `crates/nvs-codegen/src/emit.rs` and run in order. Stages 3 and 4
touch only `crates/nvs-runtime/` and can run as a side goal beside them.

## Stages

### 1. a scalar test, a widening and a string comparison are inline

**Does:** emit native instructions for truthiness and widening of a known scalar, and test pointer
and length before a string comparison calls.

- [x] `crates/nvs-codegen/src/emit.rs:892`: `IntTruthy`, `UintTruthy` and `FloatTruthy` are one
  compare. NaN is truthy and -0.0 is not, as `crates/nvs-runtime/src/helpers.rs:277` says
  (`rule:errors/helper-abi`).
- [x] `crates/nvs-codegen/src/emit.rs:2913`: `IntToFloat`, `UintToFloat`, `IntToUint` and
  `UintToInt` test the range inline and call the helper outside it (`rule:errors/helper-abi`).
- [x] `crates/nvs-codegen/src/emit.rs:2033`: string and bytes `==` and `!=` answer from equal
  pointers or unequal lengths before the call (`rule:errors/helper-abi`).
- [x] `crates/nvs-codegen/src/emit.rs:3505`: a class test answers from an equal class word before
  the call (`rule:errors/helper-abi`).
- [x] `crates/nvs-codegen/src/lib.rs:140`: the test `a_known_scalar_test_and_widening_call_no_helper`,
  and the module doc names what is inline now (`rule:testing/perf-two-mechanisms`).

### 2. a release that frees nothing is inline

**Does:** emit the null, immortal and count-above-one cases of a release, and call the helper only
to free.

**Split:** an `Explore` subagent lists everything the three release helpers do before they free, as
`file.rs:NN`, before the first edit.

- [ ] `crates/nvs-codegen/src/emit.rs:4500`: the release branch of `emit_refcount`, for strings and
  bytes (`rule:errors/helper-abi`).
- [ ] `crates/nvs-codegen/src/emit.rs:1383`: the same for objects and arrays, when the list shows a
  decrement does no more for them than for a string (`rule:errors/helper-abi`).
- [ ] `crates/nvs-runtime/src/budget.rs:1268`: the debug counter counts an inline release
  (`rule:testing/perf-two-mechanisms`).
- [ ] `crates/nvs-codegen/src/emit.rs:4500`: the test `a_release_that_frees_nothing_calls_no_helper`.
  Run it and the refcount balance tests by name once (`rule:testing/perf-two-mechanisms`).

### 3. an array key allocates only what the array keeps

**Does:** format an integer key on the stack, keep a `mixed` string key by reference, and read
elements with one borrow.

- [ ] `crates/nvs-runtime/src/array.rs:589`: one `int_key_bytes` helper over a stack buffer, used at
  `:516`, `:589`, `:606`, `:649`, `:688`, `:789` and `:1129` (`rule:programs/memory-priority`).
- [ ] `crates/nvs-runtime/src/helpers.rs:772`: `nvs_value_to_array_key` retains a string key and
  formats an integer key with `decimal_digits` at `:100` (`rule:programs/memory-priority`).
- [ ] `crates/nvs-runtime/src/array.rs:872`: `take_values` reserves the live count
  (`rule:programs/memory-priority`).
- [ ] `crates/nvs-runtime/src/helpers.rs:3024`: `write_rendered` uses `as_text` for a string
  (`rule:programs/memory-priority`).
- [ ] `crates/nvs-runtime/src/helpers.rs:2513`: `every_element_has_tag` walks the values under one
  borrow (`rule:programs/memory-priority`).
- [ ] `crates/nvs-runtime/src/array.rs:2813`: `nvs_array_spread` reserves the source's count once
  through `try_reserve` at `:1775`, and keeps the check per entry (`rule:programs/memory-priority`).
- [ ] `crates/nvs-runtime/src/array.rs:584`: the test
  `an_integer_key_on_a_hashed_array_allocates_no_text` (`rule:testing/perf-two-mechanisms`).

### 4. a spawned task and a freed container allocate once

**Does:** build a child context from its parent's shared parts, and free a small tree without a
heap worklist.

- [ ] `crates/nvs-runtime/src/ctx/wiring.rs:131`: an internal constructor takes the tree and the
  deadline, and `Ctx::child` at `crates/nvs-runtime/src/ctx/isolate.rs:396` calls it. The trace id
  is still drawn per task, so exported spans do not change (`rule:security/no-cross-request-state`).
- [ ] `crates/nvs-runtime/src/ctx/isolate.rs:422`: `origin` is an `Arc<str>`
  (`rule:programs/memory-priority`).
- [ ] `crates/nvs-runtime/src/release.rs:84`: the first eight dying entries are held inline
  (`rule:programs/memory-priority`). Safe default if this needs `unsafe`: decline.

### 5. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 1 is done in `32223460f4`. Sessions used: stage 1: 1, stage 2: 0, stage 3: 0, stage 4: 0.

- `Emitter::emit_inline_helper` (`crates/nvs-codegen/src/emit.rs:2929`) runs first in the
  `HelperCall` arm. Truthiness is one compare. A widening branches to a fast block and a slow block
  that calls `emit_helper`, then joins at a block parameter. A constant operand in range emits no
  call, and one out of range takes the plain helper call.
- `Emitter::emit_str_eq` and the class-word test in `emit_class_test` end in a merge block, so the
  `ClassTest` arm returns `self.current_block()`.
- `a_known_scalar_test_and_widening_call_no_helper` passed by name. A scratch script checked the
  results at run time: the 2^53 bounds, `-0.0`, equal and unequal strings, a subclass, and both
  error messages. Nothing else ran.
- Tests that may go red at the gate: none expected. A test that greps the CLIF for a call to
  `nvs_str_eq`, `nvs_object_is_class` or a truthiness helper next to a fixed block layout is
  the likely one, such as in `crates/nvs-codegen/tests/refcount_calls.rs` or `probes.rs`.
- The sweep's `env-secret-proofs` is stale state in looper's service. `looper check --name` finds
  no such check, and nothing in the tree names it.

### Next

Stage 2, in one session. It follows its `**Split:**` line first.

- [ ] `crates/nvs-codegen/src/emit.rs:4693`: `emit_refcount`'s release branch, for strings and
  bytes first (`:4710`).
- [ ] `crates/nvs-codegen/src/emit.rs:4711`: the same for objects and arrays, only if the `Explore`
  list shows that a decrement does no more for them.
- [ ] `crates/nvs-runtime/src/budget.rs:1280`: `count_refcount_call` counts an inline release.
- [ ] `crates/nvs-codegen/src/lib.rs:3820`: the test `a_release_that_frees_nothing_calls_no_helper`,
  run once by name with the balance tests.

### Backlog

- Declined before the run: an optimising tier, inline caches, a layout pass, a new calling
  convention.
- Declined before the run: sharing one debug-flags load per block. It would delay a probe turned on
  in a running request.
