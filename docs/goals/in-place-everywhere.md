---
looper: 1
title: "every statement that replaces a value with one made from it changes it in place"
summary: "Reached when one compiler pass decides every move and every lend, every Core member whose result keeps its subject's kind owns that subject, every replacement form in the grammar has a case that runs the same with and without in-place changes, and the fuzzer, Miri and the debug lend check find nothing."
context: [docs/rules/core-api/nothing-mutates.md, docs/rules/types/arrays.md, crates/nvs-ir/src/lib.rs, crates/nvs-ir/src/ir.rs]
stages:
  - id: 1
    title: the rule, the off switch and the matrix of every replacement form
    status: done
    context: [crates/nvs-cli/src/main.rs, tools/nv/cmd/suite.ts, tests/conformance/core/arr-append-derives-the-next-free-key-from-the-entries-it-copies.nvst]
  - id: 2
    title: a Core member owns its subject whenever its signature says so, and every such member does
    status: done
    context: [crates/nvs-runtime/src/abi.rs, crates/nvs-stdlib/src/lib.rs, crates/nvs-stdlib/src/registry.rs, crates/nvs-stdlib/src/arr.rs]
  - id: 3
    title: one ownership pass moves every value at its last use, on every path
    status: done
    context: [crates/nvs-ir/src/lower/call.rs, crates/nvs-ir/src/lower/stmt.rs, crates/nvs-ir/src/lower/mod.rs, crates/nvs-ir/src/lower/tests.rs]
  - id: 4
    title: the same pass lends every slot a statement replaces, and a debug build traps a wrong lend
    status: done
    context: [crates/nvs-ir/src/lower/stmt.rs, crates/nvs-ir/src/lower/tests.rs, crates/nvs-codegen/src/emit.rs]
  - id: 5
    title: a Novis function takes a lent or moved value whenever the compiler proves it safe
    status: done
    context: [crates/nvs-ir/src/lower/tests.rs, crates/nvs-ir/src/lower/call.rs]
  - id: 6
    title: concatenation, interpolation and a literal that spreads its target append in place on every target
    status: done
    context: [crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/lower/stmt.rs, crates/nvs-ir/src/lower/tests.rs]
  - id: 7
    title: every copy the compiler keeps is reported where it is written, with its reason and its fix
    status: done
    context: [crates/nvs-diagnostics/src/code/warnings.rs, crates/nvs-codegen/src/lib.rs, crates/nvs-cli/src/cache.rs, crates/nvs-lsp/src/recorded.rs, crates/nvs-cli/src/debug/index.rs]
  - id: 8
    title: the fuzzer, Miri and an attack hold the guarantee
    status: done
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
  - id: own-entry-loop
    stage: 3
    name: "a by-value loop whose body writes only the entry it is on holds no reference of its own, and one whose body can change the array another way keeps it"
    run: [cargo, test, --lib, --, a_loop_that_writes_only_its_own_entry_holds_no_reference, a_loop_whose_body_can_change_the_array_another_way_keeps_its_reference]
    timeout: 1800
    tests:
      - a_loop_that_writes_only_its_own_entry_holds_no_reference
      - a_loop_whose_body_can_change_the_array_another_way_keeps_its_reference
  - id: generator-local
    stage: 3
    name: "an array local a generator changes after a yield is changed in place, and a generator that is dropped while parked frees each local once"
    run: [cargo, test, --lib, --, a_generator_local_changed_after_a_yield_is_changed_in_place, a_generator_dropped_while_parked_frees_each_local_once]
    timeout: 1800
    tests:
      - a_generator_local_changed_after_a_yield_is_changed_in_place
      - a_generator_dropped_while_parked_frees_each_local_once
  - id: carry
    stage: 5
    name: "a reduce callback that changes its carry changes it in place, and a callback that throws frees the carry once"
    run: [cargo, test, --test, in_place, --, a_reduce_callback_changes_its_carry_in_place, a_reduce_callback_that_throws_frees_the_carry_once]
    timeout: 1800
    tests:
      - a_reduce_callback_changes_its_carry_in_place
      - a_reduce_callback_that_throws_frees_the_carry_once
  - id: spread
    stage: 6
    name: "a literal that opens with a spread of its own target appends in place on a list, and still renumbers a map with number keys"
    run: [cargo, test, --lib, --, a_literal_that_spreads_its_target_appends_in_place_on_a_list, a_literal_that_spreads_a_map_with_number_keys_still_renumbers]
    timeout: 1800
    tests:
      - a_literal_that_spreads_its_target_appends_in_place_on_a_list
      - a_literal_that_spreads_a_map_with_number_keys_still_renumbers
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
    run: [cargo, test, --lib, --bin, nvs, --test, debug_ingest, --, every_copy_the_pass_keeps_is_in_the_table, a_warning_is_raised_exactly_where_an_easy_fix_removes_a_copy, the_copy_table_survives_the_compile_cache, a_recorded_request_counts_its_copies_by_site]
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

**Four more replacement forms, found by the review of 2026-11-17 (my calls).** Each was measured on
the release binary that day, and none was named above:

