---
looper: 1
title: "no request lifts its own limits, and no native loop runs past them"
summary: "Reached when a spawned script's memory ceiling stays within what its tree has left after any Core\\Config call, the backtracking regex tier, every token-chosen key derivation and every native walk over caller-sized input stop at the CPU limit, a guest extension stops once the request is over its ceiling, and whole-object reads and error text are bounded in one place."
context: [crates/nvs-runtime/src/abi.rs, crates/nvs-runtime/src/ctx/limits.rs, docs/rules/http-server/time-is-bounded-inside-a-helper.md, tests/hostile/README.md]
stages:
  - id: 1
    title: a spawned script keeps the clamp of its tree
    status: done
    context: [crates/nvs-runtime/src/ctx/isolate.rs, crates/nvs-runtime/tests/tree_budget.rs, crates/nvs-stdlib/src/config.rs, docs/rules/security/isolate-budget-is-the-trees.md]
  - id: 2
    title: the backtracking regex tier stops at the CPU limit
    status: done
    context: [crates/nvs-stdlib/src/regex.rs, Cargo.toml, docs/rules/core-classes/regex-two-tiers.md]
  - id: 3
    title: a key derivation and a whole read ask before the work
    status: done
    context: [crates/nvs-stdlib/src/crypto.rs, crates/nvs-stdlib/src/jwe.rs, crates/nvs-stdlib/src/storage.rs, crates/nvs-stdlib/src/io.rs, docs/rules/errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself.md]
  - id: 4
    title: error text is clipped where a fault is made
    status: open
    context: [crates/nvs-stdlib/src/json.rs, crates/nvs-stdlib/src/response.rs, docs/rules/errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself.md]
  - id: 5
    title: a native walk over caller-sized input reaches a poll
    status: open
    context: [crates/nvs-stdlib/src/hash.rs, crates/nvs-stdlib/src/sort.rs, crates/nvs-stdlib/src/encoding.rs]
  - id: 6
    title: a guest stops once its request is over the memory ceiling
    status: open
    context: [crates/nvs-ext/src/call.rs, crates/nvs-cli/src/extensions.rs, crates/nvs-ext/src/world.rs, docs/rules/packaging/a-guest-runs-under-the-requests-budget.md]
  - id: 7
    title: "a crossing value is counted once, and the pool has a place for every request"
    status: open
    context: [crates/nvs-ext/src/call.rs, crates/nvs-ext/src/typed.rs, crates/nvs-cli/src/extensions.rs, docs/rules/packaging/a-fresh-instance-per-request.md]
  - id: 8
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: child-keeps-its-clamp
    stage: 1
    name: "a spawned script that calls Core\\Config::set or ::restore keeps the ceiling its tree had left, and its spawn-site sub-cap"
    run: [cargo, test, --test, tree_budget]
    timeout: 1800
    tests:
      - a_child_that_refreshes_its_limits_keeps_the_ceiling_its_tree_had_left
      - a_child_that_restores_its_sub_cap_keeps_the_sub_cap
      - a_placed_child_that_refreshes_keeps_what_remained_when_it_was_placed
      - a_refresh_never_raises_a_childs_script_depth
  - id: backtracking-stops-at-the-deadline
    stage: 2
    name: the backtracking tier polls in step with the bytes it scans and stops at an expired deadline
    run: [cargo, test, --lib, --, a_backtracking_search_stops_at_an_expired_deadline, the_backtracking_tier_polls_in_step_with_the_bytes_it_scans, a_deadline_inside_a_backtracking_search_is_fatal_and_a_spent_budget_is_thrown]
    timeout: 1800
    tests:
      - a_backtracking_search_stops_at_an_expired_deadline
      - the_backtracking_tier_polls_in_step_with_the_bytes_it_scans
      - a_deadline_inside_a_backtracking_search_is_fatal_and_a_spent_budget_is_thrown
  - id: derivation-and-whole-read-ask-first
    stage: 3
    name: "every caller of the key derivation is checked against the CPU time left, and no member reads a whole source outside the bounded reader"
    run: [cargo, test, --lib, --, a_token_chosen_iteration_count_is_checked_against_the_cpu_time_left, every_door_to_the_key_derivation_is_refused_with_no_cpu_time_left, a_stored_object_larger_than_the_memory_left_is_refused_before_it_is_held, no_member_reads_a_whole_source_outside_the_bounded_reader]
    timeout: 1800
    tests:
      - a_token_chosen_iteration_count_is_checked_against_the_cpu_time_left
      - every_door_to_the_key_derivation_is_refused_with_no_cpu_time_left
      - a_stored_object_larger_than_the_memory_left_is_refused_before_it_is_held
      - no_member_reads_a_whole_source_outside_the_bounded_reader
  - id: fault-text-is-clipped
    stage: 4
    name: every Fault constructor cuts a long message on a character boundary and a short one is unchanged
    run: [cargo, test, --lib, --, every_fault_constructor_clips_a_long_message, a_clipped_message_ends_on_a_character_boundary, a_long_json_key_is_quoted_escaped_and_short]
    timeout: 1800
    tests:
      - every_fault_constructor_clips_a_long_message
      - a_clipped_message_ends_on_a_character_boundary
      - a_long_json_key_is_quoted_escaped_and_short
  - id: native-walks-poll
    stage: 5
    name: "every digest, every encoding and the native sort stop at an expired deadline, and the sort leaves its array as it was"
    run: [cargo, test, --lib, --, every_digest_stops_at_an_expired_deadline, every_encoding_stops_at_an_expired_deadline, a_native_sort_stops_at_an_expired_deadline_and_leaves_the_array_as_it_was]
    timeout: 1800
    tests:
      - every_digest_stops_at_an_expired_deadline
      - every_encoding_stops_at_an_expired_deadline
      - a_native_sort_stops_at_an_expired_deadline_and_leaves_the_array_as_it_was
  - id: guest-stops-at-the-ceiling
    stage: 6
    name: a running guest is interrupted at the tick after its request passed the memory ceiling
    run: [cargo, test, --lib, --, a_guest_is_interrupted_once_its_request_is_over_the_memory_ceiling, a_guest_that_logs_without_end_is_stopped_at_the_memory_ceiling, a_guest_cannot_fill_the_resource_table_past_its_cap]
    timeout: 1800
    tests:
      - a_guest_is_interrupted_once_its_request_is_over_the_memory_ceiling
      - a_guest_that_logs_without_end_is_stopped_at_the_memory_ceiling
      - a_guest_cannot_fill_the_resource_table_past_its_cap
  - id: counted-once-and-a-place-for-each
    stage: 7
    name: "a crossing value is charged to its request once, and the pool is sized for every request in flight"
    run: [cargo, test, --lib, --, a_crossing_value_is_charged_to_its_request_once, the_pool_has_a_place_for_every_request_in_flight_times_every_class]
    timeout: 1800
    tests:
      - a_crossing_value_is_charged_to_its_request_once
      - the_pool_has_a_place_for_every_request_in_flight_times_every_class
