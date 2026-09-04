# Handoff

## State

**Goal 6, Stage 5: ADR 0105 § 3's two consumers landed on `Core\Request\Part`.**
`content(): Iterable<tainted bytes>` answers a third handle class —
`Core\Request\PartContent` (`crates/nvs-stdlib/src/request.rs:654` is the row block), whose
iterate/advance/current pull `crates/nvs-stdlib/src/multipart.rs`'s `next_chunk` — and
`readAll({max?}): tainted bytes` is that same pull with a `Vec` and a bound around it,
which is `body`'s relationship to `bodyStream` one level in.

**§ 3's identity is checked, in one place.** `part_parse`
(`crates/nvs-stdlib/src/request.rs:1817`) compares the part's `PART_ORDINAL` stamp against
`Multipart::opened()` and refuses a part the walk has moved past with a `LogicError`; both
consumers and `content()`'s own `advance()` go through it. A stale part still reads back
its own three declarations, which is why handing it the *current* part's bytes was the
failure worth a check.

**Two bounds, and which applies is what the call says** — a bare `readAll()` is held to
`[limits] request_body`, a call naming `max` to that number, and a `max` over the request's
`[limits] memory` is refused **at the call** rather than clamped. The member's own doc
comment is the argument, against ADR 0106 § 13's opposite choice for a boot. `max: 0` is the
absent option; `READ_ALL_OPTIONS` (`crates/nvs-stdlib/src/request.rs:773`) says why a
sentinel is what lets the two bounds be told apart at all.

**A size is a `uint` of bytes, not `"200M"`.** ADR 0105 § 3 writes the prose spelling; the
argument is a count, because a size in text is a second parser in front of a number. Pinned
by `tests/conformance/core/a-file-parts-read-all-bound-is-a-count-of-bytes-in-a-closed-bag.nvst`.

**`examples/upload.nvs` — the failing acceptance check — stays failing**, unchanged by this
session: the check wants `parts=2` / `field=title` out of a *program* leg, and a program
answering no request cannot call `files()` at all. That is still the open question below.

## Next group

**§ 4's delegation, then the example.** One file set: `crates/nvs-stdlib/src/request.rs`,
`crates/nvs-stdlib/src/io.rs`, `examples/upload.nvs`, `docs/agent/loop-goal.toml`.

- [ ] **`Part::saveTo(string $path, {max?, overwrite?})`** — ADR 0105 § 4, a delegation
      rather than an implementation: `Core\IO::writeStream` already writes an
      `Iterable<bytes>` to a path under exactly that pair of options
      (`crates/nvs-stdlib/src/io.rs:2724`, its bag at `crates/nvs-stdlib/src/io.rs:426`).
      Row and card go beside `readAll`'s at `crates/nvs-stdlib/src/request.rs:654` and
      `crates/nvs-stdlib/src/request.rs:773`; the pull and § 3's identity check are
      `crates/nvs-stdlib/src/request.rs:1817`'s `part_parse`, already written for two
      callers. The path is a `CoreTy::Text(Qual::Sink)` while `filename()` is `tainted`,
      so a case pinning that `saveTo($part->filename())` refuses is § 2's whole point
      asserted where it bites.
- [ ] **`examples/upload.nvs` and the acceptance check** — `docs/agent/loop-goal.toml:3385`.
      Settle first whether that stdout can come out of a program leg at all: every reader
      behind `files()` refuses where there is no request, so the check may be naming
      something no `.nvs` program can print — the playbook's "a `loop-goal.toml` check can
      name something that is not a test at all" bullet is the same shape. If it is, fix the
      check, and fix `docs/agent/goals/<goal>.toml` in the same commit or `goal-switch.py`
      restores it.

## Backlog

- `Core\Request::post()` reads the fields the walk buffers — `crates/nvs-stdlib/src/multipart.rs`'s
  `fields()` still carries a `dead_code` allow naming that member.
- ADR 0105 § 5's `upload_total` is the door's, in `nvs_server::body`; `docs/plan/m7.md`'s *Verify*
  wants the bounded-resident-memory assertion over a body far larger than any in-memory bound.
- `clientIp`/`scheme`/`host`/`mount`/`route` are `crates/nvs-stdlib/src/request.rs`'s named gaps.
- **A `[context]` gap:** ADR 0105 §§ 3-4 were not in the orientation pack and had to be sliced by
  hand. Add `0105 §3` and `0105 §4` to `[context] adrs` in `docs/agent/loop-goal.toml`.
