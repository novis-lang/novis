---
looper: 1
title: "`Core\\Arr` orders keys by a list, and removes `null` with a narrowed type"
summary: "Core\\Arr::sortByKey takes a keys option that puts the listed keys first in the listed order and the others after them in their current order, Core\\Arr::withoutNull takes an array<?T> and returns an array<T> with every null entry removed and every other key kept, a Core signature that takes ?T binds T to the argument without null, both have their feature proofs and their reference cards, and the checks below pass."
context: [crates/nvs-stdlib/src/arr.rs]
stages:
  - id: 1
    title: "a `Core` signature's `?T` binds `T` without `null`, and `withoutNull` returns `array<T>`"
    status: done
    context: [crates/nvs-types/src/generics.rs, crates/nvs-stdlib/src/registry.rs, docs/rules/types/arrays.md]
  - id: 2
    title: "`sortByKey` takes a `keys` option"
    status: done
    context: [docs/examples/core/Arr/sortByKey/about.md, tests/conformance/core/arr-sort-by-key-orders-keys-bytewise-and-takes-a-comparator.nvst]
  - id: 3
    title: "both members have their feature proofs, and every stale figure and page is current"
    status: done
    context: [docs/examples/core/Arr/withoutFirst/about.md, benches/members/core/Arr/withoutFirst.nvs, tests/hostile/core/Arr/withoutFirst/01-copies-written-to-break-the-walk.nvs]
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: without-null-proofs
    stage: 3
    name: Core\Arr::withoutNull owes no feature proof
    run: [bun, nv, proofs, --gate, --only, "Core\\Arr::withoutNull"]
  - id: sort-by-key-proofs
    stage: 3
    name: Core\Arr::sortByKey owes no feature proof
    run: [bun, nv, proofs, --gate, --only, "Core\\Arr::sortByKey"]
  - id: owners-closes
    stage: 3
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, arr-key-order-and-nulls]
    reach: true
    floor: false
---

# `Core\Arr` orders keys by a list, and removes `null` with a narrowed type

## Why

Two common array jobs have no direct member today, and the workarounds are slow or loose.

- **Putting keys in a given order.** A program moves `'a'` in front of `'1'` and `'2'` in every row,
  or orders a record's fields by a list of column names. Today that is
  `Core\Arr::overlay(Core\Arr::fillKeys($order, ''), $row)`. It works, but a listed key the row lacks
  stays in the result with the placeholder value. The other way is `sortByKey` with a `comparator`,
  which calls back into Novis code `n log n` times
  (`crates/nvs-stdlib/src/arr.rs:4737`).
- **Removing `null`.** `Core\Arr::filter` calls a callback once per entry
  (`crates/nvs-stdlib/src/arr.rs:2386`), and neither it nor `diff($a, [null])` narrows the type: the
  result of either over `array<?string>` is still `array<?string>`, so the caller writes
  `as array<string>` after it.

Both new forms run natively with no callback, in one pass.

## Standing decisions

**`keys` is an option of `sortByKey`, not a new member.** `rule:types/arrays` says only the sort
members reorder, and that they say so in their names. The option is
`keys: array<int|string>`, typed like `fillKeys`' parameter (`crates/nvs-stdlib/src/arr.rs:554`), and
an `int` key is its decimal string as everywhere else. Its default follows `comparator`'s
`Const::Null` (`crates/nvs-stdlib/src/arr.rs:2199`) unless `Const` already has an empty-array value.

**What `keys` does is stated as a sort, so it obeys every rule the other options do.** Each listed key
ranks by its first place in the list. Every key not in the list ranks after all listed keys, and those
keys count as equal to each other. Equal keys keep the order they were added in, as in every sort. So
by default the listed keys come first in the listed order, and the others follow in their current
order. `order: Desc` reverses the comparison, as it already does (`crates/nvs-stdlib/src/arr.rs:4764`):
the unlisted keys come first in their current order, then the listed keys from last to first. A
listed key the array does not have is skipped. A key listed twice keeps its first place.

**`keys` and `comparator` together throw a `LogicError` at the call.** Each one says how keys are
ordered, so a call with both has two answers. A checker diagnostic would need an option-exclusivity
mechanism that no `Core` options bag has. Fallback: if a session finds such a mechanism already in
the checker, it uses that and records it here.

**`keys` never sorts.** The implementation builds one map from key to rank for the list (`k` entries),
emits the listed keys the array has by lookup, then walks the array once for the others. That is
O(n + k), with no comparison and no callback. **Memory:** that map, per call, freed when the call
returns.