---

# no request lifts its own limits, and no native loop runs past them

## Why

A request's limits are its memory ceiling, its `cpu_time` and its depth bounds. Seven places let a
request pass one of them today. Most are siblings of a fix that repaired one member.

**A spawned script lifts its own memory ceiling.** `Ctx::isolate` gives a child what the tree has
left (`crates/nvs-runtime/src/ctx/isolate.rs:605`), and writes it to the cached `memory_limit` alone.
`refresh_limits` recomputes every ceiling from the configuration
(`crates/nvs-runtime/src/ctx/limits.rs:934`) and arms the allocator from the child's own zero point
(`crates/nvs-runtime/src/ctx/limits.rs:315`). `Core\Config::restore` always calls it
(`crates/nvs-stdlib/src/config.rs:383`), and an accepted `set` does too. So one call gives the child
the whole configured ceiling again, on top of what its parent holds. A spawn-site `limits:` sub-cap
is an overlay entry (`crates/nvs-runtime/src/ctx/isolate.rs:817`), and `restore` deletes it. Seen on
the release binary: a parent held 20 MiB of a 32 MiB limit, and its child then held 25 MiB. Each
nesting level repeats it, up to `max_script_depth` times the ceiling. Any package code can reach it.

**The backtracking regex tier runs past `cpu_time`.** `max_regex_steps` counts backtracks
(`crates/nvs-stdlib/src/regex.rs:1384`). One step may scan the rest of the subject, and no
backtracking arm polls the deadline (`crates/nvs-stdlib/src/regex.rs:2069`). A lookahead pattern on
160,000 letters ran 18.8 seconds and returned "no match". A remote client reaches it with one body.

**A token picks a key-derivation count.** `Core\Jwe::decrypt` reads `p2c` from the token's header and
runs it with no CPU check (`crates/nvs-stdlib/src/jwe.rs:1137`). 3d8f65f845 gave `deriveKey` the
check (`crates/nvs-stdlib/src/crypto.rs:3175`) and left this door.

**`Core\Storage::get` reads a whole object with no ask** (`crates/nvs-stdlib/src/storage.rs:558`).
3bca039a55 bounded `Core\IO` through `read_bounded` (`crates/nvs-stdlib/src/io.rs:2639`). An object
may be larger than the memory limit, because `put` streams it.

**An error message copies caller text of any length.** d2b9613334 made seven members quote at most
64 bytes (`crates/nvs-runtime/src/abi.rs:290`). The same shape is still written for a repeated JSON
key (`crates/nvs-stdlib/src/json.rs:2386`, `:3464`), a header value, a redirect URL and a cookie
(`crates/nvs-stdlib/src/response.rs:1516`, `:1559`, `:1570`, `:1866`, `:1876`). The JSON key is
request bytes.

**Native walks do not poll.** `rule:http-server/time-is-bounded-inside-a-helper` says a helper that
scales with its input polls the deadline. `sort.rs`, `hash.rs` and `encoding.rs` hold no deadline
read. `bounded_loop` (`crates/nvs-runtime/src/abi.rs:440`) has two callers, and both run a callback
that polls anyway.

**A guest extension runs past the request's memory ceiling.** The epoch tick asks the CPU flag alone
(`crates/nvs-ext/src/call.rs:1326`). A guest that calls `log.write` in a loop makes the host keep a
copy of every line (`crates/nvs-cli/src/extensions.rs:680`) until `cpu_time` ends it. One installed
extension can end the process. Crossing values are also counted twice
(`crates/nvs-ext/src/call.rs:1188`), and the instance pool stops at 4,096 places
(`crates/nvs-cli/src/extensions.rs:127`) while the default admits 10,000 requests in flight.

## Standing decisions

**One function clamps a spawned script to its tree, and the spawn and `refresh_limits` both call
it.** The child stores what the spawn decided: the memory bound as an absolute balance on its own
thread, the output and CPU bounds, and the copied `max_script_depth`. `Ctx::isolate`,
`PlacedIsolate::build` and `narrow_under` write that record through the one function.
`refresh_limits` ends with `tighter(configured, stored)` for each ceiling before it arms the
allocator. A spawn-site sub-cap is folded into the record, so `Core\Config::restore` cannot remove
it. No `Core\Config` call can raise a ceiling above the record. **What it spends:** four words per
`Ctx`, and one subtraction and four compares per refresh. Nothing per allocation and nothing for a
request that spawns nothing. It removes the hand-kept save of `max_script_depth`
(`crates/nvs-runtime/src/ctx/isolate.rs:820`). Usability: after a `restore`, `Core\Config::get`
returns the configured text and the ceiling in force is the tighter record. The limit report names
the real ceiling. Safe default if a ceiling cannot be clamped in `refresh_limits`: a child's
`Core\Config::set` and `::restore` of a `limits.*` key change nothing and `set` returns `false`.
Declined: a second overlay layer in `nvs_config::Request` that `restore` returns to. It is a second
mechanism for one number, and the runtime clamp is what enforces.

