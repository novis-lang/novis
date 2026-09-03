# Handoff

## State

**Goal 6, Stage 4 has opened: `Core\Response` is registered**, with three of ADR 0088 § 4's
five body members — `text`, `json` and `bytes` — each setting its own `Content-Type`.
`crates/nvs-stdlib/src/response.rs` is the module and its doc owns every decision below.

**A response body is the output an isolate wrote, and the declaration crosses beside it.** § 3's
table already binds a request's `echo` to the body, so a body member writes through
`Ctx::write_output` like every other writer; what it adds is one string, the `Content-Type`.
`Ctx::declare_content_type` records it (`crates/nvs-runtime/src/ctx.rs:4128`), the isolate's
finish path takes it onto `Completion::content_type`, and `nvs_server::serve::answer`
(`crates/nvs-server/src/serve.rs:445`) turns it into the header — defaulting to `text/html`
for a request that only echoed, which is § 4's last bullet. A `Completion` field rather than
something read off a `Ctx`, because the accept loop never holds a served request's context.

**`bytes`' content type is a sink on both sides.** § 1 refuses a `tainted` one at compile time;
`spellable` (`crates/nvs-stdlib/src/response.rs:216`) refuses a runtime value no header field
can carry, and `answer`'s `UNSPELLABLE` fallback is the layer below that rather than the only
one. Both are asserted — the member's refusal in the corpus, the header's in `-p nvs-server`,
which is the only place a `Content-Type` is observable at all.

**Unchanged limits.** Nothing refuses `echo` beside a body member yet — § 4's sixth row is a
*compile* error and is the next item, which is why nothing at runtime adjudicates between two
declarations. `html` waits on `Core\Html\Markup` being spellable as a registry parameter, and
`sendFile` on the mount root a path resolves against. Off a request the declaration is inert and
the bytes reach the sink in force unsubstituted (module gap 1). The driver's
`native examples/upload.nvs` failure is stage 5's frozen `want`, not a regression.

**`[context]` gaps still open**, both reported twice now: `adrs` prints `0097 §2`, `§4` and
`0106 §13` but not **`0097 §5`**; and `0088 §§ 3-4` had to be sliced by hand this session
despite being the section every Stage 4 item is specified by. `modules` still names no
`nvs-cli` pattern.

## Next group

**Stage 4 items 3 and on: what a response carries besides its body** — ADR 0088 § 4 and spec
§ 15. One file set, all of it loaded by the session that wrote the body members:
`crates/nvs-stdlib/src/response.rs`, `crates/nvs-runtime/src/ctx.rs`,
`crates/nvs-runtime/src/host.rs`, `crates/nvs-host/src/isolate.rs`,
`crates/nvs-server/src/serve.rs`.

- [ ] **`echo` and a body member on one response is a compile error** — ADR 0088 § 4's sixth
      row, and the reason the runtime does not adjudicate. The check is in the type layer, not
      in `nvs-stdlib`: `crates/nvs-types/src/expr/args.rs:461` is where a `Core` row's
      arguments are checked and `crates/nvs-types/src/expr/quals.rs:288` is the neighbouring
      ADR 0088 rule to read for shape. Next free diagnostic in the types band is **E0503** in
      `E05xx`; `E04xx` and `E07xx` are both FULL, so this needs `python tools/brief.py` to
      confirm the band before a code is claimed. The runtime half is already written:
      `crates/nvs-stdlib/src/response.rs:232` declares before it writes.
- [ ] **`setStatus`, and the status crosses the same way the content type does** — spec § 15.
      `crates/nvs-runtime/src/ctx.rs:4128` and `:4136` are the pair to copy,
      `crates/nvs-runtime/src/host.rs:263` is `Completion`'s field block, and
      `crates/nvs-server/src/serve.rs:445` is where `answer` would set it. Decide there whether
      a status and a header list are two more fields or one `response` struct — three fields is
      the point at which the second is worth it.
- [ ] **`setHeader`, a header sink over that same channel** — spec § 15 says it overrides a
      policy-owned header on one response. `crates/nvs-stdlib/src/response.rs:216`'s
      `spellable` is the value check already written; the name needs the same treatment, and
      ADR 0074's own headers are what "policy-owned" means.

## Backlog

- `Core\Response::html` — waits on `Core\Html\Markup` as a registry parameter type
  (`crates/nvs-stdlib/src/html.rs`); ADR 0088 § 4.
- `Core\Response::sendFile` and `redirect` — ADR 0088 § 4 and § 1; `sendFile` wants
  `crates/nvs-server/src/statics.rs`'s policy rather than a second one.
- `Core\Response::addCookie` — ADR 0074's `[http.cookies]` defaults every field.
- `Core\Request` is not registered at all — spec § 15; ADR 0097 §§ 3, 6, 7, 8.
- A request's `echo` still renders through the terminal table rather than § 3's HTML sink —
  `crates/nvs-runtime/src/ctx.rs`'s `write_output`; ADR 0088 § 3.
- The `catch`-name ICE in the lowerer is a real bug, not only a test-writing trap —
  `crates/nvs-ir/src/lower/expr.rs:2424`.