- **A literal that opens with a spread of its own target.** `$list = [...$list, $x]` built a list of
  10,000 entries in 667 ms and one of 20,000 in 3.6 s. It is the array form of `$s = $s . 'x'`. The
  literal's first spread takes its source as an owning operand, so the pass moves a local into it.
  At run time, a sole source that is a list is the literal itself, and the rest of
  the literal appends to it. A source that is shared, or that is not a list, builds as it does today,
  because a spread renumbers number keys and only a list keeps its keys under that. **What it
  spends:** one shape test for each such literal.
  **A slot is not lent to a spread (my call, 2026-11-18).** The literal's items after the spread
  are appends, and an append can fail at the memory ceiling. Its landing releases the literal, which
  is then the slot's own array, and the slot would name freed memory until the request's teardown
  frees it again. A lend needs nothing that can fail between the change and the store. So
  `$this->list = [...$this->list, $x]` copies, and `$this->list[] = $x` is the in-place form.
  **The spread is switched, as a `Core` call is (my call, 2026-11-18).** The lowering emits every
  `ArraySpread` borrowed. Where the pass moves a local into a literal's first spread into its empty
  array, it sets the instruction's `owned`, and codegen calls `nvs_array_spread_owned`, which takes
  the subject's reference on success only. So `--no-in-place` needs no case of its own.
- **A `Core\Arr::reduce` callback is handed its carry.** `reduce` whose callback returns
  `Core\Arr::append($carry, $x)` took 2.9 s for 20,000 entries and 16.1 s for 40,000. `reduce` keeps a
  reference to the carry while the callback runs (`crates/nvs-stdlib/src/arr.rs:4504`), and
  `call_callable` adds one more (`crates/nvs-runtime/src/callable.rs:175`), so the callback's change
  always copies. `reduce` instead gives the carry away: the callback owns it as a Novis callee owns a
  moved parameter, and its error exit frees it. A shared first carry separates once, at the first
  change. This is the one member whose callback receives a value the member itself made, so it is the
  one member that hands a value over.
- **A by-value loop that writes only the entry it is on holds no reference of its own.**
  `foreach ($a as $k => $v) { $a[$k] = f($v); }` copies the whole array once, because the loop keeps a
  reference to the array it walks (`crates/nvs-ir/src/lower/control.rs:955`). On 2M entries it took
  164 ms, against 83 ms for the `inout` loop. The pass drops that reference when every write to the
  array in the body is an element
  write at the loop's own key, the key is not assigned, and no instruction in the body may write the
  slot another way by its effects. Nothing is added or removed, so the walk is the same. Any other
  body keeps the reference.
  **The loop walks the variable's current array (my call, 2026-11-18).** With no reference of its
  own, the loop cannot keep walking the array it started on: when another holder shares it, the
  first write copies, and that holder may then free the original under the walk. So the pass also
  points the walk at the variable's header phi. In SSA the condition is that every back edge's value
  comes from the header's value through `ArraySet`s at the key binding's value and joins of those.
  The walk reads each entry before the body runs, so it sees the original values. For positions to
  hold across a copy, `Hashed::separate` keeps the holes (`crates/nvs-runtime/src/array.rs`). **What
  it spends:** a copy of a hashed array carries its holes, at most one per live entry.
- **A generator's local is its only reference after a resume.** A generator that appends to an array
  local and then yields took 184 ms for 10,000 values and 729 ms for 20,000. Every `yield` parks each
  local in the generator's state object, and the resume reads it back with a reference of its own
  (`reload_field`, `crates/nvs-ir/src/lower/generator.rs:21`), so the state object and the local both
  hold the array and the next change copies it. The resume instead takes the value out of the state
  object and leaves the field empty until the next `yield` parks it again. The field is read by
  nothing else while the body runs.

