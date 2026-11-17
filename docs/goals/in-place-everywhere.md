---
looper: 1
title: "every statement that replaces a value with one made from it changes it in place"
summary: "Reached when one compiler pass decides every move and every lend, every Core member whose result keeps its subject's kind owns that subject, every replacement form in the grammar has a case that runs the same with and without in-place changes, and the fuzzer, Miri and the debug lend check find nothing."
context: [docs/rules/core-api/nothing-mutates.md, docs/rules/types/arrays.md, crates/nvs-ir/src/lib.rs, crates/nvs-ir/src/ir.rs]
stages:
  - id: 1
    title: the rule, the off switch and the matrix of every replacement form
    status: open
    context: [crates/nvs-cli/src/main.rs, tools/nv/cmd/suite.ts, tests/conformance/core/arr-append-derives-the-next-free-key-from-the-entries-it-copies.nvst]
  - id: 2
    title: a Core member owns its subject whenever its signature says so, and every such member does
    status: open
    context: [crates/nvs-runtime/src/abi.rs, crates/nvs-stdlib/src/lib.rs, crates/nvs-stdlib/src/registry.rs, crates/nvs-stdlib/src/arr.rs]
  - id: 3
    title: one ownership pass moves every value at its last use, on every path
    status: open
    context: [crates/nvs-ir/src/lower/call.rs, crates/nvs-ir/src/lower/stmt.rs, crates/nvs-ir/src/lower/mod.rs, crates/nvs-ir/src/lower/tests.rs]
  - id: 4
    title: the same pass lends every slot a statement replaces, and a debug build traps a wrong lend
    status: open
    context: [crates/nvs-ir/src/lower/stmt.rs, crates/nvs-ir/src/lower/tests.rs, crates/nvs-codegen/src/emit.rs]
  - id: 5
    title: a Novis function takes a lent or moved value whenever the compiler proves it safe
    status: open
    context: [crates/nvs-ir/src/lower/tests.rs, crates/nvs-ir/src/lower/call.rs]
  - id: 6
    title: concatenation and interpolation append in place on every target
    status: open
    context: [crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/lower/stmt.rs, crates/nvs-ir/src/lower/tests.rs]
  - id: 7
    title: every copy the compiler keeps is reported where it is written, with its reason and its fix
    status: open
    context: [crates/nvs-diagnostics/src/code/warnings.rs, crates/nvs-codegen/src/lib.rs, crates/nvs-cli/src/cache.rs, crates/nvs-lsp/src/recorded.rs, crates/nvs-cli/src/debug/index.rs]
  - id: 8
    title: the fuzzer, Miri and an attack hold the guarantee
    status: open
    context: [crates/nvs-fuzz-checks/src/lib.rs, crates/nvs-runtime/src/array.rs, crates/nvs-runtime/src/string.rs]
  - id: 9
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: matrix
    stage: 1
    name: "every replacement form the grammar allows has a conformance case"
    run: [cargo, test, --lib, --, every_replacement_form_has_a_case]
    timeout: 1800
    tests:
      - every_replacement_form_has_a_case
  - id: owning-signatures
    stage: 2
    name: "every Core member whose signature makes it own its subject has an owning helper, and no other member does"
    run: [cargo, test, --test, in_place, --, every_member_owns_exactly_what_its_signature_says, an_owning_helper_leaves_a_refused_subject_with_its_caller]
    timeout: 1800
    tests:
      - every_member_owns_exactly_what_its_signature_says
      - an_owning_helper_leaves_a_refused_subject_with_its_caller
  - id: moves
    stage: 3
    name: "a value is moved at its last use on every path, kept for a catch that reads it, moved inside a generator, and retained where any path reads it later"
    run: [cargo, test, --lib, --, a_value_is_moved_at_its_last_use, a_value_read_after_on_one_path_keeps_its_retain, a_value_a_catch_reads_is_kept_on_the_error_edge, a_generator_moves_like_a_function, the_lowering_no_longer_scans_text_for_moves]
    timeout: 1800
    tests:
      - a_value_is_moved_at_its_last_use
      - a_value_read_after_on_one_path_keeps_its_retain
      - a_value_a_catch_reads_is_kept_on_the_error_edge
      - a_generator_moves_like_a_function
      - the_lowering_no_longer_scans_text_for_moves
  - id: lends
    stage: 4
    name: "a plain property, a static property, an element and a chain of them are lent, and a hook, a guard, code between the read and the store and a callback keep the retain"
    run: [cargo, test, --lib, --, a_plain_property_is_lent, a_static_property_is_lent, an_element_is_lent_through_its_containers, a_property_chain_is_lent, a_hooked_or_guarded_slot_keeps_the_retain, code_between_the_read_and_the_store_keeps_the_retain, a_callback_that_reads_the_slot_sees_the_old_value, a_callback_that_writes_the_slot_keeps_the_retain, a_dead_binding_is_released_at_its_last_use]
    timeout: 1800
    tests:
      - a_plain_property_is_lent
      - a_static_property_is_lent
      - an_element_is_lent_through_its_containers
      - a_property_chain_is_lent
      - a_hooked_or_guarded_slot_keeps_the_retain
      - code_between_the_read_and_the_store_keeps_the_retain
      - a_callback_that_reads_the_slot_sees_the_old_value
      - a_callback_that_writes_the_slot_keeps_the_retain
      - a_dead_binding_is_released_at_its_last_use
  - id: lend-trap
    stage: 4
    name: "a debug build traps a read of a lent slot, naming the statement"
    run: [cargo, test, --test, in_place, --, a_debug_build_traps_a_read_of_a_lent_slot]
    timeout: 1800
    tests:
      - a_debug_build_traps_a_read_of_a_lent_slot
  - id: user-callees
    stage: 5
    name: "a known Novis callee changes a lent slot or a local inside a try in place, copies only at a change after which a failure or a read of the slot is reachable, survives its own write to the slot, never frees a lent value on an error exit, and a callee that can be overridden or is a callable keeps the retain"
    run: [cargo, test, --lib, --, a_callee_that_cannot_fail_after_its_change_changes_the_slot_in_place, a_callee_that_can_fail_after_its_change_copies_at_that_change_only, a_local_inside_a_try_is_lent_to_a_callee_that_cannot_fail_after_its_change, a_lent_parameter_is_not_freed_on_an_error_exit, a_callee_that_reads_the_slot_before_its_change_changes_it_in_place, a_callee_that_reads_the_slot_after_its_change_copies_at_that_change_only, a_callee_that_writes_the_slot_keeps_both_values_alive, an_overridable_or_dynamic_callee_keeps_the_retain, recursive_callees_reach_one_summary, a_fatal_is_not_a_failure_the_summary_counts]
    timeout: 1800
    tests:
      - a_callee_that_cannot_fail_after_its_change_changes_the_slot_in_place
      - a_callee_that_can_fail_after_its_change_copies_at_that_change_only
      - a_local_inside_a_try_is_lent_to_a_callee_that_cannot_fail_after_its_change
      - a_lent_parameter_is_not_freed_on_an_error_exit
      - a_callee_that_reads_the_slot_before_its_change_changes_it_in_place
      - a_callee_that_reads_the_slot_after_its_change_copies_at_that_change_only
      - a_callee_that_writes_the_slot_keeps_both_values_alive
      - an_overridable_or_dynamic_callee_keeps_the_retain
      - recursive_callees_reach_one_summary
      - a_fatal_is_not_a_failure_the_summary_counts
  - id: concat
    stage: 6
    name: "a concatenation or an interpolation that opens with its own target appends in place on a local, a property, a static property and an element"
    run: [cargo, test, --lib, --, every_concatenation_target_appends_in_place, an_interpolation_that_opens_with_its_target_appends_in_place]
    timeout: 1800
    tests:
      - every_concatenation_target_appends_in_place
      - an_interpolation_that_opens_with_its_target_appends_in_place
  - id: copy-report
    stage: 7
    name: "every copy the pass keeps is in the compiled program's table with its reason and fix, a warning is raised exactly where an easy fix removes a copy, the table survives the compile cache, and a recorded request counts its copies by site"
    run: [cargo, test, --lib, --, every_copy_the_pass_keeps_is_in_the_table, a_warning_is_raised_exactly_where_an_easy_fix_removes_a_copy, the_copy_table_survives_the_compile_cache, a_recorded_request_counts_its_copies_by_site]
    timeout: 1800
    tests:
      - every_copy_the_pass_keeps_is_in_the_table
      - a_warning_is_raised_exactly_where_an_easy_fix_removes_a_copy
      - the_copy_table_survives_the_compile_cache
      - a_recorded_request_counts_its_copies_by_site
  - id: copy-report-editor
    stage: 7
    name: "the editor warns at a read after the change and its fix moves the read, and a recorded copy shows its count and its help on the line"
    run: [bun, nv, suite, lsp-test, tests/lsp/, --case, tests/lsp/diagnostics/a-read-after-the-change-warns-and-its-fix-moves-it.lspt, --case, tests/lsp/diagnostics/a-failure-after-the-change-warns-and-its-fix-moves-it.lspt, --case, tests/lsp/lens/a-recorded-copy-shows-its-count-and-its-help.lspt]
    timeout: 1800
    tests:
      - tests/lsp/diagnostics/a-read-after-the-change-warns-and-its-fix-moves-it.lspt
      - tests/lsp/diagnostics/a-failure-after-the-change-warns-and-its-fix-moves-it.lspt
      - tests/lsp/lens/a-recorded-copy-shows-its-count-and-its-help.lspt
  - id: differential
    stage: 8
    name: "every conformance case prints the same with in-place changes switched off"
    run: [bun, nv, suite, tests/conformance/, --no-in-place-too]
    timeout: 3600
  - id: fuzz
    stage: 8
    name: "the replacement fuzzer finds no program that prints differently with in-place changes switched off"
    run: [cargo, test, --lib, --, replacement_programs_print_the_same_without_in_place]
    timeout: 1800
    tests:
      - replacement_programs_print_the_same_without_in_place
  - id: attack
    stage: 8
    name: "a program that cuts an array and a string it read from the configuration changes only its own copy, and the next read of the configuration is unchanged"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/in-place/a-value-from-the-configuration-is-cut-only-in-its-own-copy.nvst]
    timeout: 1800
    tests:
      - tests/conformance/in-place/a-value-from-the-configuration-is-cut-only-in-its-own-copy.nvst
