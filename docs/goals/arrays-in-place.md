---
looper: 1
title: "every Core\\Arr member keeps the entries it does not change where they are"
summary: "Reached when a list has room at both ends, a map finds its first live entry in one step, every Core\\Arr member that owns its subject changes only what its result changes, and every array ladder that cuts, extends or rebuilds one array a step at a time is linear."
context: [docs/rules/types/a-replaced-value-is-changed-in-place.md, docs/rules/types/arrays.md, crates/nvs-runtime/src/array.rs]
stages:
  - id: 1
    title: the ladders show every quadratic array shape
    status: done
    context: [benches/scaling/README.md, benches/scaling/arrays/sort.nvs]
  - id: 2
    title: a list has room at both ends, and a map finds its first live entry in one step
    status: done
  - id: 3
    title: every Core\Arr member changes only what its result changes
    status: done
    context: [crates/nvs-stdlib/src/arr.rs, crates/nvs-stdlib/tests/in_place.rs]
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: runtime-room
    stage: 2
    name: "a list drops its first entries and prepends without moving the rest, a map finds its first entry past front holes in one step, and compaction frees both"
    run: [cargo, test, --lib, --, a_list_drops_its_first_entries_without_moving_the_rest, a_list_prepends_into_its_front_room, a_map_finds_its_first_entry_past_front_holes_in_one_step, front_room_and_holes_are_freed_by_compaction]
    timeout: 1800
    tests:
      - a_list_drops_its_first_entries_without_moving_the_rest
      - a_list_prepends_into_its_front_room
      - a_map_finds_its_first_entry_past_front_holes_in_one_step
      - front_room_and_holes_are_freed_by_compaction
  - id: runtime-reads
    stage: 2
    name: "an array names its last live entry without walking from the front, and a list finds an entry by its position in one step"
    run: [cargo, test, --lib, --, an_array_names_its_last_live_entry_without_walking_from_the_front, a_list_finds_an_entry_by_its_position_in_one_step]
    timeout: 1800
    tests:
      - an_array_names_its_last_live_entry_without_walking_from_the_front
      - a_list_finds_an_entry_by_its_position_in_one_step
  - id: read-ladders
    stage: 3
    name: "reading the last entry of a growing list and reading a long list page by page both grow linearly"
    run: [bun, nv, scaling, --check, arrays/read-]
    timeout: 3600
  - id: every-member
    stage: 3
    name: "every owning Core\\Arr member has a row, and on a sole subject each one keeps the entries it does not change in the same storage"
    run: [cargo, test, --test, in_place, --, every_arr_member_has_a_row_here, every_arr_member_keeps_what_it_does_not_change, a_shared_subject_builds_only_the_window_it_keeps]
    timeout: 1800
    tests:
      - every_arr_member_has_a_row_here
      - every_arr_member_keeps_what_it_does_not_change
      - a_shared_subject_builds_only_the_window_it_keeps
  - id: ladders
    stage: 3
    name: "every in-place array ladder grows linearly"
    run: [bun, nv, scaling, --check, arrays/in-place-]
    timeout: 3600
---

# every Core\Arr member keeps the entries it does not change where they are

## Why

`in-place-everywhere` makes the compiler hand every replaced array to its `Core\Arr` member as the
array's only reference. This goal makes each member use that: it changes only what its result changes,
and leaves every other entry where it is.

Measured on 2026-11-17, emptying a list one entry at a time:

| entries | `slice($x, 1)` on a static property | `withoutFirst($x)` on a local | `unset($x[firstKey])` |
|---|---|---|---|
| 4,000 | 101 ms | 261 ms | 1 ms |
| 16,000 | 1.7 s | 5.6 s | 15 ms |
| 32,000 | 8.7 s | 29 s | 62 ms |

Two causes are in the representation. A packed list has no room at its front, so dropping its first
entry moves every other one. A map walks its front holes on every `firstKey` until compaction
(`crates/nvs-runtime/src/array.rs:140`), which is why even the `unset` column grows faster than linearly.
`withoutFirst` is also about 3× slower per entry than `slice`, because it inserts every kept key into a
new table one at a time.

The review of 2026-11-17 measured two reads that walk, on the release binary:

| Shape | Smaller | Doubled |
|---|---|---|
| `Core\Arr::last($a)` once per entry | 10 ms at 20,000 entries | 49 ms at 40,000 |
| `Core\Arr::slice($a, $at, 50)` page by page over the whole list | 62 ms at 100,000 entries | 336 ms at 200,000 |

`last` and `lastKey` walk every slot from the front, on a list too (`crates/nvs-stdlib/src/arr.rs:5302`,
`:5340`), because the array has no way to name its last slot. `slice` walks from the first slot to its
start (`arr.rs:3340`), so a late page costs its whole distance from the front.

## Standing decisions

**The rule, the protocol, the ABI, the moves and the lends are `in-place-everywhere`'s** and are used
as they are. This goal changes `apply` bodies and the representation only.

**The representation (my call).** A packed list keeps a front offset: dropping its first k entries
releases them and moves the offset, and prepending writes into front room. A prepend with no front room
reallocates with front room equal to the list's length, so both ends grow by doubling. A map keeps the
slot of its first live entry, so `firstKey`, `first` and the start of `foreach` take one step. Front room
and front holes are freed by the compaction that already frees holes, when they outnumber the live
entries. **What it spends:** a list that was prepended to holds up to twice its entries, and a map holds
one more word. A list that is never prepended to holds nothing more. Home: `rule:types/arrays` and the
decisions at the top of `crates/nvs-runtime/src/array.rs`.

**A read finds its place without a walk (my call, from the review of 2026-11-17).** The array names its
last live slot: one step on a list, and a walk backward past the holes at its end on a map, which
compaction bounds. A list with no hole names the slot of a position in one step. `last`, `lastKey`, the
end `withoutLast` looks for and the start of a `slice` window use them, on a shared subject as on a sole
one. A stack read with `last` and cut with `withoutLast` is then O(1) for each step, and paging a list
with `slice` costs each page its own length. **What it spends:** nothing held. A map's first and last
live slot are found from what it already keeps.

**What each member does with a sole subject (my call).** Every owning member, all 33:

- **Removes entries in place:** `slice`, `replaceRange`, `withoutFirst`, `withoutLast`, `withoutNull`,
  `filter`, `unique`, `diff`, `intersect`. A renumbering `slice` of a packed list moves its front offset.
  `withoutFirst` on a packed list keeps its keys, so it becomes a map once and is O(1) after that.
- **Adds entries in place:** `append`, `prepend`, `appendAll`, `padStart`, `padEnd`, `overlay`,
  `overlayDeep`, `underlay`.
- **Reorders or rewrites in place:** `reverse`, `sort`, `sortByKey` (a permutation from `check`), `values`
  (a no-op on a list), `map` (the results from `check`, written into the existing slots), `mapKeys` and
  `flip` (the new keys from `check`, the index rebuilt in the same table).
- **Build, because the result keeps no entry of the subject in its place:** `chunk`, `column`, `groupBy`,
  `countBy`, `keys`, `flatten`, `flattenDeep`, `fillKeys`, `fromKeysAndValues`. Each releases the
  subject once it has read it. `from` takes no array subject, so it owns none.
- **The adding members' `check` (2026-11-18).** It validates and retains what `apply` stores: the
  values, the layers (`Retained::layers`), or the padding value with its count, which
  `nvs_runtime::affordable` already allowed. `apply` cannot throw, so a growth the memory limit refuses
  there is recorded and ends the request, as `prepend` already did. A layer that is the subject makes
  the subject shared. `overlay`, `overlayDeep` and `underlay` copy a shared subject with its append
  counter (`combine`), so a shared and a sole subject give the same result. `overlayDeep` still copies
  each nested map it merges into.

On a shared subject, every member builds only what it keeps, so `slice($big, 0, 10)` touches ten entries.
A member that rewrites a shared subject copies the table in one block, never key by key.

**How stage 2 built it (my call, 2026-11-18).** The packed list is a `VecDeque`: its head is the
front offset, and it doubles at either end. A cut list frees its room once the room is more than three
times its entries, so a list cut a step at a time holds up to four times its entries. Every in-place
operation takes positions in iteration order, never slots, and calls `make_unique` first, which is a
no-op on the sole subject every caller passes. A renumbered result is a list whose append counter is
its length. A result that keeps its keys keeps the subject's counter on a map, and on a list cut only
at its end the counter is the new length, so `withoutLast` keeps a stack a list. A stage 3 member that
must give a fresh build's counter on a map recomputes it.