**The backtracking tier polls the request's deadline in its own loop, by work done.** `fancy-regex`
is vendored under `vendor/fancy-regex` with one patch, as `html5ever` and `pulldown-cmark` are. The
virtual machine has one dispatch loop that every instruction and every backtrack passes. The patch
counts work there: one unit per instruction, and one unit per 64 bytes of the span a delegated
search is given. Every 4,096 units it calls a poll function set on the builder. A poll that
returns `true` ends the search with the engine's own runtime error. On the Novis side,
`Compiled::Backtracking` holds a wrapper whose every method takes the context. The wrapper puts the
request's deadline word where the poll function reads it, for that call only. `budget_exhausted`
(`crates/nvs-stdlib/src/regex.rs:1526`) asks the deadline first, so an expired deadline is the CPU
or wall-time `FATAL` and a spent step budget stays the catchable throw. It reuses `cpu_time`. No new
limit and no new setting. **What it spends:** one add and one compare per instruction, one relaxed
load per 4,096 units, and one reference count and two thread-local writes per backtracking call. No
memory. The linear tier pays nothing. Safe default if the patch cannot land: the wrapper clamps the
step budget to a work bound divided by the subject's length, and polls between matches. Declined:
charging scanned bytes to `max_regex_steps`. One scan of a 1 MB subject would spend the default
budget, so patterns that pass today would throw.

**The linear tier needs no poll inside one search.** It reads each byte of the subject a bounded
number of times and never backtracks. One search costs the pattern's compiled size times the
subject's length, and both are bounded. `polled` (`crates/nvs-stdlib/src/regex.rs:1876`) already runs
between two matches of `matchAll`, `replace`, `replaceWith` and `split`. Stage 2 pins this with an
attack that runs the same subject through a linear pattern.

**A guest is stopped by the epoch tick, which asks the memory ceiling beside the CPU flag.** `Budget`
gains `memory_spent`. For a served request it reads the flag the allocator raises when the request
crosses its ceiling. The callback at `crates/nvs-ext/src/call.rs:1326` sets `Limit::Memory` and
interrupts. `drive`'s `between` step asks the same after it sends and reads. This needs no new
mechanism: the tick, the flag and `Limit::Memory` exist. It bounds every present and future import
to what a guest can make the host allocate in one tick of 1 ms. **What it spends:** one relaxed load
per tick per running guest. The resource table is also capped near `handle::MOST`. Declined: a
charge in every import. A new import would forget it.

**The CPU check for an iteration count lives in `pbkdf2_sha256`.** It takes the context and the
member's name, and calls `cpu_affords` before the first HMAC
(`crates/nvs-stdlib/src/crypto.rs:1714`). Its callers are `derive_key`
(`crates/nvs-stdlib/src/crypto.rs:1767`) for `Core\Crypto::deriveKey`, and `pbes2_key`
(`crates/nvs-stdlib/src/jwe.rs:577`) for `Core\Jwe::encrypt` (`:740`) and `Core\Jwe::decrypt`
(`:1137`). The check in `nvs_core_crypto_derive_key` (`crates/nvs-stdlib/src/crypto.rs:3174`) goes.
**What it spends:** one compare per derivation, and one lock and one clock read when the cost is
over 1 ms. A refused derivation is the CPU-time `FATAL`, as `deriveKey` gives today. `Core\Password`
keeps its own checks (`crates/nvs-stdlib/src/password.rs:462`), because Argon2 and bcrypt are other
functions. The Postgres SCRAM login has no `Ctx` and keeps its cap on the count.

**A whole read asks the budget in one reader.** `read_bounded` moves beside the byte sources in
`crates/nvs-stdlib/src/byte_source.rs` and `Core\IO` and `Core\Storage::get` both call it. Verified
today: `Core\IO::read` asks (`crates/nvs-stdlib/src/io.rs:2639`), both byte-source file reads ask per
piece (`crates/nvs-stdlib/src/byte_source.rs:749`, `:777`), and `Core\Storage::get` does not. Not
verified, so stage 3 lists them first: the HTTP client's body, `Core\Cache::get` on each store, a
claimed queue payload, a child process's captured output (`crates/nvs-stdlib/src/process.rs:2418`)
and a session read. Each one the list shows without an ask moves to the reader in stage 3. A source
test then fails on any `read_to_end` or `read_to_string` in a member that is not behind `take` or in
the reader. **What it spends:** one compare per growth of the buffer.

**Error text is clipped once, in the `Fault` constructors.** `plain`
(`crates/nvs-runtime/src/abi.rs:267`) is the one function all six constructors pass. It cuts a
message past 8 KiB on a character boundary and adds `…`. The message becomes a type only `abi.rs` can
build, so a `Fault` written as a variant no longer compiles. **What it spends:** one length compare
per fault. Usability: a message over 8 KiB loses its tail, which may shorten a long `Core\Test`
diff. Where a test in the tree expects a longer message, the cap rises to the next power of two that
passes it.

**`quoted` stays, as the bound on the build of a message.** A constructor sees the message after
`format!` built it. For an 8 MiB argument the build holds the argument and up to 24 MiB more for a
moment (`crates/nvs-runtime/src/abi.rs:283`). That put the seven members of d2b9613334 past the
`no-overshoot` bound, and a clip in the constructor does not lower it. So the eight `quoted` calls
stay, and the sites that format request text gain one: the JSON key, the header value, the redirect
URL and the cookie name and value. Security outranks simplicity here. `Quoted` gains a `Debug` form,
so the JSON key keeps its escaping. A member that forgets `quoted` is still held to 8 KiB kept, and
`no-probe-overshoot` and the debug backstop (`crates/nvs-runtime/src/abi.rs:705`) catch its build.
Declined: constructors that take only `format_args!`. It is a change at 1,500 sites in 116 files,
and a wrapper that passes a built `String` would need a second door.