**Every copy the pass keeps is reported, and the report is compiled in (the person's call).** The pass
records each place it keeps a copy, with a reason and, where one exists, a fix. That table is part
of the compiled program, beside its code, and it is stored in the compile cache entry. So it is
available after the compile, without the compiler:

**The table lives on the IR, and the cache payload carries nothing more (my call, 2026-11-18).**
A warm hit still lowers the program and runs the pass, and skips only code generation
(`rule:packaging/a-warm-hit-skips-codegen-not-the-front-end`), as the query site table relies on.
So `Program::copies` is rebuilt on every run, `Unit::copies` takes it on both paths, and a second
copy in the payload would be a second home for one fact. A row holds spans; `nvs_ir::copies::table`
adds the file, the line, the BLAKE3 and the edit from the source map when asked.
**What a row is:** a place a copy may happen. A refused move of an `array`, `string` or `bytes`
value is `StillUsed`, or `SlotKept` for a property's value, or `CalleeUnknown` at a virtual call.
A lent body's separated change is `ReadAfterChange` or `FailureAfterChange`, with a fix where the
first offending statement after it, in the same block, can move above the change's statement:
it reads no value made from the change on, makes no value read later, and its effects meet
neither the lent slots nor the statements it passes. Only those two reasons with a fix warn.

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
  **How it is counted (my call, 2026-11-18).** The runtime's copy paths do not take a site: the
  pass puts `InstKind::CopyCount { site, before, after }` right after each row's call or change,
  and codegen calls `nvs_copy_count`, which cannot fail. It does nothing unless the context counts
  copies, which is armed when the context has a `CopySites` table and `[debug] store` is in force.
  It counts a copy when `after` is another value than `before`, and adds `before`'s entries or
  bytes. The request's context writes one `copies` spool record when its isolate finishes
  (`nvs-host` `finish`), and the index keeps one row per site (`copies`, schema 8). Threading a
  site id through every array, string and helper copy path was declined: many runtime ABIs would
  change, for the same number. **What it spends:** one call per run of a kept-copy site, which
  tests one field and returns, and a slice of two counters per row on a recorded request. A
  counted row whose callee builds a new value of the same kind counts it as a copy; a task's own
  copies (`Ctx::child`) are not counted.

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
  - **A callee that may write the lent slot is not lent (my call, 2026-11-18).** Its call keeps its
    retain, as a callback that writes the slot does, so both values stay alive with no new rule. The
    retain in the callee and the caller's pointer comparison before its release are declined: they
    put a second protocol on the store for a callee that both changes and replaces one value, which
    is rare. **What it spends:** such a call copies at its change, as it does today.
  - **The lent body covers the common shapes, and any other callee keeps the retain (my call,
    2026-11-18).** The parameter may be released, changed, returned, read by a borrowed `Core` or
    helper call, or read by an element read. A retain of it, a phi or a call that takes it gives no
    lent body. Neither does a release from which both a failure and a return are reachable, or a
    separated change whose return is reachable another way, or whose landing has another way in.
    The separate is a retain before the change and a release before each return it reaches; the
    change's own landing keeps its release. `crates/nvs-ir/src/ownership/lent.rs` is the home.
  - A failure is an instruction with an error edge in the IR. A `FATAL`, such as the memory ceiling
    (`rule:errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself`), ends the request
    and is not one. An append that is refused leaves its array unchanged, so it is not a failure after
    a change either.
  - **Every call to a Novis function is a failure (my call, 2026-11-18).** Each function may throw a
    catchable `RecursionError` at its entry (`rule:errors/stack-depth`), so the effect summary has no
    "may fail" flag: it would be true for every function. A lent body copies at a change after which
    any such call is reachable. **What it spends:** a callee that changes its parameter and then calls
    another Novis function copies once, on that call only.

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
row and fails naming a member whose helper disagrees with its signature. A nullable first parameter
does not own: an owning entry takes a subject of its declared kind, and `null` is none of the three.

**Two entries per owning member (my call, 2026-11-18).** The owning form emits the member's own
symbol, which borrows as every helper does today, and `<symbol>_owned`, which takes the subject's
reference (`crates/nvs-runtime/src/owned.rs`). The borrowed entry hands `apply` a handle of its own,
so its count is at least 2 and nothing changes in place. Each module's `address` returns an
`nvs_runtime::Entry` with both, and `nvs_stdlib::owning_entry` gives the lowering the owning symbol and
the subject's slot (1 for an instance member). Stage 3's pass calls the owning symbol where it moves
or lends, and keeps the borrowed one with no retain everywhere else. This replaces "the lowering
retains, the pass removes the retain": no existing caller changes in stage 2, a call the pass does not
move costs nothing extra, and a symbol the table does not name can only be called borrowed. **What it
spends:** each owning member is compiled twice, and the borrowed entry takes one retain and one
release per call for `apply`'s handle.

**A `Core` move is a symbol switch, not a transferred argument (my call, 2026-11-18).** The lowering
records the first declared parameter's index on `InstKind::CoreCall::subject`. The pass, seeing the
subject released once on every path from the call's normal exit, switches the call to the owning
entry and removes those releases. The error edge keeps its releases, because an owning entry leaves
the subject with the caller when it fails. Marking the argument `ArgOwnership::Transferred` in the
lowering would stage it as a transferred temporary, which is the lowering deciding a move. `nvs-ir`
depends on `nvs-stdlib` for `owning_entry`, which `nvs-types` already linked, so no cycle and no new
code is built.

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

**Where the poison is written (my call, 2026-11-18).** "Between the read and the store" cannot mean
the call too. A callback the lend allows, and a lent Novis body before its first change, may read the
slot and must see the old value, so a poison across the call would trap a correct program. The poison
covers the two windows where any read is a wrong effect decision:

- **Before the call:** from just after the slot's read to just before the call, the slot's tag byte
  holds a poison tag, and the codegen restores the tag right before the call. Only where nothing in
  that window has an error edge, because a failure there would leave the slot poisoned for a `catch`.
- **After the call:** from the call's normal exit to the store, the slot may point at memory the call
  freed, so the poison tag is written there too, and the store overwrites it. Lend::of already
  refuses a failing instruction in that window.

The pass records it as one instruction, `InstKind::LendPoison` with the slot and on or off, emitted
only where it lends; the emitter skips it in a release build. In a debug build, `FieldGet` and
`StaticGet` compare the tag byte with the poison tag and call a runtime trap that fails with a fatal
naming the read's statement. A runtime helper that reads the slot by name sees an unknown tag and
panics in debug, which still fails the suite at once. An element's poison waits for the element's
entry to have a tag of its own to write. The trap test forces a wrong lend through a debug-only
`NOVIS_LEND_IGNORE_EFFECTS`, read by the pass under `cfg(debug_assertions)`. **What it spends:**
nothing in release; in debug one tag compare per slot read.

**How the poison is built (my calls, 2026-11-18).** The poison is the byte `Tag::LENT_POISON`
(`0xEE`, `crates/nvs-runtime/src/value.rs`), not a `Tag` variant: no value ever has it, so none of
the 40-odd `match`es on `Tag` names it, `Tag::from_byte` gives `None`, and a release skips it. The
trap is `nvs_lend_trap` (`crates/nvs-runtime/src/owned.rs`); the cold block returns its `FATAL`
from the frame without the frame's releases, which leaks only after a compiler bug was found. In a
debug build the lowering's `emit` gives every `FieldGet` and `StaticGet` its statement as
`raise_site`, which the trap names. With `NOVIS_LEND_IGNORE_EFFECTS` the poison before the call is
written even where that window can fail, because the forced lend's read is itself a call.

**What it spends.** One retain and one release per call where the subject is still read after the call
(it is borrowed today). Nothing per request. Compile time grows by one liveness pass per function,
which the `compile` ladders under `benches/scaling/compiler/` keep linear.

**Lanes.** Stage 2 touches `crates/nvs-runtime/` and `crates/nvs-stdlib/`. Stages 3 to 6 touch
`crates/nvs-ir/` in order. Stage 2 can run beside stage 1. Stages 3 to 6 need stage 2's ABI. Stage 7 needs stages 3 to 6, and touches the diagnostics, the cache, the debug index and the language server.
`arrays-in-place` stage 2 and `strings-in-place` stages 2 and 3 touch only the runtime representation
and can run as side goals at any time.

**How the off switch reaches the IR (my call, 2026-11-18).** `nvs_ir::lower::without_in_place` is a
scoped thread-local, as `with_debug_slots` is, so a test lowers both ways in one process with no
global state. The `Program` records it (`Program::in_place`), and the artifact cache skips such a
program as it skips a debug one. The CLI's process switch is `crates/nvs-cli/src/in_place.rs`, and
`nvs test` hands it to each case's process as `NOVIS_NO_IN_PLACE`, as `--probes` does. Today it turns
off `last_use_move` and the `=` self-concatenation; stage 3 points it at `ownership.rs` instead, and
the pass is skipped whole. `--no-in-place-too` runs the trees a second time outside the selection
store, because the store's verdicts are the shipped binary's.

**The matrix's ruled-out cells (my call).** `.` and interpolation make a `string`, a spread an
`array`. The three `fn-` values that read or write the slot need a slot a callee can reach: a property,
a static property, an element of a property or a chain. A generator takes no `inout` parameter. A
hole in a string opens with `{$`, so an interpolation cannot open with a static property:
`"{Store::$text}x"` is E0301 (found on 2026-11-18). That leaves 684 cells.

**The case brief (my call).** Every batch gets the same brief, written to a scratch file it reads.
The exemplar is `tests/conformance/in-place/local-fn-fails-try-array.nvst`. The value slugs: `core`
(an owning member with no callback), `core-callback`, `fn-safe` (a `public static function` on a
`final class` that changes its parameter and cannot throw after it), `fn-fails` (throws `LogicError`
after its change when an argument says so; outside `try` it is called so it does not throw),
`fn-reads-before`, `fn-reads-after`, `fn-writes`, `concat` (`$t = $t . "x"`), `interpolation`
(`$t = "{$t}x"`), `spread` (`$t = [...$t, 9]`). The contexts: `straight`; `loop` (at least three
runs, print after); `try` (the `catch` prints the target, an error after the statement, and a print
after the `try`); `finally` (the `finally` prints it); `generator` (the statement between two
`yield`s, the target yielded after it, and the consumer keeps the first yielded value and prints it
after the change, so a change made in place on a value the consumer still has shows). An array
literal written in a `foreach` header has `mixed` items, so a loop uses `as var $x` or a typed local.
Arrays are `array<int>` joined with `,` to print. Each case
is 15 to 35 lines, cites `rule:types/a-replaced-value-is-changed-in-place` in its `--TEST--` line,
passes under `nvs test` and `nvs test --no-in-place` by path, and runs no `cargo`. A block-body
anonymous function is `callable $f = fn(): string => { …; return …; };`, and a plain `callable` call
gives `mixed`, so a generator that yields one declares `callable(): string $f`. A property default
may be only `[]` for an array (E0472), so a class sets `[1, 2]` in its `constructor`, which is the
constructor's name.

## Stages

### 1. the rule, the off switch and the matrix of every replacement form

**Split:** once `every_replacement_form_has_a_case` prints the missing cells, writing subagents write
the cases in batches of about fifteen. Each batch is one target kind, so the subagents' files are
disjoint and they run side by side. The session runs the test once all batches end.

- [x] `docs/rules/types/a-replaced-value-is-changed-in-place.md:1`: the rule, with every decision
  above, the lent body for a Novis callee and the cost lines. `rule:core-api/nothing-mutates` points to it
  and drops its own claim. One line in `docs/ground-rules.md`. `bun nv rules --render`, then
  `bun nv rules --check` is green.
- [x] `--no-in-place` on `run`, `test` and `serve` in `crates/nvs-cli/src/main.rs`, hidden, reaching
  the IR's options. `tools/nv/cmd/suite.ts` gains `--no-in-place-too`.
- [x] `crates/nvs-ir/src/lower/tests.rs`: `every_replacement_form_has_a_case` builds the matrix from
  the grammar and fails naming each missing case under `tests/conformance/in-place/`. The axes are:
  - Target: local, parameter, `inout` parameter, captured variable, property of `$this`, property of a
    local, static property, array element, element of a property, and a property chain.
  - Value: an owning `Core` member with no callback, one with a callback, a Novis function that
    cannot fail after its change, one that can, one that reads the slot before its change, one that
    reads it after, one that writes it, `.`, interpolation, and an array literal that opens with a
    spread of the target.
  - Context: straight code, a loop, a `try` whose `catch` reads the target, a `finally`, and a
    generator.
  - Kind: array and string.
  Each case prints the target after the statement, and in a `try` after an error too. It is named
  `<target>-<value>-<context>-<kind>.nvst`.
- [x] The cases, through the subagents: 684 cells. Each batch's brief is § *Standing decisions*'
  **The case brief**, written to a scratch file the subagents read.
  - [x] `local` and `parameter` (110 cases)
  - [x] `inout` and `captured` (44 and 55; no `inout` in a generator)
  - [x] `this-property` and `local-property` (85 each)
  - [x] `static-property` and `element` (80 and 55; no interpolation of a static property)
  - [x] `property-element` and `property-chain` (85 each), then the test is green

### 2. a Core member owns its subject whenever its signature says so, and every such member does

- [x] `crates/nvs-runtime/src/abi.rs:1096`: the `check` and `apply` form, generic over `NvsArray` and
  `NvsStr`, emitting its ABI. Its doc comment states the protocol.
- [x] Every module's `address` returns the ABI beside the address (`crates/nvs-stdlib/src/lib.rs:18`).
- [x] Every member the signature rule names is moved to the owning form. Its `check` is today's body
  over the borrowed subject, and its `apply` builds and releases. This stage changes no member's
  algorithm. Writing subagents take one stdlib module each, and the modules are disjoint files.
- [x] `crates/nvs-stdlib/tests/in_place.rs:1`: the two tests the `owning-signatures` check names. Run
  once.

### 3. one ownership pass moves every value at its last use, on every path

- [x] `crates/nvs-ir/src/ownership.rs:1`: liveness over the SSA form, including the error edges and a
  generator's resume. Every move rule in § *Standing decisions* is applied here.
  - [x] `InstKind::for_each_operand` and `Terminator::for_each_operand` (`crates/nvs-ir/src/ir.rs`, end)
  - [x] the forward walk: one release on every path, phis read per edge, error edges followed
  - [x] the avoid walk: no removed release is reachable from the entry without the call
  - [x] run at the end of `lower_program_as`, skipped whole under `without_in_place`
- [x] `crates/nvs-ir/src/lower/stmt.rs:1444` `last_use_move` and `pending_move`
  (`crates/nvs-ir/src/lower/mod.rs:2407`) are deleted, with their call sites at `stmt.rs:189`, `:1541`
  and `crates/nvs-ir/src/lower/call.rs:248`. The lowering emits plain retains. `crates/nvs-ir/src/lib.rs:139`
  becomes the pass's decision bullet.
- [x] A `Core` call whose subject the pass moves calls `nvs_stdlib::owning_entry`'s symbol with the
  subject `ArgOwnership::Transferred` (`call.rs:187`), only where the call's own ABI slot of the
  first parameter equals the entry's `slot`. Every other call keeps the borrowed symbol and no retain.
  - [x] `InstKind::CoreCall::subject` set at the two written-member sites in `lower/expr.rs`
  - [x] the pass switches `symbol` to the entry's and removes the normal path's releases
- [x] A binding that is never read again is released right after its last use.
  - [x] `Walk::excess_releases` in `ownership.rs`: each path's first release moves before the call
  - [x] the moved case in `a_value_is_moved_at_its_last_use`, green
- [x] The loop's own reference (`crates/nvs-ir/src/lower/control.rs:955`) is the pass's decision: it is
  dropped for a by-value loop whose body writes only the entry it is on, under the conditions in
  § *Standing decisions*.
  - [x] `Function::own_entry_loops`, filled by `lower_foreach`, in a function, an anonymous function and a generator
  - [x] `drop_loop_reference`: the walk reads the current array, and the retain and first releases go
  - [x] a hashed array's copy keeps its holes (`crates/nvs-runtime/src/array.rs` `Hashed::separate`)
  - [x] `tests/conformance/in-place/a-loop-that-writes-its-own-entry-visits-every-entry.nvst`, both ways
- [x] `crates/nvs-ir/src/lower/generator.rs:21` `reload_field` takes the parked value and leaves the
  field empty, with no retain. A generator that is dropped while parked still frees what its fields
  hold, and one dropped while its body runs frees nothing twice.
  - [x] `InstKind::FieldTake` in `ir.rs`, with its operands and its print (stage 4 adds every effects row)
  - [x] its codegen `emit_field_take` beside `emit_field_get`: read, then store `null`
  - [x] `reload_field` emits it with no retain, and the two `generator-local` tests
- [x] The five tests the `moves` check names, the two the `own-entry-loop` check names and the two the
  `generator-local` check names. Run once.
  - [x] the five `moves` tests, green (`crates/nvs-ir/src/lower/tests.rs` `lower_moves`)
  - [x] the two `own-entry-loop` tests
  - [x] the two `generator-local` tests

### 4. the same pass lends every slot a statement replaces, and a debug build traps a wrong lend

- [x] Each IR instruction states its effects: may run user code, and which slots it may read or
  write. The pass reads them, never the source.
- [x] The lend for every slot kind in § *Standing decisions*. `lower_slot_append`
  (`crates/nvs-ir/src/lower/stmt.rs:623`) and `write_back_array` become uses of it.
  - [x] A plain property, a static property and a property chain are lent to an owning `Core`
    member, and a callback is judged by its body's write summary.
  - [x] An element, through containers made sole before the element's read.
  - [x] `lower_slot_append` emits a plain load, retain and store, and the lend removes the retain.
  - [x] `write_back_array`'s decision is the lend's.
    - [x] A one-level write or append into a plain `array` property or static property
      (`Lowering::lendable_root`).
    - [x] A nested write (`ArrayWriteThrough`) and a nested `.=` (`ArrayStrAppendThrough`).
    - [x] An `unset`, one level and nested.
    - [x] A guarded root, a union receiver and a `lateinit` root, or a recorded reason each keeps
      the lowering's store.
- [x] The debug poison, emitted only in a debug build, in `crates/nvs-codegen/src/emit.rs`.
- [x] The tests the `lends` and `lend-trap` checks name. Run once.

### 5. a Novis function takes a lent or moved value whenever the compiler proves it safe

- [x] `crates/nvs-ir/src/effects.rs:1`: each function's summary (properties it may read or write, by
  class and name, and "anything"), solved to a fixed point over the call graph. The resolver that
  names the one body of a call while compiling lives here too.