**`withoutNull(array<?T> $a): array<T>` is a new member beside `withoutFirst` and `withoutLast`**
(`crates/nvs-stdlib/src/arr.rs:422`). It removes every `null` entry and keeps every other key, as
`filter` and `withoutFirst` do. It does not renumber, and `Core\Arr::values` does that. It removes
`null` only: no "empty" member is added, because whether `0`, `'0'` or `[]` count as empty has no one
answer, and `diff($a, [null, ''])` already names exactly what it removes. **Memory:** one result array
per call, as `filter` makes.

**The narrowed type comes from binding, not from a new `CoreTy` variant.** Today a declared `null|T`
against an actual `null|string` binds `T` to `null|string`, because the union arm pairs each declared
member with the whole actual type (`crates/nvs-types/src/generics.rs:304`). The new rule: when both
sides are unions, the declared union's members that are not type variables are removed from the actual
union, and the variable binds to what remains. `null|T` against `null|int|string` binds `T` to
`int|string`. Against a non-union `string`, the existing arm already binds `T` to `string`. The
module doc of `generics.rs` is where this rule is written, beside first-binding-wins. No `Core` row
takes a `?T` parameter today, so no existing signature changes its answer. A user class that
implements a generic interface at a `?T` position can; the gate stage's `nvs-types` tests show it.
The rule applies only where the declared union has exactly one bare variable and every other member
mentions no variable (`narrowed_pair` in `generics.rs`). `array<T>|Iterable<T>|Iterator<T>`, or a
union of two bare variables, keeps the old pairing, because which member the remainder belongs to
has no one answer there. When nothing is left after the removal, the old pairing applies too.

**`withoutNull` over an `array<mixed>` returns `array<mixed>`.** `mixed` is not a union, so `T` binds
to `mixed`. That is correct: the static type cannot say more. The parameter then substitutes to
`array<null|mixed>`, so a union target with a `mixed` member accepts what `mixed` does
(`assignable` in `crates/nvs-types/src/expr/assign.rs`). Before that, a `mixed` source was refused
ahead of the union arm, and the call did not compile.

**Trade-offs.** Performance: both forms are one native pass with no callback, where today's ways call
back into Novis code per entry or per comparison. Memory: one rank map of `k` entries per `keys` call,
and one result array per call of either, all freed at return. Usability: `withoutNull` returns the
narrowed type, so no `as` follows it. Simplicity: one option and one member, and one binding rule in
the checker that every future `?T` parameter uses.

**Lanes.** Stages 1 and 2 both edit `crates/nvs-stdlib/src/arr.rs` and run in order. Stage 1's
`generics.rs` change is disjoint from everything else in the chain. The whole goal is disjoint from
the registry goals ahead of it on the chain and can run as a side goal.

## Stages

### 1. a `Core` signature's `?T` binds `T` without `null`, and `withoutNull` returns `array<T>`

**Does:** `null|T` against a union binds `T` to the rest of the union, and `Core\Arr::withoutNull`
removes every `null` entry and returns `array<T>`.

- [x] The union-against-union arm in `bind`: the declared union's non-variable members are removed from the actual union, and the variable binds to what remains, with first-binding-wins kept. `crates/nvs-types/src/generics.rs:298`
- [x] The module doc of `generics.rs` states the rule and why. `crates/nvs-types/src/generics.rs:1`
- [x] Rust tests in `generics.rs`: `null|T` against `null|int|string` binds `T` to `int|string`, against `string` binds `string`, and against `mixed` binds `mixed`. `crates/nvs-types/src/generics.rs:488`
- [x] The `withoutNull` row, `params: &[CoreTy::Array(&CoreTy::Nullable(&CoreTy::Var("T")))]` and `return_ty: CoreTy::Array(&CoreTy::Var("T"))`, after `withoutLast`. `crates/nvs-stdlib/src/arr.rs:431`
- [x] `nvs_core_arr_without_null`: one walk, skipping each `Tag::Null` value, keeping every other key with `store_at`, and its arm in `address`. `crates/nvs-stdlib/src/arr.rs:3512`, `crates/nvs-stdlib/src/arr.rs:2208`
- [x] Its reference card, `WITHOUT_NULL_DOC`, in the shape of `WITHOUT_LAST_DOC` (`rule:core-api/reference-card`). `crates/nvs-stdlib/src/arr.rs:1259`
- [x] A conformance case: `array<?string>` in, the result assigned to an `array<string>` variable with no `as`, keys kept, and `null` gone. `tests/conformance/core/arr-without-null-removes-null-and-narrows-the-type.nvst:1`

### 2. `sortByKey` takes a `keys` option

**Does:** `sortByKey($a, { keys: [...] })` orders keys as the standing decision says, natively.