**`bounded_loop` stays, and the native walks adopt it.** The rule requires the poll, and the
watchdog cannot interrupt native code. A SHA-3 digest, a transcode or a sort over a value the default
memory ceiling admits runs far past the 50 ms poll gap. So `Core\Hash` and `Core\Encoding` walk
their input in chunks of 4 KiB through `bounded_loop`. The native sort orders a permutation of
indexes (`crates/nvs-stdlib/src/sort.rs:115`), so it can stop and leave the array as it was. Its key
function polls through a batch counter that `abi.rs` owns, and after a stop every key compares equal
until the sort returns the fault. A walk polls when its cost per byte is more than a copy: hashing,
sorting, transcoding, parsing, matching. A walk that only copies is bounded by the bytes the memory
ceiling let it hold. **What it spends:** one decrement per chunk or per key, and one relaxed load
per 256 of them. Declined: deleting the combinator and rewording the rule. It would leave a hash of
128 MiB outside `cpu_time`.

**Crossing values are counted once, by the allocator.** `budget::Accounting` already counts every
host allocation of a lowered argument, a lifted result, a body and a header list. The second count
goes: `Sent::heap` and `Got::heap` (`crates/nvs-ext/src/call.rs:1445`, `:1462`), `on_heap` and
`lifted` (`:1620`, `:1626`), the `Heap` trait and its impls (`crates/nvs-ext/src/typed.rs:228`), the
`Charge` of `wasi/http.rs` and `Handles.grown` (`crates/nvs-ext/src/handle.rs:85`). `charge` stays
for guest linear memory, which is mapped and not allocated. `afford` stays before a size the guest
chose. After lowering and after lifting, the call asks `memory_spent`. **What it saves:** one walk
over every crossing value per call, and about 250 lines. A 40 MiB result is charged 40 MiB, not 80.
This needs stage 6's `memory_spent` first.

**The pool has a place for every request in flight, for each class.** `size_pool`
(`crates/nvs-cli/src/extensions.rs:271`) computes `max_in_flight` times the classes, and the constant
`MOST_SLOTS` goes. No new setting. A place reserves 6 GiB of address space today, so the default
product of 40,000 places does not fit. Where the product passes half of the address space, the
engine is built with a smaller reservation per place, and guest code runs with explicit bounds
checks. The compiled-component cache keys on that reservation. **What it spends:** address space
only, never committed memory. With the default settings every guest memory access gains a bounds
check, which is latency spent to keep one client from taking every place. A pool that fits at 6 GiB
a place keeps today's code. The session reads the pooling allocator's reference of the installed
wasmtime first. Safe default if the pool cannot run under a smaller reservation: the places stay at
6 GiB up to the address-space share, a call with no place still throws `ExtensionError`, and
`rule:packaging/a-fresh-instance-per-request` states the real count.

**Each limit fix gets an attack that ends at the limit.** Each is a case under `tests/hostile/`, in
the folder of the member it attacks, with the next free number. A case that a limit ends carries
`// hostile: ends-early <N> FATAL`, and its ending is read off a run. Its comments follow AGENTS.md
§ *Text an end user reads*. No case holds a ready payload: it builds its input in its own steps. The
floor checks `no-overshoot` and `no-probe-overshoot` stay green.

**A spawned script that reaches a limit is the child's failure, and its attack runs to its end.**
A child stopped at its memory share returns a failed `ScriptResult` to `await`, and the parent goes
on, because the child's memory is freed. So an attack on a child's limit carries no `ends-early`
marker. It throws `RuntimeError` when `$result->ok` says the child finished, which makes the case red
on a runtime that let the child through. The stage 1 case was run once with the clamp disabled and
failed that way. A child on its parent's core records no output or CPU ceiling, because the root's
readings of those bound it, as they did before.

**Lanes:** stages 1, 2, 3 and 6 touch disjoint files and may run side by side. Stage 5 follows
stage 4, because both change `crates/nvs-runtime/src/abi.rs`. Stage 7 follows stage 6, because both
change `crates/nvs-ext/src/call.rs` and `crates/nvs-cli/src/extensions.rs`.

## Stages

### 1. a spawned script keeps the clamp of its tree

**Does:** store what a spawn decided on the child, and clamp every refresh of its ceilings to it.

**Split:** an `Explore` subagent finds one case under `tests/hostile/` that uses `spawn script`, and
reports its path and how it names the child's file. The attack below copies that shape.

- [x] `Ctx` gains the spawn record: memory as an absolute balance, output, CPU and the copied script
  depth. One function writes it (`rule:security/isolate-budget-is-the-trees`,
  `crates/nvs-runtime/src/ctx/isolate.rs:1139`).
- [x] `Ctx::isolate` and `PlacedIsolate::build` set the child's ceiling through that function
  (`crates/nvs-runtime/src/ctx/isolate.rs:605`, `crates/nvs-runtime/src/ctx/isolate.rs:1098`).
- [x] `narrow_under` folds the resolved sub-cap into the record, and its save of `max_script_depth`
  goes (`crates/nvs-runtime/src/ctx/isolate.rs:808`).
- [x] `refresh_limits` clamps memory, CPU, output and script depth to the record before
  `arm_memory_ceiling` (`rule:security/isolate-budget-is-the-trees`,
  `crates/nvs-runtime/src/ctx/limits.rs:934`).
- [x] Tests `a_child_that_refreshes_its_limits_keeps_the_ceiling_its_tree_had_left`,
  `a_child_that_restores_its_sub_cap_keeps_the_sub_cap`,
  `a_placed_child_that_refreshes_keeps_what_remained_when_it_was_placed` and
  `a_refresh_never_raises_a_childs_script_depth` (`crates/nvs-runtime/tests/tree_budget.rs:192`).
