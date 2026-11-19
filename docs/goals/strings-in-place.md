---
looper: 1
title: "every Core\\Str and Core\\Bytes member keeps the bytes it does not change where they are"
summary: "Reached when a position is found by walking only to it, a string can drop its front without moving the rest, every Core\\Str and Core\\Bytes member that owns its subject changes only what its result changes, and every string ladder that cuts, extends or rebuilds one string a step at a time is linear."
context: [docs/rules/types/a-replaced-value-is-changed-in-place.md, docs/rules/types/string-is-utf8.md, crates/nvs-runtime/src/string.rs]
stages:
  - id: 1
    title: the ladders show every quadratic string shape
    status: done
    context: [benches/scaling/README.md, benches/scaling/strings/build.nvs]
  - id: 2
    title: a forward position reads only the characters before it, and a backward one only the characters after it
    status: done
    context: [crates/nvs-stdlib/src/granularity.rs, crates/nvs-stdlib/src/str.rs]
  - id: 3
    title: a string can drop its front without moving the rest
    status: done
    context: [docs/rules/types/bytes.md]
  - id: 4
    title: every Core\Str and Core\Bytes member changes only what its result changes
    status: done
    context: [crates/nvs-stdlib/src/str.rs, crates/nvs-stdlib/src/bytes.rs, crates/nvs-stdlib/tests/in_place.rs]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: positions
    stage: 2
    name: "a forward position never counts the whole string, a backward one walks from the end, and both agree with the old walk on every window"
    run: [cargo, test, --lib, --, a_forward_position_never_counts_the_whole_string, a_backward_position_walks_from_the_end, both_walks_agree_on_every_window]
    timeout: 1800
    tests:
      - a_forward_position_never_counts_the_whole_string
      - a_backward_position_walks_from_the_end
      - both_walks_agree_on_every_window
  - id: front-room
    stage: 3
    name: "a sole string drops its front in place, keeps its grapheme count, resets its digest, and an append reclaims the dropped front before it grows"
    run: [cargo, test, --lib, --, a_sole_string_drops_its_front_in_place, a_front_cut_keeps_a_known_grapheme_count, an_in_place_change_resets_the_digest, an_append_reclaims_the_dropped_front_before_it_grows]
    timeout: 1800
    tests:
      - a_sole_string_drops_its_front_in_place
      - a_front_cut_keeps_a_known_grapheme_count
      - an_in_place_change_resets_the_digest
      - an_append_reclaims_the_dropped_front_before_it_grows
  - id: every-member
    stage: 4
    name: "every owning Core\\Str and Core\\Bytes member has a row, and on a sole subject each one keeps the bytes it does not change in the same allocation"
    run: [cargo, test, --test, in_place, --, every_string_member_has_a_row_here, every_string_member_keeps_what_it_does_not_change]
    timeout: 1800
    tests:
      - every_string_member_has_a_row_here
      - every_string_member_keeps_what_it_does_not_change
  - id: ladders
    stage: 4
    name: "every in-place string ladder grows linearly"
    run: [bun, nv, scaling, --check, strings/in-place-]
    timeout: 3600
---

# every Core\Str and Core\Bytes member keeps the bytes it does not change where they are

## Why

`in-place-everywhere` makes the compiler hand every replaced string to its member as the string's only
reference, and makes concatenation and interpolation append in place on every target. This goal makes
each `Core\Str` and `Core\Bytes` member use that.

Measured on the release binary on 2026-11-17, each doubling of the input makes these about 4× slower:

| Shape | 32k | 64k |
|---|---|---|
| `$rest = Core\Str::slice($rest, 1)` on a local or a static property | 20 ms | 86 ms |
| a line reader, `$rest = Core\Str::after($rest, "\n") ?? ''`, 16-byte lines | 137 ms | 597 ms |
| `$s = Core\Str::slice($s, 0, -1)` | 22 ms | 76 ms |
| `$s = Core\Str::padEnd($s, $i, 'x')` | 20 ms | 84 ms |
| `$s = "{$s}x"` on a local | 11 ms | 38 ms |
| `self::$buf = self::$buf . 'x'`, and `$this->text = "{$this->text}x"` | 10 ms | 41 ms |
| `$b = Core\Bytes::slice($b, 1)` | 10 ms | 40 ms |

A 1 MB request body read line by line costs 0.6 s. The concatenation rows are `in-place-everywhere`'s
stage 5. The causes this goal fixes:

1. **A forward position counts the whole string first.** `byte_of_signed_index_of`
   (`crates/nvs-stdlib/src/granularity.rs:317`) calls `length_of` before it looks at the sign. On a fresh
   string, with no cached count, that segments every character. So `slice($rest, 1)` is O(n) even
   without the copy.