- [x] The lent body: built once per function and parameter that a call site lends. Its error exits
  never free that parameter, and its changes to it separate first wherever a catchable failure is
  still reachable. `crates/nvs-ir/src/ownership/lent.rs` builds it and `ownership.rs` points the
  call at it.
  - [x] `lent::body` classifies each use of the parameter and declines an uncovered shape.
  - [x] Releases on failure-only paths go; a separated change retains first and releases before its
    returns.
  - [x] `lend_to_callees` collects the sites, builds each body once and rewrites the calls.
- [x] `ownership.rs` lends a slot, and a local that a `catch` reads, to a Novis callee under the three
  conditions in § *Standing decisions*.
  - [x] A property or a static property, through `Lend::of_call`, with the callee's summary in place
    of the callback check.
  - [x] A local inside a `try` whose `catch` reads it.
- [x] A callee that may write the lent slot keeps the call's retain (§ *Standing decisions*, "A callee
  that may write the lent slot is not lent").
- [x] `Core\Arr::reduce` (`crates/nvs-stdlib/src/arr.rs:4452`) hands its carry to the callback. `call_callable`
  (`crates/nvs-runtime/src/callable.rs:193`) gains the form that takes an argument the caller gives
  away, with no retain for it, and `reduce`'s guard no longer holds the carry during the call.