- [x] Attack: a parent holds most of a small memory limit and spawns a script. The child calls
  `CoreConfig::restore` and then builds a string that only a fresh ceiling could hold. The child
  stops at the memory limit and the parent runs to its end; the case throws if the child finished
  (`tests/hostile/core/Config/restore/02-a-spawned-script-restores-a-setting.nvs:1`).
- [x] The rule fragment says a refresh never widens a spawned script, and states the cost
  (`docs/rules/security/isolate-budget-is-the-trees.md:6`).
- [x] The comment on `Core\Config::restore` says what a spawned script gets back
  (`crates/nvs-stdlib/src/config.rs:377`).

### 2. the backtracking regex tier stops at the CPU limit

**Does:** vendor `fancy-regex` with a work-counted poll in its dispatch loop, and run every
backtracking call under the request's deadline.

- [x] Copy the published 0.19.2 to `vendor/fancy-regex`, keep its license file, and add the
  `[patch.crates-io]` row with a comment like `html5ever`'s (`Cargo.toml:1318`, `Cargo.toml:640`).
- [x] The patch: a poll function on the builder, a work counter in the dispatch loop, one unit per
  instruction and one per 64 bytes of a delegated span, a poll every 4,096 units
  (`rule:http-server/time-is-bounded-inside-a-helper`, `vendor/fancy-regex/src/vm.rs:1`).
- [x] `Compiled::Backtracking` holds a wrapper whose methods take the context and set the deadline
  word for the call. Every arm that runs the engine calls it
  (`crates/nvs-stdlib/src/regex.rs:1382`, `crates/nvs-stdlib/src/regex.rs:1830`,
  `crates/nvs-stdlib/src/regex.rs:2021`, `crates/nvs-stdlib/src/regex.rs:2069`,
  `crates/nvs-stdlib/src/regex.rs:2243`, `crates/nvs-stdlib/src/regex.rs:2346`,
  `crates/nvs-stdlib/src/regex.rs:2408`).
- [x] `budget_exhausted` asks the deadline first and gives the `FATAL` through `poll_deadline`
  (`rule:core-classes/regex-two-tiers`, `crates/nvs-stdlib/src/regex.rs:1526`).
- [x] The doc comment of `polled` says what bounds each tier (`crates/nvs-stdlib/src/regex.rs:1869`).
- [x] Tests `a_backtracking_search_stops_at_an_expired_deadline`,
  `the_backtracking_tier_polls_in_step_with_the_bytes_it_scans`, which counts polls at two subject
  sizes, and `a_deadline_inside_a_backtracking_search_is_fatal_and_a_spent_budget_is_thrown`
  (`crates/nvs-stdlib/src/regex.rs:2531`).
- [x] Attack: a small `cpu_time`, then an unanchored lookahead pattern on a long run of one letter.
  It ends `FATAL` (`tests/hostile/core/Regex/matches/02-a-lookahead-over-a-long-subject.nvs:1`).
- [x] Attack: the same subject through a pattern of the linear tier returns inside the timeout
  (`tests/hostile/core/Regex/matches/03-a-linear-pattern-over-a-long-subject.nvs:1`).
- [x] `rule:core-classes/regex-two-tiers` says the step budget bounds backtracks and `cpu_time`
  bounds the work, with the cost (`docs/rules/core-classes/regex-two-tiers.md:5`).

### 3. a key derivation and a whole read ask before the work

**Does:** move the CPU check into `pbkdf2_sha256`, and send every whole read through one reader.

**Split:** an `Explore` subagent lists every production read of a whole source in
`crates/nvs-stdlib/src`: the HTTP client's body, each `Core\Cache` store's `get`, a queue payload, a
child's captured output and a session read. It reports `file.rs:NN` and whether an ask comes first.

- [x] `pbkdf2_sha256` takes the context and the member's name and calls `cpu_affords` first
  (`rule:http-server/time-is-bounded-inside-a-helper`, `crates/nvs-stdlib/src/crypto.rs:1714`).
- [x] `derive_key` and `nvs_core_crypto_derive_key` pass them, and the member's own check goes
  (`crates/nvs-stdlib/src/crypto.rs:1767`, `crates/nvs-stdlib/src/crypto.rs:3174`).
- [x] `pbes2_key` passes them from `Core\Jwe::encrypt` and `::decrypt`, and `decrypt` takes its
  context (`rule:security/jwe-compact-subset`, `crates/nvs-stdlib/src/jwe.rs:571`,
  `crates/nvs-stdlib/src/jwe.rs:740`, `crates/nvs-stdlib/src/jwe.rs:978`,
  `crates/nvs-stdlib/src/jwe.rs:1137`).
- [x] Tests `a_token_chosen_iteration_count_is_checked_against_the_cpu_time_left` and
  `every_door_to_the_key_derivation_is_refused_with_no_cpu_time_left`
  (`crates/nvs-stdlib/src/jwe.rs:1137`, `crates/nvs-stdlib/src/crypto.rs:5143`).
- [x] Attack: a small `cpu_time`, then a password-keyed token whose header names the largest
  iteration count. It ends `FATAL` before the derivation
  (`tests/hostile/core/Jwe/decrypt/02-a-token-names-the-largest-iteration-count.nvs:1`).
- [x] `read_bounded` moves beside the byte sources and `Core\IO` calls it there
  (`rule:errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself`,
  `crates/nvs-stdlib/src/io.rs:2639`, `crates/nvs-stdlib/src/byte_source.rs:744`).
