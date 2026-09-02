# Handoff

## State

**Stage 9's trace half is in.** ADR 0067 § 11's span is `crates/nvs-db/src/span.rs`'s `QuerySpan`
— driver, connection name, truncated SQL, duration, rows returned, rows affected — opened by
`start_statement` before the batch goes out, counted by `next_row`, ended by whatever ends the
stream, and readable as `PgRows::span()`. **The "never parameters" half is a signature rather than
a rule**: `QuerySpan::opened` is handed the SQL and never the values, so there is nothing in scope
for a later field or a later driver to leak, and `Debug` is derived so a field added later joins the
rendering the test asserts over. That module's doc owns the rest, including what it spends.

**Nothing consumes a span yet, and that is the next group.** ADR 0041's event kind is unbuilt —
`nvs_runtime::TraceEvent` has `callee` and `status` and no kind at all, its own doc saying every
event is a `call` — so § 11's sink, its `slow_query` threshold and `executeMany`'s own span are all
waiting on the same one thing.

**Stage 9's other two items are blocked three deep and are not work a session can take.** ADR 0067
§ 10's literal `Db::open` host and ADR 0058's tainted `Settings` host both need `Core\Db::open` to
have a registry row at all, then a way for the intrinsic table to name a *field of a shape* rather
than an argument position, and the host-against-grants half additionally needs a capability set in
front of the checking pass, which no capability has today. Known gap 6 of
`crates/nvs-types/src/intrinsics.rs` owns this; the two test names stay open in `loop-goal.toml`.

**Stage 2's `local_infile_is_refused_and_no_file_is_sent` still fails acceptance and always will**
until MySQL's driver exists — unchanged, permanent, backlog and not work.

**`orient.py` did not print** `crates/nvs-runtime/src/ctx.rs` (the trace record the next group
edits) or ADR 0041 at all: `[context] modules` wants an `nvs-runtime/src/ctx.rs` pattern, `rules`
wants `0041`, and `adrs` wants ADR 0067 § 11 beside § 10.

## Next group

**Closing § 11 end to end — one file set: `crates/nvs-runtime/src/ctx.rs`,
`crates/nvs-stdlib/src/db.rs` and `crates/nvs-db/src/pg.rs`, with `crates/nvs-db/src/span.rs`
already loaded by the third.**

- [ ] **`TraceEvent` gains ADR 0041's kind** — `call`/`gc`/`spawn`/`query`, the fourth being the
      one ADR 0067 § 11 asks for. The record is `crates/nvs-runtime/src/ctx.rs:1174` and its own
      doc comment already names the four; the three constructions to widen are
      `crates/nvs-runtime/src/ctx.rs:3307`, `crates/nvs-runtime/src/ctx.rs:4827` and
      `crates/nvs-runtime/src/ctx.rs:4831`, and every one of them is a `call`.
- [ ] **`Core\Db`'s query path files the span as a `query` event**, naming the block with
      `QuerySpan::named` — the driver cannot, per `crates/nvs-db/src/span.rs:110`. The two call
      sites that hold a `PgRows` are `crates/nvs-stdlib/src/db.rs:3474` and
      `crates/nvs-stdlib/src/db.rs:3760`; the helper body is
      `crates/nvs-stdlib/src/db.rs:3409`.
- [ ] **`executeMany` gets its own span** — it answers with a count and no handle, so the span is
      built and finished inside `crates/nvs-db/src/pg.rs:2824`'s `execute_many` and filed by
      `crates/nvs-db/src/pg.rs:682`'s caller rather than borrowed out.

## Backlog

- ADR 0067 § 10's literal host and ADR 0058's tainted host — blocked; `crates/nvs-types/src/intrinsics.rs` known gap 6.
- `Core\Db::open` needs a registry shape *parameter* — `crates/nvs-stdlib/src/db.rs` known gap 1.
- § 10's unterminated string literal is a doc disagreement, not a scan — `crates/nvs-types/src/intrinsics.rs` known gap 5.
- § 11's `slow_query` threshold per connection block — `crates/nvs-db/src/span.rs` module doc.
- Stage 2's `local_infile_is_refused_and_no_file_is_sent` waits on MySQL's driver — `docs/agent/loop-goal.toml`.
- Stage 5 is four of seven — `docs/plan/m8.md`.
