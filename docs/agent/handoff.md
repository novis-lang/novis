# Handoff

## State

**Goal 6, Stage 5: `Core\Request` exists and answers the request line.** `method`, `path` and
`query` are registered, implemented and pinned by three `.nvst` cases each;
`crates/nvs-stdlib/src/request.rs`'s `//!` owns the twelve members of spec § 15 still owed and
what each waits on. `Core\Http\Method`'s parse lives there, which closes
`crates/nvs-stdlib/src/router.rs`'s known gap 4 — `HEAD` answers `Get` per § 15, so `Head` is a
case `method()` never returns and `isHead` is what will carry it.

**The inbound carrier is `nvs_runtime::Inbound`, behind a `Box` on the context.** It holds the
verb verbatim, the mount-stripped path and the raw query string, and interprets none of the
three: every roster and convention stays in `nvs-stdlib`. `Ctx::inbound()` answers `None` in any
process serving no request, and every member turns that into ADR 0012 § 7's `LogicError` rather
than an empty string. **Nothing writes one yet** — `nvs_server` is the first writer and that is
the third slice below.

**The `Box` is load-bearing, and the ceiling it dodges is the session's real find.**
`corosensei` refuses an entry closure over 1024 bytes, and `nvs_host::scheduler::start`'s closure
captured a whole `Ctx` — 984 bytes at the previous commit, leaving three. Adding *any* field
broke every coroutine in the workspace with `type is too big to transfer`. The context now
crosses boxed and lands back on the coroutine's own stack in the closure's first statement, at
one allocation and one move per **task**; `CORO_TRANSFER_LIMIT`'s doc comment owns the reasoning
and `start` asserts on it. The playbook bullet is the trap.

**`Core\Uri::parseQuery`'s body is now `uri::parse_query`, a free function**, because spec § 9
promises `Core\Request::query` answers the bracket convention *from the same code* rather than a
second implementation of it. `query` parses per call, which `request.rs`'s `//!` states and
prices; a memoized parse belongs on the carrier once it holds more than three strings.

**`[context]` gaps.** `adrs` printed 0074 § 5, 0088 § 3, 0097 §§ 2 and 4 and 0106 § 13; this
slice needed **0012 §§ 2-3 and 7** (the whole of what `Core\Request` replaces and why an isolate
throws) and read them by hand. `spec` still has no selector, so § 15 costs a read a session.
`modules` names no `nvs-host` pattern, and this session had to edit `nvs-host/src/scheduler.rs`
to land anything at all.

## Next group

**`Core\Request`'s inbound headers, and the server that writes them.** One file set:
`crates/nvs-runtime/src/ctx.rs`, `crates/nvs-stdlib/src/request.rs`,
`crates/nvs-server/src/serve.rs`. Take them in this order — the carrier has to hold headers
before a member can read one, and nothing can write a carrier until the last.

- [ ] **`Inbound` carries the request's headers, and `Core\Request::header`/`::headers` read
      them** — spec § 15, ADR 0074 § 1. `crates/nvs-runtime/src/ctx.rs:4361` is `Inbound` and
      `crates/nvs-runtime/src/ctx.rs:4312` is `set_inbound`; `crates/nvs-stdlib/src/request.rs:96`
      is `CLASS` and `crates/nvs-stdlib/src/request.rs:199` is the shared refusal every member
      goes through. A repeated field name is one entry per value, not the last — `Ctx::headers`'
      outbound half made that mistake's opposite decision for the same reason. Both answer
      `tainted string`, which `path` already does and `query` deliberately cannot.
- [ ] **`Core\Request::cookie(name)` reads byte for byte and enforces the prefixes on read** —
      ADR 0095 § 3, spec § 15. `crates/nvs-stdlib/src/response.rs:979` is `HOST_PREFIX` and
      `crates/nvs-stdlib/src/response.rs:1226` the write-side refusal to agree with; the read side
      is the other half of that ADR — a `__Host-` cookie that arrived without conforming is not
      one a browser would have stored, so it is not visible here. Parse the `Cookie` header off
      the carrier; do not add a second field for cookies.
- [ ] **The server writes the carrier, and an unrecognized verb is a `501` at the door** — ADR
      0097 §§ 2 and 4. `crates/nvs-server/src/serve.rs:1076` is where a matched mount becomes an
      `Isolate` and `crates/nvs-host/src/isolate.rs:128` is `Isolate::new`; the request line and
      the headers are already in hand there, and the mount strip is `serve.rs`'s step 2. This is
      what makes every `.nvst` case above a refusal rather than an answer, and closing it is what
      lets the next session pin `method()` returning `Post`.

## Backlog

- `examples/upload.nvs`'s acceptance check still wants `Core\Request::files()` — ADR 0105, and
  the largest single item left in § 15 (`docs/plan/m7.md`'s *Verify*).
- `Core\Request::mount`/`::route` wait on the match `nvs_server` makes once — ADR 0102.
- `Core\Request::clientIp`/`::scheme` need `[server] trusted_proxies` — ADR 0074 § 1.
- `query`'s answer is `mixed`, so `tainted` does not survive it — `request.rs`'s `//!` owns the
  reasoning; the same hole is `Core\Uri::parseQuery`'s.
- `Core\Response::html`/`::sendFile` remain that module's own gaps — ADR 0088 § 4.
- `docs/agent/loop-goal.toml` `[context]`: add `0012 §§ 2-3, 7`, an `nvs-host` module pattern
  and a `spec` selector for § 15.