- [x] The ten tests the `user-callees` check names and the two the `carry` check names. Run once.

### 6. concatenation, interpolation and a literal that spreads its target append in place on every target

- [x] A concatenation is one instruction that the pass lends its first operand to, for every target
  the pass can lend. `InstKind::consumes` names a `Concat` of strings' first piece, and the pass's
  `candidates` moves a local into it. `lower_string_self_concat` and `concat_spine_opens_with` are
  gone, so an interpolation is the same case. `lower_carrier_self_append` stays: a carrier is an
  object that a borrowing `CoreCall` grows at count 1, not a `Concat`.
- [x] A literal whose first item is a spread takes that source as an owning operand of its first
  `ArraySpread` (`owned`, set by the pass). `nvs_array_spread_owned` returns a sole list source
  itself, and builds otherwise.
- [x] The two tests the `concat` check names and the two the `spread` check names, and
  `a_literal_that_opens_with_a_spread_of_a_local_takes_its_reference`. Run once, green.

### 7. every copy the compiler keeps is reported where it is written, with its reason and its fix

**Split:** after the table and the two codes land, one writing subagent writes the three `.lspt` cases,
copying the shape of an existing case under `tests/lsp/diagnostics/` and `tests/lsp/lens/`.

- [x] `crates/nvs-ir/src/ownership.rs` records each kept copy: its span, the file's BLAKE3, its reason,
  and its fix as a text edit where one exists. `crates/nvs-codegen/src/lib.rs:382` `Unit` carries the
  table, and a unit loaded from the compile cache has the same table.
  - [x] `crates/nvs-ir/src/copies.rs`: `KeptCopy`, `Reason`, `Fix`, and `table` with the hash and edit.
  - [x] The pass records refused moves (`refusal`) and lent bodies' separated changes (`Reach::fix`).
  - [x] `Program::copies` and `Unit::copies`, on the compiled and the loaded path.
  - [x] `every_copy_the_pass_keeps_is_in_the_table` and `the_copy_table_survives_the_compile_cache`
    pass, run once by name.