- [x] `Core\Storage::get` reads through it (`crates/nvs-stdlib/src/storage.rs:556`).
- [x] Each read the list shows without an ask moves to the reader
  (`crates/nvs-stdlib/src/process.rs:2418`).
  - [x] The list: a child's capture asks before each growth (`crates/nvs-stdlib/src/process.rs:1297`),
    and `process.rs:2418` is test code. `Core\Storage::get` read whole with no ask, and now reads
    through the reader.
  - [x] The HTTP client's body asks for each 8 KiB pull before it holds it
    (`crates/nvs-stdlib/src/http/transport.rs:1188`).
  - [x] A Redis bulk reply, for the shared cache and the shared session store, asks for its length
    before it reads the body (`crates/nvs-stdlib/src/cache/redis.rs:826`).
  - [x] A session's SQLite record comes out of the query only when it fits the headroom, and a
    larger one asks its length (`crates/nvs-stdlib/src/session/sqlite.rs:146`).
  - [x] The memory cache tier asks for an entry's length before it copies it, with test
    `a_memory_entry_larger_than_the_memory_left_is_refused_before_it_is_copied`
    (`crates/nvs-stdlib/src/cache.rs:1273`).
- [x] Tests `a_stored_object_larger_than_the_memory_left_is_refused_before_it_is_held` and
  `no_member_reads_a_whole_source_outside_the_bounded_reader`. The second reads the sources of
  `crates/nvs-stdlib/src` and names each offending line (`crates/nvs-stdlib/src/storage.rs:757`).
- [x] Attack: a small memory limit, `put` of a streamed object larger than it, then `get`. It ends
  `FATAL` inside the bound (`tests/hostile/core/Storage/get/02-an-object-larger-than-the-limit.nvs:1`).
- [x] The rule fragment names the one reader
  (`docs/rules/errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself.md:1`).

### 4. error text is clipped where a fault is made

**Does:** clip every message in `plain`, make the message a type only `abi.rs` builds, and quote
request text at the sites that still copy it.

**Split:** an `Explore` subagent lists every construction of a `Fault` variant outside
`crates/nvs-runtime/src/abi.rs`, and every `match` that reads a variant's message, as `file.rs:NN`.

- [ ] `plain` cuts a message past 8 KiB on a character boundary and adds `…`
  (`rule:errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself`,
  `crates/nvs-runtime/src/abi.rs:267`).
- [ ] The message of `Thrown`, `ThrownWithSlots` and `Fatal` becomes a type with a private field, and
  the readers the list names take its text (`crates/nvs-runtime/src/abi.rs:103`).
- [ ] `Quoted` gains a `Debug` form that escapes the excerpt (`crates/nvs-runtime/src/abi.rs:296`).
- [ ] The repeated-key messages of the generic and the typed decoder quote the key
  (`crates/nvs-stdlib/src/json.rs:2386`, `crates/nvs-stdlib/src/json.rs:3464`).
- [ ] `setHeader`, `redirect` and `addCookie` quote the value, the URL and the cookie's name and
  value (`crates/nvs-stdlib/src/response.rs:1516`, `crates/nvs-stdlib/src/response.rs:1559`,
  `crates/nvs-stdlib/src/response.rs:1570`, `crates/nvs-stdlib/src/response.rs:1866`,
  `crates/nvs-stdlib/src/response.rs:1876`).
- [ ] Tests `every_fault_constructor_clips_a_long_message`,
  `a_clipped_message_ends_on_a_character_boundary` and `a_long_json_key_is_quoted_escaped_and_short`
  (`crates/nvs-runtime/src/abi.rs:1599`, `crates/nvs-stdlib/src/json.rs:5908`).
- [ ] Attacks, each an 8 MiB text under a 16 MiB limit that checks the message is short: a JSON
  document with one long key twice, a header value, a redirect URL and a cookie value
  (`tests/hostile/core/Json/decode/02-a-long-key-twice-under-a-16-mib-limit.nvs:1`,
  `tests/hostile/core/Response/setHeader/02-an-8-mib-value-under-a-16-mib-limit.nvs:1`,
  `tests/hostile/core/Response/redirect/02-an-8-mib-url-under-a-16-mib-limit.nvs:1`,
  `tests/hostile/core/Response/addCookie/02-an-8-mib-value-under-a-16-mib-limit.nvs:1`).
- [ ] The rule fragment says a message is held to 8 KiB, and that `quoted` bounds its build
  (`docs/rules/errors/an-allocation-past-the-ceiling-is-refused-in-front-of-itself.md:1`).

### 5. a native walk over caller-sized input reaches a poll

**Does:** walk hashes and transcodes in chunks through `bounded_loop`, and poll inside the native
sort's key function.

**Split:** an `Explore` subagent lists every helper in `crates/nvs-stdlib/src` that walks a
caller-sized value with no callback and no deadline read. It reports `file.rs:NN` and whether the
walk copies or does more work per byte.

- [ ] A batch counter for a loop another crate drives, beside `poll_deadline`, with its cost in the
  doc comment (`rule:http-server/time-is-bounded-inside-a-helper`,
  `crates/nvs-runtime/src/abi.rs:469`).
- [ ] `Core\Hash::of`, `::hmac` and a stream's `update` feed their input in 4 KiB chunks through
  `bounded_loop` (`crates/nvs-stdlib/src/hash.rs:764`, `crates/nvs-stdlib/src/hash.rs:983`,
  `crates/nvs-stdlib/src/hash.rs:1031`, `crates/nvs-stdlib/src/hash.rs:1169`).
- [ ] Every decode and encode walk of `Core\Encoding` runs in chunks through `bounded_loop`
  (`crates/nvs-stdlib/src/encoding.rs:167`).
- [ ] The native sort's key function polls through the batch counter, and a stopped sort returns the
  fault with the array untouched (`crates/nvs-stdlib/src/sort.rs:115`,
  `crates/nvs-stdlib/src/sort.rs:214`).
- [ ] Tests `every_digest_stops_at_an_expired_deadline` and
  `every_encoding_stops_at_an_expired_deadline`, which read the digest and the encoding tables, and
  `a_native_sort_stops_at_an_expired_deadline_and_leaves_the_array_as_it_was`
  (`crates/nvs-stdlib/src/hash.rs:1512`, `crates/nvs-stdlib/src/encoding.rs:299`,
  `crates/nvs-stdlib/src/sort.rs:739`).