**A kept key keeps the append counter (my call, 2026-11-18).** A member whose result keeps the
subject's keys gives it the subject's append counter, as `unset` does, on a sole subject and on a shared
one alike. A list cut only at its end stays a list, and appends at its new length. `append` alone sets
the counter a fresh build of its entries gives, which its conformance case pins. So `slice($a, 2, 2,
{preserveKeys: true})` of a five-entry list appends at `5`, not at `4`. Why: the in-place cut keeps the
counter at no cost, and a recount on a map is a walk per call that would make a queue quadratic. The
build path matches it through `kept_counter` in `crates/nvs-stdlib/src/arr.rs`, so the result does not
depend on who else holds the subject. **What it spends:** nothing.

**Lanes.** Stage 2 touches `crates/nvs-runtime/src/array.rs` alone and can run as a side goal at any
time, even before `in-place-everywhere`. Stage 3 needs `in-place-everywhere` reached.

## Stages

### 1. the ladders show every quadratic array shape

**Split:** one writing subagent writes the fourteen ladders, copying the shape of
`benches/scaling/arrays/sort.nvs`.

- [x] Three ladders under `benches/scaling/arrays/` for the forms `in-place-everywhere` added on
  2026-11-17, each `// scaling: expect linear`, start 1000, max 32000:
  `in-place-a-list-built-with-a-spread.nvs` (`$list = [...$list, $x]`),
  `in-place-a-list-built-by-reduce.nvs` (a `Core\Arr::reduce` callback that returns
  `Core\Arr::append($carry, $x)`) and `in-place-a-list-built-inside-a-generator.nvs` (a generator that
  appends to a local and then yields). The spread ladder is linear. The other two call `append`, so
  they are red until stage 3.
- [x] Two ladders for the reads, each `// scaling: expect linear`, start 1000, max 32000:
  `read-the-last-entry-of-a-growing-list.nvs` (`Core\Arr::last` once per entry) and
  `read-a-list-page-by-page-with-slice.nvs` (`Core\Arr::slice($a, $at, 50)` over the whole list). They
  are red until stage 3.
- [x] Nine ladders under `benches/scaling/arrays/`, each `// scaling: expect linear`, start 1000, max
  32000: `in-place-a-queue-in-a-local-with-without-first.nvs`,
  `in-place-a-queue-in-a-static-property-with-slice.nvs`,
  `in-place-a-stack-in-an-object-property-with-without-last.nvs`, `in-place-a-queue-inside-a-try.nvs`,
  `in-place-a-list-built-with-append.nvs`, `in-place-a-list-built-with-prepend.nvs`,
  `in-place-a-list-in-an-element-built-with-append.nvs` (`$rows['a'] = Core\Arr::append($rows['a'], $v)`),
  `in-place-a-map-drained-by-first-key.nvs` (`unset` of `Core\Arr::firstKey` on string keys),
  and `in-place-a-window-kept-by-slice-of-append.nvs` (`$w = Core\Arr::slice(Core\Arr::append($w, $v), -50)`).
  They are red until stage 3. A member that reads every entry (`filter`, `map`, `sort`) is O(n) per call
  by its nature, so no ladder loops it; `every-member` checks that it keeps its storage instead.

### 2. a list has room at both ends, and a map finds its first live entry in one step

- [x] `crates/nvs-runtime/src/array.rs:276` `Shape::Packed` gains its front offset. Every reader of the
  packed `Vec` (`key_at`, `value_at`, `next_slot`, the separation at `:921`, the size accounting at
  `:760`, `:785` and `:806`) reads through it.
- [x] `Hashed` keeps the slot of its first live entry, updated by `unset` (`:1879`) and by compaction
  (`:1082`). `next_slot(0)` (`:1930`) starts there.
- [x] `NvsArray` gains the in-place operations the members call, each on a sole array only: drop the
  first k, drop the last k, keep a window (renumbered or with its keys), prepend values, permute by a
  plan, keep by a mask, rewrite values, and rekey. Each states its cost in its doc comment.