- [x] The `keys` option in `SORT_BY_KEY_OPTIONS`, and its doc comment rewritten for three options. `crates/nvs-stdlib/src/arr.rs:2182`
- [x] `nvs_core_arr_sort_by_key` takes the fourth argument: with `keys`, the rank map and the two emitting walks; with `keys` and `comparator`, a `LogicError` that names both options. `crates/nvs-stdlib/src/arr.rs:4687`
- [x] The reference card `SORT_BY_KEY_DOC` names `keys` (`rule:core-api/reference-card`). `crates/nvs-stdlib/src/arr.rs:1461`
- [x] A conformance case: listed keys first in list order, the rest in their current order, a missing listed key skipped, a key listed twice, an `int` key, and `order: Desc`. `tests/conformance/core/arr-sort-by-key-puts-listed-keys-first.nvst:1`
- [x] A conformance case: `keys` with `comparator` throws `LogicError`. `tests/conformance/core/arr-sort-by-key-refuses-keys-with-a-comparator.nvst:1`

### 3. both members have their feature proofs, and every stale figure and page is current

**Does:** `withoutNull` has its about page, examples, bench and attack. `sortByKey`'s page and one
example show `keys`. Every perf figure and website page the change made stale is current again.

**Split:** one writing subagent writes `withoutNull`'s about page, three examples and bench, in the
shape of `withoutFirst`'s, held to § *Text an end user reads* in `AGENTS.md`. A second, side by side
because its files are disjoint, writes `sortByKey`'s about-page paragraph and the `keys` example. The
session writes both attacks, then records perf.

- [x] `withoutNull`: about page, three examples (a form's optional fields, a lookup that returned `null` for some ids, and the keys that stay), and the bench. `docs/examples/core/Arr/withoutNull/about.md:1`, `benches/members/core/Arr/withoutNull.nvs:1`
- [x] `withoutNull`'s attack: copies of the subject written while the walk runs do not change its result. `tests/hostile/core/Arr/withoutNull/01-copies-written-to-break-the-walk.nvs:1`
- [x] `sortByKey`'s about page names `keys`, and one example moves a key to the front of every row of a table. `docs/examples/core/Arr/sortByKey/about.md:1`, `docs/examples/core/Arr/sortByKey/04-some-keys-first.nvs:1`
- [x] `sortByKey`'s attack gains a step: a `keys` list far longer than the array, and one that lists every key many times. `tests/hostile/core/Arr/sortByKey/01-comparators-written-to-break-the-sort.nvs:1`
- [x] `bun nv proofs --record-perf` over every feature `bun nv proofs --gate --limit 0` names stale, then that gate passes. `benches/members/core/Arr/sortByKey.nvs:1`
- [x] `bun nv site --check stale questions` passes: each page and question that covers a changed member is reread and stamped. `website/site.lock.json:1`
- [x] `bun nv owners --closes arr-key-order-and-nulls` passes. `docs/goals/arr-key-order-and-nulls.md:1`

### 4. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stages 1, 2 and 3 are done and committed. Both members have every feature proof, and
`bun nv proofs --gate --limit 0` owes nothing across the whole roster. `withoutNull` has a Rust test
(`without_null_removes_null_and_keeps_every_other_key` in `crates/nvs-stdlib/src/arr.rs`). All 57
`Core\Arr` perf figures are recorded again, and `bun nv site --check stale questions` and
`bun nv owners --closes arr-key-order-and-nulls` pass. `core.json` and `llms-full.txt` are rendered.

Stage 3's attack found that `withoutNull` over an `array<mixed>` did not compile (E0401, expected
`array<null|mixed>`). `assignable` in `crates/nvs-types/src/expr/assign.rs` now accepts any value
into a union with a `mixed` member. The conformance case for `withoutNull` covers it.

Expected red at the gate: any `nvs meta --json` golden in `crates/nvs-cli/tests/meta.rs` or registry
count that lists `Core\Arr`'s members, and any hover `.lspt` or stub golden that quotes
`sortByKey`'s card.

### Next

**Stage 4: the gate**, as its `**Split:**` line says.

- [ ] Start `bun nv affected --run` with `run_in_background`, end the turn, and group its failures by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Check first the `assign.rs` change's reach: `nvs-types` tests and every conformance case that writes `?mixed`. `crates/nvs-types/src/expr/assign.rs:212`
- [ ] Then the goal-end gates: `bun nv verify --doc`, `bun nv ci --check`. `docs/goals/arr-key-order-and-nulls.md:1`

### Backlog

- Inlining an arrow function passed to a `Core\Arr` callback member is a separate question, not part of this goal.