- [ ] Attacks: a small `cpu_time`, then a slow digest of a large value, and a sort of a large array.
  Each ends `FATAL` (`tests/hostile/core/Hash/of/02-a-large-value-under-a-small-cpu-time.nvs:1`,
  `tests/hostile/core/Arr/sort/02-a-large-array-under-a-small-cpu-time.nvs:1`).
- [ ] Each further walk the list names that does more than copy gets the poll here when it is in
  these three files (`crates/nvs-stdlib/src/hash.rs:764`). Any other gets a gap record under
  `data/gaps/`, in the shape the records there have, naming its module and no goal.
- [ ] The rule fragment names the batch counter beside the combinator, and says which walks poll
  (`docs/rules/http-server/time-is-bounded-inside-a-helper.md:7`).

### 6. a guest stops once its request is over the memory ceiling

**Does:** make the epoch tick and the step between host calls ask the request's memory ceiling, and
cap the resource table.

**Split:** an `Explore` subagent finds the fixture components the tests of `crates/nvs-ext` and
`crates/nvs-cli` load, and whether a case under `tests/hostile/` can load a third-party component.

- [ ] `Budget` gains `memory_spent`, and `Meter` reports it from a figure a test sets
  (`rule:packaging/a-guest-runs-under-the-requests-budget`, `crates/nvs-ext/src/call.rs:121`,
  `crates/nvs-ext/src/call.rs:540`).
- [ ] `Spent` answers it from the flag the allocator raises at the crossing
  (`crates/nvs-cli/src/extensions.rs:658`).
- [ ] The epoch callback sets `Limit::Memory` and interrupts when it is `true`
  (`crates/nvs-ext/src/call.rs:1324`).
- [ ] `drive`'s `between` step asks it after sends and reads
  (`crates/nvs-cli/src/extensions.rs:877`, `crates/nvs-cli/src/extensions.rs:925`).
- [ ] The resource table gets a capacity near `handle::MOST` (`crates/nvs-ext/src/wasi.rs:176`).
- [ ] Tests `a_guest_is_interrupted_once_its_request_is_over_the_memory_ceiling`,
  `a_guest_that_logs_without_end_is_stopped_at_the_memory_ceiling` and
  `a_guest_cannot_fill_the_resource_table_past_its_cap` (`crates/nvs-ext/src/call.rs:540`,
  `crates/nvs-cli/src/extensions.rs:1513`).
- [ ] Attack: where a hostile case can load a third-party component, a guest logs 1 MiB lines in a
  loop and the case ends `FATAL` at the memory limit. Where none can, the second test above is the
  attack and the handoff says so (`tests/hostile/README.md:117`).
- [ ] The rule fragment says the tick asks memory, and what a guest can allocate in one tick
  (`docs/rules/packaging/a-guest-runs-under-the-requests-budget.md:1`).

### 7. a crossing value is counted once, and the pool has a place for every request

**Does:** delete the second count of crossing values, and size the instance pool from
`max_in_flight` and the class count alone.

- [ ] The call path drops the measure-and-charge pairs and asks `memory_spent` after lowering and
  after lifting (`rule:packaging/a-guest-runs-under-the-requests-budget`,
  `crates/nvs-ext/src/call.rs:1188`, `crates/nvs-ext/src/call.rs:1379`).
- [ ] `Sent::heap`, `Got::heap`, `on_heap`, `lifted`, the `Heap` trait with its impls, the `Charge`
  of `wasi/http.rs` and `Handles.grown` go (`crates/nvs-ext/src/call.rs:1445`,
  `crates/nvs-ext/src/call.rs:1620`, `crates/nvs-ext/src/typed.rs:228`,
  `crates/nvs-ext/src/wasi/http.rs:82`, `crates/nvs-ext/src/handle.rs:85`).
- [ ] Test `a_crossing_value_is_charged_to_its_request_once` counts the balance around one call
  with a large result (`crates/nvs-cli/src/extensions.rs:1513`).
- [ ] `size_pool` computes `max_in_flight` times the classes, and `MOST_SLOTS` goes
  (`rule:packaging/a-fresh-instance-per-request`, `crates/nvs-cli/src/extensions.rs:127`,
  `crates/nvs-cli/src/extensions.rs:271`).
- [ ] The engine takes a smaller reservation per place when the pool would pass half of the address
  space, and the compiled-component cache keys on it (`crates/nvs-ext/src/call.rs:893`,
  `crates/nvs-ext/src/load.rs:359`).
- [ ] Test `the_pool_has_a_place_for_every_request_in_flight_times_every_class` reads the default
  `max_in_flight` and the built-in class list (`crates/nvs-cli/src/extensions.rs:1513`).
- [ ] Both rule fragments state the one count, the pool's size and what the bounds checks cost
  (`docs/rules/packaging/a-guest-runs-under-the-requests-budget.md:1`,
  `docs/rules/packaging/a-fresh-instance-per-request.md:1`).

### 8. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1, 2 and 3 are done. Every whole read in `nvs-stdlib` asks the memory ceiling before it
holds its piece: `read_bounded` (`crates/nvs-stdlib/src/byte_source.rs`) for the byte sources, and
an `nvs_runtime::affordable` ask in `Incoming::whole`, Redis's `Wire::exact`, the session file
store's `load` and the cache's `memory_get`. A refusal there records the breach, so a `String` or
`io::Error` that carries it still ends as the memory `FATAL` through `run_helper`. `memory_get` is
private and returns a `Result`, and tests read the tier through `memory_held`. The session `load`
passes `budget::headroom()` into its query, which gives the record only when it fits.

`crate::tests::out_of_cpu` (`crates/nvs-stdlib/src/lib.rs`) installs a CPU reading that is one
hour only on a thread that asked. Use it for any later CPU-refusal unit test.