---

# every statement that replaces a value with one made from it changes it in place

## Why

`$x = Core\Arr::slice($x, 1)`, `self::$p = Core\Str::after(self::$p, "\n")`, `$this->t = "{$this->t}x"`
and `$rows[$i] = Core\Arr::append($rows[$i], $v)` all mean "replace this value with one made from it".
The person's requirement: **every** such statement keeps what is unchanged where it is and writes only
what changes, whatever the target, the call, the operator or the surrounding statement. The compiler
handles every case. Nobody has to know which way of writing it is fast, and every case is tested.

Today it holds only by accident of syntax. Measured on 2026-11-17, `.=` is linear and
`= self::$p .` is quadratic. `$s . 'x'` is linear and `"{$s}x"` is quadratic. A local passed to a
Novis function moves, but the same local passed to a `Core` member is copied. The causes:

- **Moves are found by scanning source text** for two statement shapes (`$x = call(…, $x, …)` and
  `return call(…, $x, …)`, `crates/nvs-ir/src/lower/stmt.rs:1444`). Every other last use is missed, and
  nothing moves inside a `try` or a generator. `crates/nvs-ir/src/lib.rs:79` already says refcount
  insertion is "naive and syntactic, not a liveness analysis".
- **Every `Core` member borrows** (`crates/nvs-ir/src/lower/mod.rs:240`), so no member can reuse its
  subject.