- [x] The two warning codes in `crates/nvs-diagnostics/src/code/warnings.rs` after the last one, each
  with its card in end-user prose. `nvs check` lowers and runs the pass, and prints them.
  - [x] `W1032` and `W1033`, each with its card.
  - [x] `nvs_ir::copies::warnings` builds them from the rows, with the fix as one reviewed suggestion.
  - [x] `front_end_in` lowers under `Sink::Check` and adds them before the secret literals are concealed.
  - [x] `a_warning_is_raised_exactly_where_an_easy_fix_removes_a_copy` passes, run once by name.
- [x] `crates/nvs-lsp/Cargo.toml` links `nvs-ir`. The server runs the pass on the open document after
  its type diagnostics, publishes the warnings, and offers each fix as a code action.
  - [x] `CheckCache::left_out` counts the files a partial check left out of the fold.
  - [x] `analyse_through` lowers an error-free program whose table is whole (`copy_warnings`).
  - [x] The two diagnostics cases and two `tests/lsp/actions/` cases for the fixes pass, run once.
- [x] The copy path counts by site when a request is recorded. `crates/nvs-cli/src/debug/index.rs`
  stores the counts, the debug viewer lists the hottest sites with their help, and
  `crates/nvs-lsp/src/recorded.rs` shows the lens.
  - [x] `InstKind::CopyCount` after each row's call or change, and `nvs_copy_count` in codegen.
  - [x] `CopySites`, the armed counts on `Ctx`, and the `copies` spool record at `finish`.
  - [x] The index's `copies` table and its ingest; `a_recorded_request_counts_its_copies_by_site`
    and `a_kept_copy_is_counted_right_after_its_call` pass, run once by name.
  - [x] The debug viewer lists the hottest copy sites with `Reason::help`.
  - [x] `crates/nvs-lsp/src/recorded.rs` shows the lens, and its `.lspt` case.