The sweep's reds: no check named `env-secret-proofs` exists. Nightly 43001191130 ran at
`f4c1d5c7e2`, and its one failure, `a_changed_server_timeout_applies_to_the_next_connection`
(`crates/nvs-cli/tests/live_config.rs:2520`), held a connection 30 s after a reload of
`server.header_timeout` on every runner. The push CI at `ec3c68d1d0` ran the same test jobs green,
so it is not reproduced. If a later nightly shows it again, it belongs to `waits-are-bounded`.
That push CI's own reds are fixed here: the duplicate `.http.out` snippet output and two Miri
failures in `reflect.rs`'s tests, where each `vault_open as NvsFn` had its own address. Its `lint`
and Miri 7/12 reds were a rustup download timeout.

### Next

Stage 4 is next, and it starts with its `Explore` list of `Fault` variant constructions outside
`abi.rs`.
- `crates/nvs-runtime/src/abi.rs:267`: `plain` clips a message past 8 KiB on a character boundary.
- `crates/nvs-runtime/src/abi.rs:103`: the message becomes a type with a private field.
- `crates/nvs-stdlib/src/json.rs:2386`, `:3464` and `crates/nvs-stdlib/src/response.rs:1516`:
  the request-text sites gain `quoted`.
Stage 6 (`crates/nvs-ext/src/call.rs:1326`) touches other files and may run beside it.

Tests that may go red at the gate: `crates/nvs-host/tests/limits.rs`'s two `handler_isolate`
cases, which assert reserve numbers the handler's allocator is now armed with. The session file
store's cases in `crates/nvs-stdlib/src/session/sqlite.rs` and `session.rs`, whose `load` query
changed and did not run this session. Any test that matches `fancy_regex::RuntimeError`
exhaustively outside `regex.rs` would need the new variant; none was found in the workspace build.

### Backlog

- `Core\Config::restore` takes no package word. It belongs to the goal `package-authority-closed`.
- A resource destructor that sends HTTP at request end waits with no bound. It belongs to the goal
  `waits-are-bounded`.

The open items `registry-waits-hold` left when it was verified:

- `Publish::publish` answers a retry before it checks the token's scope
  (`apps/registry/src/Publish.nvs:74`). Owner: `rule:packaging/publishing-never-overwrites`.
- Adding a passkey from a recovery-code session writes no notice (`apps/registry/src/App.nvs:319`).
  Owner: `rule:packaging/an-owner-is-notified-without-email`.

The open items `package-authority-closed` left when it was retired:

- Grant enforcement through a callable, a `mixed` receiver, inheritance and namespace identity is not
  reviewed. Owner: `rule:security/authority-is-the-enclosing-namespace`.
- `require` of an application file from a package namespace was not traced.
- `Core\Ldap\Connection::authenticate` dials again with no `require`: for a `connect`-made connection
  it resolves the host anew through `walk`, with no address policy
  (`crates/nvs-stdlib/src/ldap/mod.rs:495`, `:598`).
- `Core\Cli`'s prompt opens the terminal device by name with `std::fs` and no door
  (`crates/nvs-runtime/src/terminal.rs:632`).
- `Core\Session`'s file store opens SQLite in the data folder with no door
  (`crates/nvs-stdlib/src/session/sqlite.rs:111`).
- `Core\Crypto` signs with `ring`'s `SystemRandom`, which a test seed does not reach
  (`crates/nvs-stdlib/src/crypto.rs:2794`, `:3027`).
- The scan of `no_class_with_a_reason_for_none_calls_a_door` sees a door called in a class's module.
  A `NEEDS_NONE` class that reaches a door through a helper in another module, with a forwarded
  label, is not seen (`crates/nvs-stdlib/tests/capability.rs`).
- A package's `Core\Config::get` of a key that is not a secret directive needs `env` only because a
  grant line is asked per member. Asking it only for a literal secret key would let a package read
  `limits.*` with no line (`crates/nvs-types/src/capability.rs:224`).
- An array literal of the program's own ints, such as `[1, 2]`, passed to a method of a `mixed`
  receiver is refused as `E0895`, which calls it `mixed` text. The refusal is in the safe direction,
  but `rule:security/taint-propagation` says such a literal carries no outside text
  (`report_args_with_no_parameter_list`, `crates/nvs-types/src/expr/calls.rs`, with
  `check_array_literal`).
- A property-path or string-with-holes failure still names the bad segment's text, such as
  ``has no public property `$nmae` `` (`nvs_property_path_mismatch`, `nvs_string_holes_mismatch`,
  `crates/nvs-runtime/src/helpers.rs`). Name the segment by position instead.
- `$fn->call($obj, $a)` on a typed callable checks only `$a`'s qualifier against the written
  parameter, not its type (`infer_callable_rebind`, `crates/nvs-types/src/expr/calls.rs`).
- A `.=` compound write through an erased property is not asked the untyped-write question, only the
  plain `=` (`crates/nvs-types/src/expr/assign.rs:1083`).
- A third-party extension method that takes a resource may return text built from an earlier
  `tainted` argument as plain text (`crates/nvs-types/src/ext_lib.rs:290`). Not traced to a run.
- `slot_check` and `test_shape` are two maps from a type to a run-time test. An intersection type
  and `iterable` stay `SlotCheck::Any` in the first, and a single-value `string` keeps its tag alone
  (`crates/nvs-ir/src/lower/mod.rs:602`, `crates/nvs-ir/src/lower/expr.rs:7881`).
- A callable's `invoke` row carries no parameter type spellings, so a bare `callable`'s refusal
  names no type (`crates/nvs-ir/src/lower/anon_fn.rs:374`, `crates/nvs-runtime/src/dispatch.rs:286`).
- `check_param_class` runs on every call of an anonymous function, a proven one too
  (`crates/nvs-ir/src/lower/anon_fn.rs:454`).
- `Core\Str::format('%1$s', $o)` over a `Stringable` object throws "it does not implement
  `Stringable`": `stringify` finds no compiled `toString` (`crates/nvs-runtime/src/helpers.rs:2049`).
  Once it works, `crates/nvs-codegen/tests/borrowed_reads.rs` gains a row for it.