- **The in-place append is a handful of syntactic special cases** (`lower_string_self_concat`,
  `lower_slot_append`, `lower_carrier_self_append`), so every form they do not list copies.

`rule:core-api/nothing-mutates` already promises in-place changes. This goal makes one mechanism keep
that promise everywhere. `arrays-in-place` and `strings-in-place` then make each member write only what
changes.

## Standing decisions

**The model is reference counting with reuse (my call).** One IR pass, `crates/nvs-ir/src/ownership.rs`,
runs after lowering. It decides every move and every lend from liveness and from what each instruction
may do. The lowering emits plain retains and releases and decides nothing. This is the technique Koka's
Perceus and Lean 4 use: a value whose count is 1 at the point of change is reused, and the pass makes
the count 1 wherever the program allows it. Every special case listed in § *Why* is folded into the
pass and deleted, so there is one place to read, test and fix.

**A move (my call).** A retain whose value is passed to an owning parameter is removed, together with
the binding's later release, when no path from the call reads the binding again. That includes loop
back-edges, a `catch`, a `finally` and a generator's resume. The rules:

- An owning `Core` member leaves its subject with the caller on an error (the protocol below). So a
  move into it is allowed inside a `try`, and the error edge keeps the binding live.
- A Novis callee owns its parameter on every path. A move into one is allowed where no error edge
  reads the binding. Where one does, as inside a `try` whose `catch` reads it, the value is lent to the
  callee's lent body under the rules for a Novis callee below.