- [x] The four tests and the three `.lspt` cases the `copy-report` checks name. Run once.

### 8. the fuzzer, Miri and an attack hold the guarantee

- [x] `crates/nvs-fuzz-checks/src/in_place.rs:1`: a generator of small programs over the matrix's axes,
  with random sharing (`$b = $a`), errors and catches. Each program runs with and without the pass, and
  the outputs must match. `replacement_programs_print_the_same_without_in_place` runs a fixed seed
  set. The fuzz target runs the open-ended search.
- [x] The in-place tests of `crates/nvs-runtime/src/array.rs` and `string.rs` are on CI's Miri list.
  The `miri-units` job runs every `nvs-runtime` unit test with no filter, and none of them carries
  `cfg_attr(miri, ignore)`, so they were on it already and the workflow is unchanged.
- [x] `tests/conformance/in-place/a-value-from-the-configuration-is-cut-only-in-its-own-copy.nvst`:
  the program cuts an array and a string it read from `Core\Config`, which every request shares, and
  then reads the configuration again. Both reads print the original. Run once.
- [x] `bun nv suite tests/conformance/ --no-in-place-too` once, green.

### 9. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 8 are done; only stage 9, the gate, is left. The sweep's `env-secret-proofs` red is
stale: no goal defines it, its log file does not exist, and `git log -S` finds it only in this
goal's handoffs and a retired goal's retire commit.

Stage 8 landed this session. `crates/nvs-fuzz-checks/src/in_place.rs` draws a program from bytes
(`program`: a prelude of `Store`, `Box` and `Lib`, then one to five steps over the kind, the
target, the form and the context, with an optional share and a `try` failure), runs it through
`compile::lower_source` with and without `without_in_place`, and asserts the same status and
output, and that the run ends `OK`. `replacement_programs_print_the_same_without_in_place` runs 128
SplitMix64 seeds in about 3 s and is green. The fuzz target is `fuzz/fuzz_targets/in_place.rs`,
hand-run (`tools/nv/cmd/fuzz.ts` `BY_HAND`). The Miri item needed no edit: `miri-units` runs every
`nvs-runtime` unit test, and none is ignored under Miri. The attack case is green, and
`bun nv suite tests/conformance/ --no-in-place-too` was green over all 4032 cases.

Stage 7's report: the viewer's `copies` (`crates/nvs-cli/src/debug/api.rs` `CopySite`), the lens
(`crates/nvs-lsp/src/server.rs` `copy_lenses`), the count (`ownership.rs` `count_copies`, index
schema 8), and the warnings `W1032` and `W1033` (`nvs_ir::copies::warnings`).

