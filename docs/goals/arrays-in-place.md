---
looper: 1
title: "every Core\\Arr member keeps the entries it does not change where they are"
summary: "Reached when a list has room at both ends, a map finds its first live entry in one step, every Core\\Arr member that owns its subject changes only what its result changes, and every array ladder that cuts, extends or rebuilds one array a step at a time is linear."
context: [docs/rules/types/a-replaced-value-is-changed-in-place.md, docs/rules/types/arrays.md, crates/nvs-runtime/src/array.rs]
stages:
  - id: 1
    title: the ladders show every quadratic array shape
    status: open
    context: [benches/scaling/README.md, benches/scaling/arrays/sort.nvs]
  - id: 2
    title: a list has room at both ends, and a map finds its first live entry in one step
    status: open
  - id: 3
    title: every Core\Arr member changes only what its result changes
    status: open
    context: [crates/nvs-stdlib/src/arr.rs, crates/nvs-stdlib/tests/in_place.rs]
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: open
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

**What each member does with a sole subject (my call).** Every owning member, all 34:

- **Removes entries in place:** `slice`, `replaceRange`, `withoutFirst`, `withoutLast`, `withoutNull`,
  `filter`, `unique`, `diff`, `intersect`. A renumbering `slice` of a packed list moves its front offset.
  `withoutFirst` on a packed list keeps its keys, so it becomes a map once and is O(1) after that.
- **Adds entries in place:** `append`, `prepend`, `appendAll`, `padStart`, `padEnd`, `overlay`,
  `overlayDeep`, `underlay`.
- **Reorders or rewrites in place:** `reverse`, `sort`, `sortByKey` (a permutation from `check`), `values`
  (a no-op on a list), `map` (the results from `check`, written into the existing slots), `mapKeys` and
  `flip` (the new keys from `check`, the index rebuilt in the same table).
- **Build, because the result keeps no entry of the subject in its place:** `chunk`, `column`, `groupBy`,
  `countBy`, `keys`, `flatten`, `flattenDeep`, `fillKeys`, `fromKeysAndValues`, `from`. Each releases the
  subject once it has read it.

On a shared subject, every member builds only what it keeps, so `slice($big, 0, 10)` touches ten entries.
A member that rewrites a shared subject copies the table in one block, never key by key.

**Lanes.** Stage 2 touches `crates/nvs-runtime/src/array.rs` alone and can run as a side goal at any
time, even before `in-place-everywhere`. Stage 3 needs `in-place-everywhere` reached.

## Stages

### 1. the ladders show every quadratic array shape

**Split:** one writing subagent writes the nine ladders, copying the shape of
`benches/scaling/arrays/sort.nvs`.

- [ ] Nine ladders under `benches/scaling/arrays/`, each `// scaling: expect linear`, start 1000, max
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

- [ ] `crates/nvs-runtime/src/array.rs:276` `Shape::Packed` gains its front offset. Every reader of the
  packed `Vec` (`key_at`, `value_at`, `next_slot`, the separation at `:921`, the size accounting at
  `:760`, `:785` and `:806`) reads through it.
- [ ] `Hashed` keeps the slot of its first live entry, updated by `unset` (`:1879`) and by compaction
  (`:1082`). `next_slot(0)` (`:1930`) starts there.
- [ ] `NvsArray` gains the in-place operations the members call, each on a sole array only: drop the
  first k, drop the last k, keep a window (renumbered or with its keys), prepend values, permute by a
  plan, keep by a mask, rewrite values, and rekey. Each states its cost in its doc comment.
- [ ] The four tests the `runtime-room` check names, beside `:4269`
  `compaction_preserves_order_and_the_append_counter`, run once by name.

### 3. every Core\Arr member changes only what its result changes

**Gate:** `in-place-everywhere` is reached.

- [ ] The removing members, in `crates/nvs-stdlib/src/arr.rs` order: `slice` (`:3334`), `replaceRange`
  (`:3384`), `withoutFirst` (`:3581`), `withoutLast` (`:3619`), `withoutNull`, `filter`, `unique`,
  `diff`, `intersect`.
- [ ] The adding members: `append` (`:3514`), `prepend` (`:3548`), `appendAll`, `padStart`, `padEnd`,
  `overlay`, `overlayDeep`, `underlay`.
- [ ] The reordering and rewriting members: `reverse`, `sort`, `sortByKey`, `values`, `map`, `mapKeys`,
  `flip`.
- [ ] `crates/nvs-stdlib/tests/in_place.rs`: `every_arr_member_has_a_row_here` reads every owning
  `Core\Arr` row and fails naming one with no row in the test's table. Each row says whether the member
  keeps its storage or builds. `every_arr_member_keeps_what_it_does_not_change` checks each row on a
  sole subject. `a_shared_subject_builds_only_the_window_it_keeps` counts the entries a shared `slice`
  touches. Run once by name.
- [ ] `bun nv scaling --check arrays/in-place-` once. Every ladder grows linearly.

### 4. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Written on 2026-11-17 from a conversation with the person, and narrowed the same day when
`in-place-everywhere` took the protocol, the ABI, the moves and the lends. Their calls: fix the cause,
nothing for a developer to remember, and every replacement in place. Every other choice in
§ *Standing decisions* is the authoring session's call. Nothing is built yet.

### Next

Stage 1, and stage 2 as a side goal at any time. Stage 3 waits for `in-place-everywhere`.

### Backlog
