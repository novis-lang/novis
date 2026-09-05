# Handoff

## State

**ADR 0020 § 6's log envelope is whole.** A record written inside a request carries `ts`,
`request_id` and — for a *sampled* trace, which is § 6's "whenever a trace is active" —
`trace_id` and `span_id`; a CLI run still writes `level` and `msg` alone, which is what keeps
the addition additive. `request_id` is ADR 0076 § 2's trace id, that section having made it
Novis's only request identifier. The one home for which key comes from what is
`nvs_runtime::Ctx::stamp_envelope` (`crates/nvs-runtime/src/ctx/output.rs`), and **both** of
§ 6's writers call it — `Core\Log::write` from `nvs_stdlib::log`'s `record`, and
`nvs_runtime::floor::report` after `key` has taken the coalescing window's key, which is why it
is a call of its own rather than something `write_log_record` does. `nvs-runtime` gained a
`jiff` edge so the tree has one RFC 3339 renderer, not two.

Stage 5's statement timeout is landed, all three slices; `crates/nvs-stdlib/src/db/mod.rs`'s own
§ *A statement's `timeout` is a deadline on the socket* is that decision's home.

Nothing is blocked on a decision. The next acceptance failure the driver reports outranks the
group below.

## Next group

**What `crates/nvs-stdlib/src/db/mod.rs`'s known gaps still owe on § 18's roster.** One file set:
`crates/nvs-stdlib/src/db/mod.rs`, `crates/nvs-stdlib/src/db/registry.rs`,
`crates/nvs-stdlib/src/db/stream.rs`, `crates/nvs-db/src/pg.rs`.

- [ ] **`stream`'s `{chunk?: uint}`, the other half of gap 6.**
      `crates/nvs-stdlib/src/db/mod.rs:248` states it: a chunk size has to reach the `Execute`
      that asks for a row count, and `crates/nvs-db/src/pg.rs:584`'s `stream` asks for one row.
      The registry rows are `crates/nvs-stdlib/src/db/registry.rs:348` and
      `crates/nvs-stdlib/src/db/registry.rs:633` — both, for the playbook's reason — and the
      member body is `crates/nvs-stdlib/src/db/stream.rs:228`. Decide first whether a chunk is
      worth a second `Execute` shape at all: § 4 promises constant memory, which one row already
      gives, so the honest outcome may be to record the refusal in gap 6 rather than to add the
      option.
- [ ] **`serverVersion`, gap 5.** `crates/nvs-stdlib/src/db/mod.rs:237` is the inventory: no
      driver keeps the server's own version string, so the member cannot be written until
      PostgreSQL's `server_version` `ParameterStatus`, MariaDB's greeting, TDS's `LOGINACK` and
      SQLite's library version are each held on the connection. `nvs_db::mysql` already parses
      one into a `(u16, u16, u16)` for its own capability decisions —
      `crates/nvs-db/src/pg.rs:584` is the neighbouring driver's own statement path.

## Backlog

- `[context] adrs` for this goal is missing **ADR 0020 § 6** — the log envelope's own
  specification. Stage 6's check cites it and `orient.py` printed 0076 § 6 alone, so this
  session sliced it by hand. Add `0020` `§6` to `docs/agent/loop-goal.toml`'s manifest.
- `scope = "fleet"` parses, boots and is not armed — `crates/nvs-server/src/schedule.rs`
  § *What is not armed*, ADR 0073 § 3.
- A cycle whose only closing edge is inside an `array<T>` survives `object::sweep` —
  `crates/nvs-runtime/src/object.rs` § *The five walks*, ADR 0116 § 2.
- `Core\Server::traceId()` is a known gap in `crates/nvs-stdlib/src/server.rs`, and the id it
  would read is now stamped on every record — `TraceContext::trace_id_hex` is the spelling.
- ADR 0133 § 3's computed `$reason` is not refused — `crates/nvs-stdlib/src/html.rs`
  § *Known gaps*, blocked on a full diagnostic band.