### Next

- **Stage 9, the gate.** Run `looper verify` in the background, fix every failure it names in one
  pass (the expected reds are below), and run it once more. Then the goal-end gates of AGENTS.md
  § *In the loop*: `bun nv verify --doc`, `bun nv owners --closes in-place-everywhere` and
  `bun nv ci --check`. The conformance tree, with and without the pass, is already green, so a red
  conformance case at the gate comes from a later edit.
- `crates/nvs-stdlib/src/arr.rs` changed, so every `Core\Arr` perf record is stale. Record them at
  the goal's end as the playbook's perf bullet says, after the last edit to that file.

**What the pass-off run now spends.** With `--no-in-place`, every one-level write into a plain
property copies its array, so a loop of such writes is quadratic. `$s = $s . $x` on a local now
copies too, because the lowering no longer moves it. Only the differential run uses it.

**Expected red at the gate.** An IR snapshot of a program with a kept `array`, `string` or `bytes`
copy now prints `copy.count <site>, v<before>, v<after>` after the call or change; accept it after
reading it. A test that pins the viewer's API shape beyond `debug_api.rs` sees `copies` in a
request's answer and in its `cut`. A test that pins `recorded::viewer_url`'s old signature, or a
lens case whose index has a `copies` table, may see the new lens. A test that pins `nvs_runtime::symbols()` or a key built from it sees
`nvs_copy_count`. A debug index test that reads `SCHEMA` sees 8. A `.lspt` diagnostics case or an `nvs-lsp` test whose error-free
document has such a callee now shows `W1032` or `W1033` too. A `nvs check` test whose program has a callee that changes a lent
value and then reads the slot or throws may now print `W1032` or `W1033`; and a program that type
checks but whose lowering panics now panics in `nvs check` too, which is a lowering bug. Snapshot tests that lower a whole program now show the pass's moves:
a `Core` call on a dead local prints its `_owned` symbol, and a call at a local's last use prints no
retain. `crates/nvs-ir/tests/*.rs` and `crates/nvs-codegen`'s snapshot tests are the likely ones.
Accept each after reading it: a removed retain must sit beside a removed release. A snapshot made
by `lower_first_method` (no pass) of `$s = $s . $x` on a local now prints a `retain` before the
`concat` and a `release` of the old value after; a whole-program one prints neither. A snapshot of a
literal that opens with a spread of a dead local prints `array.spread.owned`. The whole suite
also runs stage 2's 117 owning entries for the first time through real calls. A snapshot with a
by-value `foreach` that writes `$a[$k]` now walks the header phi with no `retain` of the subject, and
`crates/nvs-runtime`'s array tests that assert a separated hashed array's vector length may change,
because the copy keeps its holes. A snapshot of a generator's `advance` now prints `field.take`
where it printed `field.get` and a `retain` at each resume. A snapshot of `$this->p = M($this->p)`
or `C::$s = M(C::$s)` with an owning `M` now prints `_owned`, and no second read of the slot and no
release of it after the store. One of `$this->g['a'] = M($this->g['a'])` prints `array.sole` and a
store after the slot's read, and `array.set.lent`. A snapshot of `$this->o .= $x` is unchanged
with the pass on; with it off (`--no-in-place`), the conformance output is the same, but the IR
shows a `retain` before `str.append` and a second read and release around the store. A snapshot
of a lent property or static property now prints `lend.poison` after the slot's read and after the
call, and `lend.restore` before the call. A snapshot of `$this->p[k] = v`, `$this->p[] = v` or
`C::$s[k] = v` now also prints `lend.poison` around the `array.set` or `array.append`, and with the
pass off a `retain` of the root and a second read and release around the store. The same holds for
a nested write, a nested `.=` and an `unset` on a property or static property root, guarded and
`lateinit` roots included. A snapshot of an element `unset` now lowers its keys before the root's
read, and a guarded one reads the root again after the guard. A `Core` call with a callback that
calls a known function writing none of the slot's chain is now lent where it kept its retain.
A snapshot of `$this->p = $this->f($this->p)` with a known `f` now calls `T::f#lent1`, prints
`lend.poison` around it, and has no retain of the slot's read; the program gains that lent body.
A snapshot of a call that passes a local inside a `try` whose `catch` reads it, to a known Novis
callee, now calls `<callee>#lent<i>` with no retain, and the normal path has no release of the
local. A `reduce` test that counts the carry's references during the callback now sees one fewer.

### Backlog

- A local's element, `$a[k] = M($a[k])`, is not lent: a local is not a slot, so after
  `array.sole` every later read of the local's old value, the error edges' releases included, would
  have to read the sole array. It belongs with stage 6's targets if a measure shows it matters.

- `foreach ([1, 2, 3] as int $x)` is E0401 because the literal's items are `mixed`. An element type
  inferred from the literal would accept it. Whether the checker should infer it belongs to a goal on
  `var` inference, not this one.
- `$f() as string` on a plain `callable`'s `mixed` result is E0401 "expected `string`, found `tainted
  string`". A `mixed` with no taint should not convert to a tainted string. It belongs to a goal on
  taint, not this one.