- [x] `NvsArray` gains two readers beside `next_slot` (`:1930`): the last live slot, and the slot of a
  position. The second answers in one step for a list with no hole and walks otherwise. Each states
  its cost in its doc comment.
- [x] The four tests the `runtime-room` check names and the two the `runtime-reads` check names, beside
  `:4269` `compaction_preserves_order_and_the_append_counter`, run once by name.

### 3. every Core\Arr member changes only what its result changes

**Gate:** `in-place-everywhere` is reached.

- [x] The removing members, in `crates/nvs-stdlib/src/arr.rs` order: `slice` (`:3334`), `replaceRange`
  (`:3384`), `withoutFirst` (`:3581`), `withoutLast` (`:3619`), `withoutNull`, `filter`, `unique`,
  `diff`, `intersect`.
  - [x] `slice`, `withoutFirst` and `withoutLast` keep or release in place, and build on a shared
    subject with [`kept_counter`]'s counter.
  - [x] `replaceRange`, `withoutNull`, `filter`, `unique`, `diff`, `intersect`.
- [x] The adding members: `append` (`:3514`), `prepend` (`:3548`), `appendAll`, `padStart`, `padEnd`,
  `overlay`, `overlayDeep`, `underlay`.
  - [x] `append` (recounts, then pushes) and `prepend` (front room) change a sole subject in place.
  - [x] `appendAll`, `padStart`, `padEnd`, `overlay`, `overlayDeep`, `underlay`.
- [x] The reordering and rewriting members: `reverse`, `sort`, `sortByKey`, `values`, `map`, `mapKeys`,
  `flip`.
  - [x] `reverse`, `sort` and `sortByKey` through `apply_permutation`.
  - [x] `values` (a sole list as it is), `map` (`rewrite_values`), `mapKeys` and `flip` (`rekey`).
- [x] `crates/nvs-stdlib/tests/in_place.rs`: `every_arr_member_has_a_row_here` reads every owning
  `Core\Arr` row and fails naming one with no row in the test's table. Each row says whether the member
  keeps its storage or builds. `every_arr_member_keeps_what_it_does_not_change` checks each row on a
  sole subject. `a_shared_subject_builds_only_the_window_it_keeps` counts the entries a shared `slice`
  touches. Run once by name.
- [x] The reads that walked use stage 2's two readers: `last` (`:5302`), `lastKey` (`:5340`), the end
  `withoutLast` looks for (`:3619`) and the start of `slice`'s window (`:3340`).
- [x] `bun nv scaling --check arrays/in-place-` and `bun nv scaling --check arrays/read-`, once each.
  Every ladder grows linearly.
  - [x] `arrays/read-`: both flat (the `read-ladders` check passes).
  - [x] `arrays/in-place-`: 12 flat. A typed array check reads the array's tag word (`ec7eb683fb`).
  - [x] `in-place-a-list-in-an-element-built-with-append.nvs`: a local array's element is lent.
  - [x] Both checks once more after the rest of stage 3's members: 12 and 2 flat.

### 4. the goal is verified once, and every failure is fixed

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
- [x] Run `looper verify` once more, and it is green.

## Handoff

### State

The person's calls (2026-11-17): fix the cause, nothing for a developer to remember, and every
replacement in place. Every other choice in § *Standing decisions* is a session's call.

Every stage is done, and `looper verify` is green over the whole goal. The gate found three things:

- `padStart` and `padEnd` swallowed the allocator's refusal of their run and returned a short array.
  Their `check` now builds the run with a fallible reserve and throws (`ecc6d5abd9`), so both
  `count-shaped-producers-*` conformance cases pass as written.
- The testing chapter's bench example counts 16 more bytes per op (`272cf2c732`), and an ambiguous
  rustdoc link to `nvs_core_arr_append` is fixed (`a387d0ac41`).
- `filter`'s verdict mask is one allocation per call, so its bench declares 10 (`55d45ade4e`). Every
  stale `Core\Arr` and testing perf record is measured again on the release binary.

`bun nv verify --doc`, `bun nv owners --closes arrays-in-place` and `bun nv ci --check` are green.
The sweep's `env-secret-proofs` red is stale: no log, and nothing defines it.

### Next

1. The driver runs the acceptance checks and retires the goal.

### Backlog

- Moved to `strings-in-place`'s backlog: `overlayDeep` copies each nested map it merges into.
