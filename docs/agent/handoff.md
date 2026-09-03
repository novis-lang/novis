# Handoff

## State

**Goal 6, Stage 2: a request runs, and it is goal 2's `Isolate`.** `nvs-server` answers an h1
request by running one isolate on the connection's own coroutine — one isolation path, which is
what keeps M7's state-bleed suite a parameterisation of one mechanism. Three unit tests in
`crates/nvs-server/src/serve.rs` drive it: one answer, two answers over one kept-alive connection,
and a request whose isolate fails answering `500` without dropping the connection.

**On disk.** `serve_connection` (`crates/nvs-server/src/serve.rs:174`) takes the connection task's
`Ctx` and a handler that answers an `Isolate`; `answer` turns the `Completion` into the response —
`Completion::output` *is* the body (ADR 0088 § 3) and `discard_value` discharges the returned
value's one reference, which is why this crate can keep `unsafe_code = "forbid"`. **The isolate
parks inside `hyper`'s poll and that is legal**, because ADR 0138 § 1 drives the connection future
on the coroutine's own stack; `serve_connection`'s doc comment is the home of that reading, of why
the `RefCell` has one borrower by construction, and of the fail-closed `500` with no body.

**Nothing user-reachable starts it, and the handler is still the caller's function.** There is no
`nvs serve` — `crates/nvs-cli/src/main.rs:141`'s `Command` has no arm — no mount table, and no
deadline on a connection (ADR 0074 § 5). The seam a real handler binds to is
`nvs_runtime::script::resolve` (`crates/nvs-runtime/src/script.rs:231`), which turns a path into the
`Program` an `Isolate` carries; nothing in `nvs-server` calls it yet.

**The driver's acceptance sweep is truncated, and it is not a regression.** `native
examples/upload.nvs` is checked against stage 5's frozen `want`. The playbook's bullet on a
`loop-goal.toml` fixture check frozen ahead of the frontier owns it, including why rewriting the
fixture to print those strings is the wrong fix.

## Next group

**A connection you can bound and a server you can start.** One file set:
`crates/nvs-server/src/serve.rs`, `crates/nvs-server/src/io.rs`, `crates/nvs-cli/src/main.rs`,
`crates/nvs-config/src/tree.rs`. `[context] adrs` gained `0074 §5` for this group; add `0097 §3`
when the mount slice starts if § 4 alone does not settle the wildcards.

- [ ] **A connection is bounded by a clock.** ADR 0074 § 5's waits, read off `[server]`
      (`crates/nvs-config/src/tree.rs:756`) and enforced through `crates/nvs-server/src/io.rs:91`'s
      `stream_mut` around the drive in `crates/nvs-server/src/serve.rs:174`, with the same
      parameter reaching `crates/nvs-server/src/serve.rs:254`'s accept loop. **Before** the CLI
      slice: a server that can be started with no deadline is the one shape ADR 0074 refuses.
- [ ] **`nvs serve` starts one core and runs the loop.** The subcommand joins
      `crates/nvs-cli/src/main.rs:141`'s `Command` and reads
      `crates/nvs-config/src/tree.rs:756`'s `[server]`; ADR 0093's installer sink already names
      `serve` as one of the two subcommands it will accept, so the spelling is fixed.
- [ ] **The handler selects a mount rather than constructing a path.** ADR 0097 § 4's five steps in
      front of `crates/nvs-server/src/serve.rs:174`'s handler, turning the selected entry into a
      `Program` with `crates/nvs-runtime/src/script.rs:231`'s `resolve`. § 2 is the rule it keeps:
      the table is expanded at boot and a request only ever selects from it.

## Backlog

- ADR 0074 § 2's secure headers and closed CORS: nothing sets a response header yet.
- A failure rendered into a development response — ADR 0092 § 3, once there is a mode to ask.
- The request body is never read, so a request carrying one ends its connection — ADR 0105.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024 *Revisiting*, narrowed by m7.md.
- `max_in_flight` as an arithmetic over cores — ADR 0106 amending ADR 0097 § 5.
- Stage 9's state-bleed suite over the shared `Isolate` — `docs/plan/m7.md` *Verify*.
