---
milestone: M8
---
# Loop goal 27 — A record names the line it came from, and a repeat is bounded at the sink that suffers

Every developer-facing record says **where it was produced** — file, line, and enclosing member — so
a reader can open an editor at the right place instead of guessing from a message.
`Throwable::$location` stops being the empty string it is set to today, filled from that same datum
rather than a second spelling of it. And a request that logs in a loop stops being able to fill the
disk that a request which *faults* in a loop already cannot, because the log target gains the
coalescing window the engine floor has had all along — sized for interleaved application logging
rather than for a fault repeating one record.

It follows goal `encoder-cycles` because both close a gap in shipped surface found in one review and neither
shares the other's file set, and it precedes everything that would inherit the gap: `Envelope.source`
is what [0163](../../decisions/0163.md)'s viewer exists to show, and it is filled nowhere.

Goal `encoder-cycles`'s whole acceptance list is this goal's floor, and it is never traded.

## Why here

Nothing ahead of it is a dependency — the model declares both envelope fields and a call-site
constant already reaches the runtime on the unwind path.

## What is wrong today, in one line each

Read out of the tree rather than inferred.

1. **`nvs_render::Source` is defined, named by `rule:errors/diagnostic-record`, and constructed
   nowhere.** `Envelope.source` is `Option<Source>` and every `Source {` in the workspace belongs to
   `nvs-db`'s unrelated bind-source type. Stage 2.
2. **`Throwable::$location` is written as the empty string when the object is built**
   (`crates/nvs-runtime/src/throwable.rs`, the `LOCATION_SLOT` store). A declared, readable property
   that always answers `""` looks as though it says something. Stage 2.
3. **`Envelope.count` is filled only by the floor.** `crates/nvs-runtime/src/floor.rs` owns the
   window; `Core\Log::write` renders the same record to the program's own stream and passes it
   entirely. Stage 3.
4. **The floor's window is one slot**, and its own comment says why that is enough *there* — a fault
   loop repeats one record. Application code interleaves, so the same single slot coalesces nothing.
   Stage 3.

## Stage 0 — the catch-up, and how the datum reaches a producer

Nothing on disk contradicts either rule; what exists is the empty-string store in item 2, which
stage 2 replaces rather than edits around.

The open question this stage answers before stage 2 writes anything: **what a producer reads to know
its call site.** The pattern already exists on the unwind path — `nvs_ir`'s `Terminator::Propagate`
carries a `frame: String` (`crates/nvs-ir/src/ir.rs`), `crates/nvs-codegen/src/emit.rs` materialises
it as static bytes in the unit's own data, and `nvs_trace_push` takes it as a pointer and a length.
This stage reads that label and answers one question: does it already carry file, line and member, or
does a producer need a sibling constant of its own? The answer is written down either way, because it
decides stage 2's shape and is the one thing a session must not guess at.

It does **not** answer it by adding a current-location word to `Ctx`
([0165](../../decisions/0165.md) § *Alternatives rejected*): that is a store on every statement to
serve the rare statement that produces a record.

## Stage 1 — the floor

Goal `encoder-cycles`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded for
anything above it.

## Stage 2 — the keystone: one construction, two readers

File set: `crates/nvs-stdlib/src/{log,debug}.rs` and `crates/nvs-runtime/src/throwable.rs`, plus
whatever stage 0 decided the constant is.

1. **The call-site constant**, in the shape stage 0 settled — a compile-time value, read at the call
   that produces a record and never maintained as running state, so no path that produces no record
   pays for it (`rule:errors/propagation`'s cost, unmoved).
2. **`crates/nvs-stdlib/src/debug.rs:@record_of`** — fill `Envelope.source` on the record it builds.
3. **`crates/nvs-stdlib/src/log.rs`** — the same on `Core\Log::write`'s record.
4. **`crates/nvs-runtime/src/throwable.rs`** — the `LOCATION_SLOT` store takes the same datum instead
   of `""`. One construction with two readers: two spellings of "where" that could disagree would be
   worse than one that was missing.
5. **A producer with no source to give omits the field**, on the envelope's existing rule that an
   absent field is omitted rather than rendered empty.

No stack is captured, at any producer. Where a trace is active the record already carries `span_id`
and the trace already has the call events, which is where "how did execution get here" is answered.

## Stage 3 — the log target's window

File set: `crates/nvs-runtime/src/floor.rs` and `crates/nvs-stdlib/src/log.rs`.

The floor keeps its single slot and its `COALESCING_WINDOW` unchanged. The log target gets the same
mechanism with a **small fixed table** and a trivial eviction, so interleaved records coalesce where
one slot would catch none of them — memory stays a constant, just a larger one.

The identity is the floor's `key`: `ts`, `request_id`, `trace_id`, `span_id` and any existing `count`
cleared before hashing, everything else counting, `source` included — two identical messages from two
lines are two facts. A record that differs is written immediately and never held behind a window.

**Nothing here touches the debug stream.** [0165](../../decisions/0165.md) § 3 sends that the other
way: its bound is an index rather than a disk, so it stores every occurrence and groups at read time.
Coalescing it at the sink for symmetry would throw away exactly what makes a group expandable.

## Stage 4 — the rulebook

`rule:errors/a-record-names-where-it-was-produced` and
`rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers` move from `designed` to `shipped` and
their `guardedBy` names the cases stages 2 and 3 landed.

## Standing decisions

- **A constant read at the call, never a current-location word in `Ctx`**
  ([0165](../../decisions/0165.md) § *Options considered*). The second spends every statement to
  serve the rare one.
- **No captured stack, at any producer** ([0165](../../decisions/0165.md) § 2). The call path is the
  trace's answer. Do not add a frame walk because PHP's `getTrace` has one.
- **One datum for `Envelope.source` and `Throwable::$location`** ([0165](../../decisions/0165.md)
  § 1), not two constructions that agree today.
- **The debug stream is not coalesced** ([0165](../../decisions/0165.md) § 3). If stage 3 looks like
  it wants to be general, that is the moment to re-read the section, not to generalise.
- **A per-call-site rate limit is not in scope** ([0165](../../decisions/0165.md) § 6). It is the
  only thing that would make a hot loop cheap rather than quiet, and it drops records that differ;
  taking it is a new record, and nothing has been measured that asks for it.
- **This goal opens no ADR number.** [0165](../../decisions/0165.md) is already accepted and its
  `changes:` block names both rules this goal ships.
- **If stage 0 finds the existing frame label already carries file, line and member**, stage 2 reuses
  it rather than adding a second constant, and the finding is written into the module doc that owns
  it. If it finds the label is a bare member name, the sibling constant is this goal's and not a
  reason to stop.