**A release at the last use (my call).** A binding that is never read again is released right after
its last use, never at the end of its scope. So `$b = $a; $a = Core\Arr::slice($a, 1);` changes `$a` in
place when `$b` is not read afterwards. A copy is left only where two values are really used.

**Every copy the pass keeps is reported, and the report is compiled in (the person's call).** The pass
records each place it keeps a copy, with a reason and, where one exists, a fix. That table is part
of the compiled program, beside its code, and it is stored in the compile cache entry. So it is
available after the compile, without the compiler:

- **A warning, only where an easy fix removes the copy.** Two new codes, each taking the next free
  `W1xxx` number when it lands. The first is a read of the lent slot after the change, and its fix
  moves the read above the change. The second is a catchable failure after the change, and its fix
  moves the failing statement above the change. A fix is offered only where the moved code neither
  writes the slot nor depends on the change. Otherwise the warning explains the copy and offers no
  fix. Every other kept copy is not a warning: two values that are really both used, a callable, an
  overridden method, or a hooked property. It is a table row, shown only when a recorded request ran
  it. That keeps the warnings to what a user can act on in one edit.
- **The warnings come from the pass's decisions, never from a second analysis**, so the editor and the
  compiler cannot disagree. `nvs check` runs the lowering and the pass after the type check, so it
  reports them. The language server links `nvs-ir` and runs the same pass on the open document after
  its type diagnostics, gated as `rule:ide/diagnostics-are-phase-gated` gates the rest.
- **A recorded request counts its copies by site.** Each table row has an id in the compiled program.
  When a request is recorded for `nvs debug`, the copy path adds the entries or bytes it copied to that
  site's count. With recording off, it checks one flag on a path that already copies. The debug
  viewer lists the hottest copy sites with their help. The editor shows a lens on the line with the
  count from the newest recorded request and the help text, read from the debug index exactly as the
  query lens is (`crates/nvs-lsp/src/recorded.rs:1`).

**What the report spends:** one table per compiled program, with one row per kept copy, and one
flag check per copy at run time.

**A lend (my call).** `slot = M(slot, …)` lends the slot's reference to `M` with no retain, and the store
puts the result back with no release. A slot is a property, a static property, an array element, or a
chain of them whose containers are made sole first, as `write_back_array` does
(`crates/nvs-ir/src/lower/mod.rs:3864`). Every condition must hold, or the retain stays:

- Each slot on the chain is plain: no hook, no `PropertyGuard`, no observer and no `lateinit` step.
- `M` is an owning `Core` member, the concatenation of stage 6, or a Novis function under the rules
  for a Novis callee below. A member's callbacks run in `check`, before anything changes, so a callback
  that reads the slot sees the old value, which is correct. A callback whose effect summary writes the
  slot, or is "anything", keeps the retain.
- No instruction between the slot's read and its store can run user code or touch the slot. The pass
  reads that from each instruction's effects, so the order of the arguments in the source does not
  matter. A call to a Novis function, a callable, a hook or a property read with a hook is "may run
  user code".

**A Novis callee takes a lent value when the compiler proves it safe (the person's call: "the
compiler knows the code").** `$this->items = $this->addItem($this->items, $x)` must keep its meaning:
when the call fails, the property is unchanged. The compiler sees the whole program, so it proves
that per call instead of copying every time. It lends the slot, or a local inside a `try`, to the
callee when all of these hold:

- **The callee is known.** The call resolves to one body while compiling: a `private`, `static` or
  `final` method, a function, or a method that no class in the program overrides. A callable and an
  interface call with more than one body keep the retain.
- **The compiler knows what the callee does with the slot.** Each function has an effect summary: which
  properties it may read or write, by class and name, with "anything" for dynamic property access,
  reflection and callables. The summary covers everything the callee calls, solved to a fixed point
  over recursion. Inside the lent body it is placed per instruction, so the pass knows which reads and
  writes of the slot come after which change. A summary of "anything" keeps the retain.
- **The callee runs a lent body, and it copies only where the old value can still be observed.** The
  compiler builds a second body of the callee for that parameter, once per function and parameter
  that any call site lends. In it:
  - The parameter is never freed on an error exit, because the caller still holds it.
  - A change to the parameter separates first, which copies at that change only, when either of
    these is reachable after it: a catchable failure, or a read of the lent slot. Both would show the
    old value while the new one exists, so two values really are alive and one copy is the minimum.
    Every other change is made in place. A read of the slot before the first change sees the old value,
    which is correct, and costs nothing.
  - A write to the lent slot first retains the parameter, so the write cannot free the value the
    callee is using. The caller's store then releases what the slot holds unless it is still the lent
    value, which is one pointer comparison.
  - A failure is an instruction with an error edge in the IR. A `FATAL`, such as the memory ceiling
    (`rule:errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself`), ends the request
    and is not one. An append that is refused leaves its array unchanged, so it is not a failure after
    a change either.

So `addItem(array $items, $x) { $items[] = $x; return $items; }` appends in place with no copy. A
callee that checks `count($this->items)` and then appends also has no copy. A callee that appends and
then reads `$this->items` or calls something that can throw copies once, at the append, and only on
that call. The statement's meaning holds on every path, and a copy happens only where two values are
observable at the same time. **What it spends:** one more compiled body per lent
function and parameter, and the effect summaries while compiling, which are linear in the call graph
plus the fixed point.

**The owned-subject protocol (my call).** An owning helper is written as `check` and `apply`, and the
`nvs_helper!` form (`crates/nvs-runtime/src/abi.rs:1096`) makes the split structural:

- `check` reads every argument borrowed, runs every callback, reserves every allocation up to the
  memory ceiling, and returns a plan or an error. On an error the subject is still the caller's,
  unchanged.
- `apply` takes the plan and the owned subject (`NvsArray` or `NvsStr`) and has no error in its type.
  It runs no user code. In a debug build it asserts the subject's count before it changes it in place.
  On a shared subject it builds only what it keeps and releases its reference.

**Which members own their subject: the signature decides (the person's "all", my rule).** A member
owns its first parameter when that parameter is declared `array<…>`, `string` or `bytes` and its result
is the same kind, nullable or not. There is no list and no opt-out. A member that keeps nothing of its
subject (`Core\Crypto::seal`, `Core\Uri::encodeComponent`) still owns it, builds its result and
releases the subject. That costs nothing, and the next member to keep something needs no ABI change.
The ABI is declared once, by the helper's macro form, and each module's symbol table returns it
(`crates/nvs-stdlib/src/arr.rs:2270` `address`). The lowering reads it there, so a wrong row can only
miss a move, never free twice. `every_member_owns_exactly_what_its_signature_says` reads every registry
row and fails naming a member whose helper disagrees with its signature.

**Members that keep part of their subject write only what changes (my call).** `arrays-in-place` does
every `Core\Arr` member, and `strings-in-place` every `Core\Str` and `Core\Bytes` member. This goal
does the rest that keep part of their subject: `Core\Random::shuffle` and `sample`, the same pair on
`Core\Random\Seeded`, `Core\Path::withExtension`, `join`, `dirname` and `basename`, and
`Core\Regex::replace` and `replaceWith`. Every other owning member builds.

**The off switch (my call).** `nvs run`, `nvs test` and `nvs serve` take a hidden `--no-in-place` that
skips the ownership pass, so every owning call sees a shared subject and builds. It is for the
differential checks only and does not appear in `--help`. `bun nv suite --no-in-place-too` runs every
case twice and fails on any difference in output.

**Safety, beyond the tests (my call).** In a debug build, the pass writes a poison value into a lent
slot after its read and before its store. Any read of the slot in between traps with the statement's
span. A wrong effect decision then fails the debug suite at once, instead of corrupting memory in
release. The in-place operations of `crates/nvs-runtime/src/array.rs` and `string.rs` run under Miri in
CI. A value that a cache, the configuration or another request can reach always has a count above 1
(the immortal count, or the cache's own reference), so it is never changed in place. The attack in
stage 8 checks that.

**What it spends.** One retain and one release per call where the subject is still read after the call
(it is borrowed today). Nothing per request. Compile time grows by one liveness pass per function,
which the `compile` ladders under `benches/scaling/compiler/` keep linear.

**Lanes.** Stage 2 touches `crates/nvs-runtime/` and `crates/nvs-stdlib/`. Stages 3 to 6 touch
`crates/nvs-ir/` in order. Stage 2 can run beside stage 1. Stages 3 to 6 need stage 2's ABI. Stage 7 needs stages 3 to 6, and touches the diagnostics, the cache, the debug index and the language server.
`arrays-in-place` stage 2 and `strings-in-place` stages 2 and 3 touch only the runtime representation
and can run as side goals at any time.

## Stages

### 1. the rule, the off switch and the matrix of every replacement form

**Split:** once `every_replacement_form_has_a_case` prints the missing cells, writing subagents write
the cases in batches of about fifteen. Each batch is one target kind, so the subagents' files are
disjoint and they run side by side. The session runs the test once all batches end.

- [ ] `docs/rules/types/a-replaced-value-is-changed-in-place.md:1`: the rule, with every decision
  above, the lent body for a Novis callee and the cost lines. `rule:core-api/nothing-mutates` points to it
  and drops its own claim. One line in `docs/ground-rules.md`. `bun nv rules --render`, then
  `bun nv rules --check` is green.
- [ ] `--no-in-place` on `run`, `test` and `serve` in `crates/nvs-cli/src/main.rs`, hidden, reaching
  the IR's options. `tools/nv/cmd/suite.ts` gains `--no-in-place-too`.
- [ ] `crates/nvs-ir/src/lower/tests.rs`: `every_replacement_form_has_a_case` builds the matrix from
  the grammar and fails naming each missing case under `tests/conformance/in-place/`. The axes are:
  - Target: local, parameter, `inout` parameter, captured variable, property of `$this`, property of a
    local, static property, array element, element of a property, and a property chain.
  - Value: an owning `Core` member with no callback, one with a callback, a Novis function that
    cannot fail after its change, one that can, one that reads the slot before its change, one that
    reads it after, one that writes it, `.`, and interpolation.
  - Context: straight code, a loop, a `try` whose `catch` reads the target, a `finally`, and a
    generator.
  - Kind: array and string.
  Each case prints the target after the statement, and in a `try` after an error too. It is named
  `<target>-<value>-<context>-<kind>.nvst`.
- [ ] The cases, through the subagents.

### 2. a Core member owns its subject whenever its signature says so, and every such member does

- [ ] `crates/nvs-runtime/src/abi.rs:1096`: the `check` and `apply` form, generic over `NvsArray` and
  `NvsStr`, emitting its ABI. Its doc comment states the protocol.
- [ ] Every module's `address` returns the ABI beside the address (`crates/nvs-stdlib/src/lib.rs:18`).
- [ ] Every member the signature rule names is moved to the owning form. Its `check` is today's body
  over the borrowed subject, and its `apply` builds and releases. This stage changes no member's
  algorithm. Writing subagents take one stdlib module each, and the modules are disjoint files.
- [ ] `crates/nvs-stdlib/tests/in_place.rs:1`: the two tests the `owning-signatures` check names. Run
  once.

### 3. one ownership pass moves every value at its last use, on every path

- [ ] `crates/nvs-ir/src/ownership.rs:1`: liveness over the SSA form, including the error edges and a
  generator's resume. Every move rule in § *Standing decisions* is applied here.
- [ ] `crates/nvs-ir/src/lower/stmt.rs:1444` `last_use_move` and `pending_move`
  (`crates/nvs-ir/src/lower/mod.rs:2407`) are deleted, with their call sites at `stmt.rs:189`, `:1541`
  and `crates/nvs-ir/src/lower/call.rs:248`. The lowering emits plain retains. `crates/nvs-ir/src/lib.rs:139`
  becomes the pass's decision bullet.
- [ ] Owning `Core` parameters are `ArgOwnership::Transferred` at `call.rs:187`, read from the ABI.
- [ ] A binding that is never read again is released right after its last use.
- [ ] The five tests the `moves` check names. Run once.

### 4. the same pass lends every slot a statement replaces, and a debug build traps a wrong lend

- [ ] Each IR instruction states its effects: may run user code, and which slots it may read or
  write. The pass reads them, never the source.
- [ ] The lend for every slot kind in § *Standing decisions*. `lower_slot_append`
  (`crates/nvs-ir/src/lower/stmt.rs:623`) and `write_back_array` become uses of it.
- [ ] The debug poison, emitted only in a debug build, in `crates/nvs-codegen/src/emit.rs`.
- [ ] The tests the `lends` and `lend-trap` checks name. Run once.

### 5. a Novis function takes a lent or moved value whenever the compiler proves it safe

- [ ] `crates/nvs-ir/src/effects.rs:1`: each function's summary (properties it may read or write, by
  class and name, and "anything"), solved to a fixed point over the call graph. The resolver that
  names the one body of a call while compiling lives here too.
- [ ] The lent body: built once per function and parameter that a call site lends. Its error exits
  never free that parameter, and its changes to it separate first wherever a catchable failure is
  still reachable. `crates/nvs-ir/src/ownership.rs` builds it and points the call at it.
- [ ] `ownership.rs` lends a slot, and a local that a `catch` reads, to a Novis callee under the three
  conditions in § *Standing decisions*.
- [ ] The lent body's write to the lent slot retains the parameter first, and the caller's store
  releases the slot's value unless it is still the lent one.
- [ ] The ten tests the `user-callees` check names. Run once.

### 6. concatenation and interpolation append in place on every target

- [ ] `crates/nvs-ir/src/lower/expr.rs:3402` `concat_spine_opens_with` accepts an `ExprKind::Interpolated`
  whose first part is the target. A concatenation is one instruction that the pass lends its first
  operand to, for every target the pass can lend. `lower_string_self_concat` (`stmt.rs:1395`) and
  `lower_carrier_self_append` are folded into it.
- [ ] The two tests the `concat` check names. Run once.

### 7. every copy the compiler keeps is reported where it is written, with its reason and its fix

**Split:** after the table and the two codes land, one writing subagent writes the three `.lspt` cases,
copying the shape of an existing case under `tests/lsp/diagnostics/` and `tests/lsp/lens/`.

- [ ] `crates/nvs-ir/src/ownership.rs` records each kept copy: its span, the file's BLAKE3, its reason,
  and its fix as a text edit where one exists. `crates/nvs-codegen/src/lib.rs:382` `Unit` carries the
  table, and `crates/nvs-cli/src/cache.rs` stores and loads it with the entry.
- [ ] The two warning codes in `crates/nvs-diagnostics/src/code/warnings.rs` after the last one, each
  with its card in end-user prose. `nvs check` lowers and runs the pass, and prints them.
- [ ] `crates/nvs-lsp/Cargo.toml` links `nvs-ir`. The server runs the pass on the open document after
  its type diagnostics, publishes the warnings, and offers each fix as a code action.
- [ ] The copy path counts by site when a request is recorded. `crates/nvs-cli/src/debug/index.rs`
  stores the counts, the debug viewer lists the hottest sites with their help, and
  `crates/nvs-lsp/src/recorded.rs` shows the lens.
- [ ] The four tests and the three `.lspt` cases the `copy-report` checks name. Run once.

### 8. the fuzzer, Miri and an attack hold the guarantee

- [ ] `crates/nvs-fuzz-checks/src/in_place.rs:1`: a generator of small programs over the matrix's axes,
  with random sharing (`$b = $a`), errors and catches. Each program runs with and without the pass, and
  the outputs must match. `replacement_programs_print_the_same_without_in_place` runs a fixed seed
  set. The fuzz target runs the open-ended search.
- [ ] The in-place tests of `crates/nvs-runtime/src/array.rs` and `string.rs` are on CI's Miri list.
- [ ] `tests/conformance/in-place/a-value-from-the-configuration-is-cut-only-in-its-own-copy.nvst`:
  the program cuts an array and a string it read from `Core\Config`, which every request shares, and
  then reads the configuration again. Both reads print the original. Run once.
- [ ] `bun nv suite tests/conformance/ --no-in-place-too` once, green.

### 9. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Written on 2026-11-17 from a conversation with the person. Their calls: fix the cause, nothing for a
developer to remember, and **every** replacement in place, handled by the compiler and tested. They
accepted every decision below, and asked that a Novis callee get the in-place path whenever the
compiler can prove it safe, which is stage 5. Every other choice in § *Standing decisions* is the authoring session's call under the priority ordering,
and stands until the person picks another. Nothing is built yet.

### Next

Stage 1 from its first item, with stage 2 beside it.

### Backlog
