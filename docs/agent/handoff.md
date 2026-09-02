# Handoff

## State

**ADR 0067 § 11's span now reaches a trace, and stage 9's trace half is closed except for the
two items below.** `nvs_runtime::TraceKind` is ADR 0041 § 1's `call`/`gc`/`spawn`/`query`
(`crates/nvs-runtime/src/ctx.rs:1179`), `TraceEvent` carries the tag and nothing else, and
`Ctx::record_query` (`crates/nvs-runtime/src/ctx.rs:3373`) files one. `Core\Db`'s `query`,
`queryAs` and `execute` name the span with their `[db.<name>]` block and file it once the rows
have ended, gated on `DebugFlags::TRACE`.

**The span crosses as its `Display` rendering rather than as fields, and that is the crate
boundary.** `nvs-db` depends on `nvs-runtime`, so a struct in the runtime holding § 11's field
set would be that set's second home; `Ctx::record_query`'s doc comment owns the reasoning and
`crates/nvs-db/src/span.rs`'s module doc owns the rest, including that the gate is
`DebugFlags::TRACE` and not § 11's capability, which no capability set can express yet.

**`gc` and `spawn` have no emitter** — ADR 0041 §§ 2-3 instrument routines this tree has not
reached — and that is stated on `TraceKind`'s own variants rather than here.

**Stage 9's other two items are blocked three deep and are not work a session can take.** ADR
0067 § 10's literal `Db::open` host and ADR 0058's tainted `Settings` host both need
`Core\Db::open` to have a registry row at all, then a way for the intrinsic table to name a
*field of a shape*; known gap 6 of `crates/nvs-types/src/intrinsics.rs` owns this, and the two
test names stay open in `loop-goal.toml`.

**Stage 2's `local_infile_is_refused_and_no_file_is_sent` still fails acceptance and always
will** until MySQL's driver exists — unchanged, permanent, backlog and not work.

**`orient.py` still did not print** `crates/nvs-runtime/src/ctx.rs` or ADR 0041, both of which
this session had to peek: `[context] modules` wants an `nvs-runtime/src/ctx.rs` pattern,
`rules` wants `0041`, and `adrs` wants `0041 § 1` beside ADR 0067 § 11.

## Next group

**§ 11's two open halves and the statements that bypass them — one file set:
`crates/nvs-stdlib/src/db.rs`, `crates/nvs-stdlib/src/queue.rs` and
`crates/nvs-config/src/tree.rs`, with `crates/nvs-db/src/span.rs` read by the first.**

- [ ] **`executeMany` gets its own span** — ADR 0067 § 11, the one statement routine that files
      nothing. It answers with a count and never lends a `PgRows` out, so the routine at
      `crates/nvs-stdlib/src/db.rs:3857` builds the span itself and files it with
      `traced_query`/`ctx.record_query` exactly as `execute` does at
      `crates/nvs-stdlib/src/db.rs:3810`; `crates/nvs-db/src/span.rs:94` is what it builds.
- [ ] **§ 11's `slow_query` threshold** — a per-connection-block duration that writes the same
      facts to `Core\Log` when the span's `duration()` passes it. The field joins the block at
      `crates/nvs-config/src/tree.rs:554` beside `statement_cache`, and the reader is the
      filing site at `crates/nvs-stdlib/src/db.rs:3535`, which already holds the rendered span.
- [ ] **ADR 0084's worker files no `query` event for its own statements** — the queue drives
      `PgRows` directly rather than through `Core\Db`'s members, so a claim, a report and a
      dead-letter are invisible in a trace that shows every application query.
      `crates/nvs-stdlib/src/queue.rs:1315`, `crates/nvs-stdlib/src/queue.rs:1469` and
      `crates/nvs-stdlib/src/queue.rs:1640` are the three sites.

## Backlog

- MySQL's driver, which is what `local_infile_is_refused_and_no_file_is_sent` waits on — ADR
  0067's *Verification* section.
- `Core\Db::open`'s registry row and the shape-field intrinsic behind it — known gap 6 of
  `crates/nvs-types/src/intrinsics.rs`.
- ADR 0041 § 4's speedscope export, and § 2/§ 3's `gc`/`spawn` emitters — that ADR.
- ADR 0018's trace sink, which is what makes `Ctx::trace`'s vector and the text-rendered span
  temporary — `crates/nvs-runtime/src/ctx.rs:899`.
