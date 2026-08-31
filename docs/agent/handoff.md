# Handoff

## State

**`Core\Http\Client` sends.** `crates/nvs-stdlib/src/http/transport.rs` composes an HTTP/1.1
request, writes it over `nvs_host::net`'s parking stream and fills `Core\Http\Response`'s two
slots. ADR 0074 § 6's retries and ADR 0058 § 4's redirects are both there under one deadline;
`send`'s `repin` closure is the seam that keeps the policy in `crate::http` and out of the socket.
That module's own doc owns the rest — one connection per attempt, no pool, `https` refused until a
trust anchor set has an owner. `nvs-stdlib` now depends on `nvs-host` for the stream.

**`examples/http.nvs` is further from green than "two errors left" said, and none of the four is
this slice's.** Checked, not assumed: (1) `Core\Env` still exists nowhere; (2) `$configured ?? "…"`
types as `string|tainted string` and `Qual::Launder` refuses the union; (3) **`allowUrl` refuses
`http://127.0.0.1:8099` outright** — `nvs_config::capability::denied_by_default` denies loopback
and its own doc says the operator's exception half "is not here yet", so the first two lines throw
before any transport is reached; (4) **nothing in this tree serves `:8099`** — `8099` appears in no
`.rs`, `.py`, `.md` or `.toml` outside the example itself, so stage 5's `exact` check has no origin
to talk to.

## Next group

**What `examples/http.nvs` still needs.** They share that file and the stage 5 `exact` check it
feeds — `docs/agent/loop-goal.toml:2357` — and each is a different owner's half, so take them in
this order and stop where the context runs out.

- [ ] **The operator exception to ADR 0058 § 3's denied table**, so a deployment can name loopback
      and the example's origin is reachable at all. `crates/nvs-config/src/capability.rs:86` is the
      table and its doc names the gap; the grant it reads is `crates/nvs-config/src/capability.rs:60`'s
      `Scope`, and the repository's own root `nvs.toml` is where `examples/http.nvs`'s `[[app]]`
      block goes, beside the `examples/capability.nvs` one.
- [ ] **A spelling for `examples/http.nvs:43`'s environment read** — placement first, under ADR 0051
      § 3's roster. `crates/nvs-stdlib/src/registry.rs:1054` is `CLASSES`, and
      `crates/nvs-stdlib/src/config.rs:170` is the nearest shape: a member that answers `?string`
      off the request's own view rather than off `std::env`.
- [ ] **Whether a `Qual::Launder` parameter admits a union carrying the tainted arm.**
      `crates/nvs-types/src/core_lib.rs:755` is the check, `crates/nvs-stdlib/src/http.rs:138` is
      the row it reads. A launderer that refuses `string|tainted string` refuses the one shape
      `?? ` produces, which is how every real `Core\Env` read will arrive.
- [ ] **An origin on `127.0.0.1:8099` for the acceptance check**, which is the driver's half rather
      than the language's — `docs/agent/loop-goal.toml:2357` is the check and `tools/loop.py` runs
      it. A `[[check]]` that brings a fixture up is a new shape for that file, so decide it there.

## Backlog

- Three of stage 5's named `-p nvs-stdlib` tests are still unwritten — `docs/agent/loop-goal.toml:2343`.
- `traceparent` on an outbound request — ADR 0076 § 2; nothing carries a trace id yet.
- TLS: a client needs a trust anchor set, and no ADR paragraph owns which — `crate::http::transport`'s doc.
- A request body, and `Core\Http\Response`'s header map — `crate::http`'s "what is not here yet".
- A connection pool: priority 3 against 4, nothing in this goal waits on it — same doc.
- `orient.py` printed no ADR 0074 §§ 5-6 and no ADR 0058 § 4; add them to `[context] adrs`.