2. **A backward position walks forward from the start** (`granularity.rs:331` into `:344`), so
   `slice($s, 0, -1)` is O(n) even when the count is cached.
3. **A string cannot drop its front.** The payload starts at `PAYLOAD_OFFSET` (`string.rs:227`), so the
   only way to cut the front is to copy the rest.
4. **Every member builds a fresh string** (`str.rs:3280` `produced`).

## Standing decisions

**The rule, the protocol, the ABI, the moves, the lends and the concatenation are
`in-place-everywhere`'s** and are used as they are.

**A string gets a start word (my call).** `StrHeader` (`string.rs:189`) gains `start`, the number of
dropped bytes between `PAYLOAD_OFFSET` and the first live byte. Compiled code never reads a payload,
only `len` (`crates/nvs-codegen/src/emit.rs:3740`). So the cost is one addition in the runtime's payload
read (`string.rs:1054`) and nothing in compiled code. `cap` keeps counting from `PAYLOAD_OFFSET`. An
append that needs room first moves the live bytes back to the start when the dropped front is at least
half of `cap`. Otherwise it grows as today, so a string used as a queue (append at the back, cut at the
front) stays within twice its live bytes. An immortal literal has `start` 0 and is never changed.
**What it spends:** 8 bytes per string allocation. A string that is only cut never holds more than its
allocation already held. Safe fallback, if the word measurably slows the string ladders or the member
benches: split the grapheme-count word into a 32-bit count and a 32-bit start. A count above
`u32::MAX` is then not cached, and a start that would pass it moves the bytes back first. Home: the
`# Layout` decision at `string.rs:12` and `rule:types/string-is-utf8`. `bytes` shares the layout
(`rule:types/bytes`).

**An in-place change keeps the caches honest (my call).** It resets the digest. It updates a known
grapheme count by the units it removed or added when the member counted them, and resets it otherwise.

**Positions (my call).** A non-negative position walks forward from the start and never needs the
total. A negative one walks backward from the end (`grapheme_indices(true).rev()`, or `char_indices`
backward for code points) and reads only the units it passes. `length_of` is read only where a result
needs the total. This is a plain bug fix, and it also speeds up every shared `slice` call.

**A long one-byte walk fills the count (my call, stage 2).** Without a cached count, a loop over
every position of an ASCII string would scan each prefix and be quadratic, which `at.nvs` and
`find-next.nvs` measure. So a walk whose one-byte part reaches `COUNT_PAST` (4096 bytes,
`granularity.rs`) also fills the cached count once. A position nearer either end reads only up to
it. A walk over wider units never counts, because a count cannot shorten it. `Value::known_grapheme_count`
reads the count without filling it. **What it spends:** no memory.

**What each member does with a sole subject (my call).** Every owning member, all 24:

- **Cuts in place** (moves `start` or shortens `len`): `Core\Str::slice`, `at`, `before`, `after`,
  `trim`, `trimStart`, `trimEnd`, and `Core\Bytes::slice`.
- **Extends in place** (writes into `cap`, growing like an append): `padEnd`, `repeat`,
  `Core\Bytes::repeat`. `padStart` writes into the dropped front when there is room, and moves the bytes
  once otherwise.
- **Rewrites in its own allocation:** `replaceRange`, `replace`, `replaceAll`, `lower`, `upper`,
  `upperFirst`, `lowerFirst`, `fold`, `normalize`, `reverse`, `wrap`. A forward pass writes over the
  subject while the result is never longer than the input up to each point. Where the result grows, the
  member writes into a scratch buffer and swaps it in, reusing the subject's allocation when it fits.
- **Builds:** `format`, whose subject is a template that the result does not keep in place.

On a shared subject every member builds only what it keeps.

**An extending member plans its room in `check` (my call, stage 4).** `apply` cannot throw, and a
count off the call site must still throw where the allocator refuses it. So `check` asks
`Value::room_behind` or `Value::room_in_front` (`Room::Fits` or `Room::Fresh(cap)`). With `Fits`,
`apply` writes a `Run` (a piece copied n times, then a prefix of one more) into the subject with
`NvsStr::extend`, `prepend_run` or `repeated`. With `Fresh`, `check` builds the result fallibly at
that capacity, which is an append's doubling for a sole subject. The padding is retained before the
room is read, so a padding that is the subject makes it shared. `padEnd` keeps the subject's count
only for an ASCII padding with no `\r`, because only `\r\n` joins two ASCII characters.
**What it spends:** nothing beyond an append's doubling.

