# Handoff

## State

**M4's Stage 8, depth.** The tree is at **796 conformance plus 189 differential**. `python
tools/gaps.py --coverage` ranks by the median cases per member; `Core\Time\Zone` left the frontier
this session — all three of its floor-1 members (`system`, `fixed`, `offsetAt`) gained a depth case,
so the class no longer ranks thin at all. Nothing is blocked.

The three cases landed are the invariance, the both-sided bound and the step-function shapes over
one class:

- **`time-zone-system-is-asserted-by-invariance-never-by-value.nvst`** — `system()` reads the host,
  so no conformance case can name its answer. Sixteen calls are each asked for their offset at four
  instants (64 of 64) and for the id they render (16 of 16); `$d->zone()` is shown to give the same
  zone back and to rebuild a byte-identical reading; the offset is shown to be an ordinary one by
  round-tripping it through `Zone::fixed`, the member that throws on either of its bounds. The one
  host class this cannot assert the id on — no IANA name, where `VV` prints `UTC` — is named in the
  case's own helper rather than folded in silently.
- **`time-zone-fixed-bounds-its-offset-on-both-sides-and-at-both-ends-of-a-second.nvst`** — both of
  `fixed`'s bounds, each with its last accepted and first refused value: ±93599 accepted and ±93600
  refused (with `86400` shown to be in the middle, not at the edge, which is where a member bounded
  at ±24:00 would stop), and a nanosecond either side of a whole second refused including across
  zero. Ten values around the range bound are swept and counted 6/4, six fractional durations
  counted 6/6. `XXX` renders `+25:59:59` as `+25:59`, so every assertion is made of `offsetAt`.
- **`time-zone-offset-at-is-a-step-function-with-one-step-at-the-transition.nvst`** — the member
  takes an instant because its answer is a step function of one. The step is asked at nanosecond
  resolution at both of Berlin's 2024 transitions, at Lord Howe's *half*-hour one, and at Berlin's
  pre-1893 `+3208` local mean time, which is neither an hour nor a minute. Sixteen instants are then
  counted constant inside their windows (8/8 each), the same sixteen counted constant for three
  zones that have no transitions (16 each), and the offset shown to be exactly the shift `in`
  applies (16 of 16).

## Next group

**`Core\Time`'s own floor** — `gaps.py --coverage` ranks it at median 3.0 with a floor of 2, and its
three thinnest members are all in the file set this session already worked:
`crates/nvs-stdlib/src/time.rs` plus `tests/conformance/core/`. Check what is already on disk first:
`ls tests/conformance/core/ | grep -E 'epoch|monotonic|sleep'` and `sed -n '2p'` over the hits.

- [ ] **`Core\Time::monotonic` is only ever compared with itself** (`time.rs:1648`) — one case. It
      is a reading like `system()` is, so the shape is the same one: assert by invariance, not by
      value. It never goes backwards over a sweep of readings, its difference from an earlier
      reading is non-negative, and it is *not* the wall clock — a monotonic reading has no epoch and
      no zone, so `in`/`toIso` over it are the members it does not have.
- [ ] **`Core\Time::sleep`'s bound at zero and at its own resolution** (`time.rs:1667`) — one case.
      The last duration it accepts and the first it refuses (a negative one), and the agreement that
      matters: a `sleep(d)` moves `monotonic` by at least `d`, counted over a sweep of small
      durations, never asserted as an exact elapsed time.
- [ ] **`Core\Time::fromEpoch` inverts every epoch reading** (`time.rs:1683`) — one case. The
      inverse-pair shape: `fromEpoch` undoes `toEpochSeconds`/`Millis`/`Micros`/`toEpochNanos` over
      a sweep including the epoch itself, a negative instant and a sub-second remainder, plus the
      `{nanos:}` option's own bound.

## Backlog

- `Core\Bytes`, `Core\Encoding` and `Core\Test` are the next thinnest classes at median 3.0 —
  `python tools/gaps.py --coverage`.
- `Core\Str::fold`/`graphemes`/`indexOf` and `Core\Arr::column`/`flattenDeep`/`overlayDeep` are the
  only floor-1 members left in the tree — `gaps.py --coverage`.
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
- `Core\Time\Zone`'s id has no reader of its own: a program reads it through
  `$i->in($z)->format("VV")`, which prints `UTC` for a fixed zone — noted here because a case
  wanting the stored spelling has no way to get it.