**A join led by a string consumes that string (my call, stage 4).** `$s = "{$s}row {$i}"` was
quadratic: a `Concat` with a number, placeholder or html piece consumed nothing, so the ownership
pass could not move `$s` in. Now `InstKind::consumes` is `ConcatPart::leads_with_str`, the lowering
hands the leading piece over as for an all-string join, and codegen calls `nvs_str_append_parts` with
it as the target. **What it spends:** nothing. A shared leading piece is copied once, as before.

**A rewriting member plans in `check` and writes in place only where no walk runs (my call, stage
4).** `str.rs`'s `Rewritten` is the plan, and `rewritten` the `apply`. `check` reads whether the
subject is sole with `room_behind(0)`, after it retains any argument whose bytes it would write. On a
sole subject, an ASCII `lower`/`upper`/`fold`/`reverse` (no `\r\n`), and a `replaceRange` or
`upperFirst`/`lowerFirst` whose new bytes are no longer than the old, go through `NvsStr::rewrite`
and allocate nothing. Every other rewrite, `replace` and `replaceAll` included, builds its result in
`check` and `apply` copies it into the subject with `NvsStr::replace_with` when it fits. `replace`
is not done in place, because its search polls the request's deadline through a `Walk`, which can
throw, and `apply` cannot. **What it spends:** on a sole subject, one copy of a built result. On a
shared subject, nothing beyond the build it did before.

**Lanes.** Stage 2 touches `crates/nvs-stdlib/src/granularity.rs` and `str.rs`'s `window` alone. Stage 3
touches `crates/nvs-runtime/src/string.rs` alone. Both can run as side goals at any time, even before
`in-place-everywhere`. Stage 4 needs `in-place-everywhere` reached and stages 2 and 3 done.

## Stages

### 1. the ladders show every quadratic string shape

**Split:** one writing subagent writes the nine ladders, copying the shape of
`benches/scaling/strings/build.nvs`.

- [x] Nine ladders under `benches/scaling/strings/`, each `// scaling: expect linear`, start 4000, max
  64000: `in-place-a-text-cut-from-the-front-in-a-local.nvs`,
  `in-place-a-text-cut-from-the-front-in-a-static-property.nvs`,
  `in-place-a-body-read-line-by-line.nvs` (`Core\Str::after`),
  `in-place-a-text-cut-from-the-back.nvs` (`slice($s, 0, -1)`),
  `in-place-a-text-built-with-pad-end.nvs`, `in-place-a-text-built-by-interpolation.nvs`,
  `in-place-a-property-built-by-interpolation.nvs`,
  `in-place-a-static-property-built-with-equals-and-concatenation.nvs` and
  `in-place-bytes-cut-from-the-front.nvs`. Text with characters outside ASCII goes in at least the line
  reader and the back cut, so the grapheme path is measured too. They are red until stage 4.

### 2. a forward position reads only the characters before it, and a backward one only the characters after it

- [x] `crates/nvs-stdlib/src/granularity.rs:317` `byte_of_signed_index_of`: a non-negative index goes
  straight to `byte_of_index` (`:344`) with no `length_of`. A negative index walks backward from the end.
  The clamp at the start stays.
- [x] `crates/nvs-stdlib/src/str.rs:3233` `window`: a negative length walks backward from the end too,
  and never reads the total.
- [x] Every other caller of `byte_of_signed_index_of` and `length_of` in `crates/nvs-stdlib/src/` reads
  the total only where its result needs it. An `Explore` subagent lists the callers first.
- [x] The three tests the `positions` check names, in `granularity.rs`'s test module.
  `both_walks_agree_on_every_window` compares the new walk with the old one over every offset and
  length from -12 to 12 on texts with combining marks and emoji. Run once by name.

### 3. a string can drop its front without moving the rest

- [x] `crates/nvs-runtime/src/string.rs:189` `StrHeader` gains `start`. `immortal_header_bytes` (`:384`)
  writes 0. Every payload read (`:1054`, `:1163`, `:1478`, `:2001`, `:2102`, `:2197`, `:2216`) adds it.
- [x] `NvsStr` gains the in-place operations the members call, each on a sole string only: drop the
  first k bytes, drop the last k bytes, append bytes, prepend into the dropped front, rewrite forward,
  and swap in a buffer. Each resets the digest and keeps or resets the grapheme count as § *Standing
  decisions* says.
- [x] `nvs_str_append` (`:1989`) and `nvs_str_append_parts` (`:2062`) reclaim the dropped front before
  they grow, under the half-of-`cap` rule. `grown_capacity` (`:529`) is unchanged.
- [x] `docs/rules/types/string-is-utf8.md` gains the start word, the reclaim rule and what each spends.
  `docs/rules/types/bytes.md` says `bytes` shares the layout. `bun nv rules --render`, then
  `bun nv rules --check` is green.
- [x] The four tests the `front-room` check names, in `string.rs`'s test module. Run once by name.

### 4. every Core\Str and Core\Bytes member changes only what its result changes

**Gate:** `in-place-everywhere` is reached, and stages 2 and 3 are done.

- [x] The cutting members: `slice` (`str.rs:3277`), `at`, `before` (`:3775`), `after` (`:3794`), `trim`
  (`:2596`), `trimStart` (`:2622`), `trimEnd` (`:2632`), `Core\Bytes::slice` (`bytes.rs:938`).
- [x] The extending members: `padEnd` (`:3970`), `padStart`, `repeat`, `Core\Bytes::repeat`.
- [x] The rewriting members: `replaceRange` (`:3361`), `replace` (`:3101`), `replaceAll` (`:3176`),
  `lower` (`:4288`), `upper` (`:4312`), `upperFirst` (`:4327`), `lowerFirst` (`:4336`), `fold`
  (`:4365`), `normalize` (`:4398`), `reverse` (`:3876`), `wrap` (`:3902`).
- [x] `crates/nvs-stdlib/tests/in_place.rs`: `every_string_member_has_a_row_here` reads every owning
  `Core\Str` and `Core\Bytes` row and fails naming one with no row in the test's table.
  `every_string_member_keeps_what_it_does_not_change` checks each row on a sole subject. Run once.
  - [x] `TEXT_ROWS` names all 24 owning members, and `every_string_member_has_a_row_here` passes.
  - [x] The cutting rows pass `every_string_member_keeps_what_it_does_not_change`.
  - [x] The extending rows pass it.
  - [x] The rewriting rows pass it.
- [x] `bun nv scaling --check strings/in-place-` once. Every ladder grows linearly.

### 5. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

The goal came from a conversation with the person, who asked for the same depth check on strings
that arrays got, for nothing a developer has to remember, and for every replacement in place. Every
other choice in § *Standing decisions* is the authoring session's call.

Stages 1 to 4 are done. Stage 4: the cutting members (`3543cf1dea`), the extending members
(`bc6704ff15`) and the rewriting members (this session) are done. The `ladders` check is green: all
nine ladders are linear. `cargo test --test in_place -- every_string_member` ran once and passes.

- **Rewriting** (this session): `str.rs` has `Rewritten` (`:4319`), `sole` (`:4359`) and
  `rewritten` (`:4369`), the shared `apply` of all 11 members. `map_first` (`:4827`) and
  `case_mapped` (`:4863`) return a plan. See § *Standing decisions*. The module docs of `str.rs` and
  `bytes.rs` now say which members own their subject.

- **Extending** (`bc6704ff15`): `crates/nvs-runtime/src/string.rs` has `Room` (`:687`), `room_for`,
  `Run` (`:732`), `NvsStr::prepend_run` (`:1454`), `extend` (`:1531`) and `repeated` (`:1608`), and
  `value.rs` has `Value::room_behind`/`room_in_front`. In `str.rs`, `padStart`/`padEnd` plan with
  `padded` (`:4098`), `repeat` and `Core\Bytes::repeat` with `repeat_plan` (`:4231`), and `extended`
  (`:4066`) is the `apply`. See § *Standing decisions*.
- **Interpolation** (`668d5f1c4e`): the two interpolation ladders grew, because a join with a number
  hole consumed nothing. A join led by a string now appends onto it (§ *Standing decisions*).
The sweep's `env-secret-proofs` red is stale: it has no log, and `git log -S` finds only goal and
handoff commits.

### Next

Stage 5, the gate. Start `looper verify` with `run_in_background`, end the turn, and fix every
failure it names in one pass. Then the goal-end gates of AGENTS.md § *In the loop*: `bun nv verify
--doc`, `bun nv owners --closes strings-in-place`, `bun nv ci --check`. Every `Core\Str` member's perf
record is stale, because `str.rs` changed: record them as the playbook's *perf: stale* bullet says,
after every doc edit in `str.rs`.

Possibly red at the gate:

- `crates/nvs-stdlib/tests/allocation_policy.rs`: pad and repeat on a sole subject now allocate an
  append's doubling or nothing, and the rewriting members on a sole subject allocate nothing or keep
  the subject's allocation.
- IR or codegen tests that print a `concat` led by a string with a number piece, which now has no
  `release` of its leading piece after it.
- A test that expects `normalize` of ASCII, or `replace` with an empty search, to return a new
  string. They now return the subject itself.

### Backlog

- From `arrays-in-place`: `Core\Arr::overlayDeep` on a sole subject still copies each nested map it
  merges into. A nested map that is itself sole could be changed in place. No ladder shows it yet.
