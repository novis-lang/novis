# Handoff

## State

**Goal 6, Stage 5: a served request reaches the program answering it.** The door builds
`nvs_runtime::Inbound` from the arrived request — verb, ADR 0097 § 4 step 2's remainder, raw query,
one entry per header field line — and it rides to the program on the `Isolate`, not on the
connection's context: `Isolate::answering` owns that direction, and its doc is why a `spawn script`
child still answers no request. `nvs-server`'s `a_served_request_reaches_its_isolate_as_the_inbound_carrier`
holds it end to end over a real socket, two `X-Trace` lines included.

**An unrecognized verb is a `501` before an isolate exists.** The roster's one home stays
`nvs_stdlib::request` — `is_known_verb` is what the door asks — and `nvs-server` gained the spelling
(`Reply::not_implemented`) rather than a dependency on `nvs-stdlib` for eight words.

**`Core\Request` answers seven of spec § 15's fifteen**, `isHead` beside `method`: the same token
read twice, because § 15 reports a `HEAD` as `Get` on purpose. `crates/nvs-stdlib/src/request.rs`'s
`//!` owns the eight still owed and what each waits on.

**Known gap this group leaves:** the door's `501` has no end-to-end case. A `Request<Incoming>`
cannot be built outside `hyper`, so the handler is not unit-testable, and `nvs serve` has no socket
harness — the roster predicate's own tests are all that stands under it today.

**The failing acceptance check is still `examples/upload.nvs`, and it is open work**: it wants
ADR 0105's `files()`, which no slice has started. That is what the next group is.

**`[context]` gaps.** The `501` rule this item implemented is stated in `Core\Request::method`'s
reference card, not in any ADR section `adrs` can select — worth a `spec` selector for § 15, which
still has none and costs a `grep` and a read a session.

## Next group

**ADR 0105's body, which is what the failing check wants.** One file set:
`crates/nvs-runtime/src/ctx.rs`, `crates/nvs-cli/src/serve.rs`, `crates/nvs-stdlib/src/request.rs`.
Item 1 is the design call and the other two rest on it.

- [ ] **The carrier holds the request body, as a stream and not as bytes** — ADR 0105 §§ 3 and 5,
      § 6's "there is still no temp file". The body is `hyper`'s `Incoming`, which only a poll can
      read, so the question is what crosses into `crates/nvs-runtime/src/ctx.rs:4364`'s `Inbound`
      beside the three strings: bytes already read under ADR 0138's `block_on` at
      `crates/nvs-cli/src/serve.rs:312`, or a reader the isolate parks on. § 5's two `[limits]` caps
      decide it, and `crates/nvs-host/src/isolate.rs:160`'s `answering` is where whatever it is
      arrives.
- [ ] **`Core\Request::body()` and `bodyStream()`** — spec § 15, ADR 0105 § 3's three ways to
      consume a part. Five edits at `crates/nvs-stdlib/src/request.rs:160` (the rows),
      `:358` (the `address` arms) and `:599` (a body's shape), plus three `.nvst` cases each.
- [ ] **`files()`, the lazy iterator** — ADR 0105 §§ 1 and 2: a part is a file part iff
      `Content-Disposition` carries `filename`. The rows and the `address` arms are
      `crates/nvs-stdlib/src/request.rs:160` and `:358` again, and the reader they call is item 1's.
      This is what `examples/upload.nvs` asks for, and the driver's acceptance check closes with it.

## Backlog

- ADR 0097 § 7's server half: a `HEAD` answer discards the body and keeps `Content-Length`.
- `Core\Request::mount()` and `route()` — ADR 0102's match, made once before the handler.
- `clientIp`/`scheme`/`host` — ADR 0097 § 6's forwarded walk and `[server] trusted_proxies`.
- The `__Host-` cookie's second half needs `scheme()` — `crates/nvs-stdlib/src/request.rs`'s `//!`.
- An end-to-end case for the door's `501`, once `nvs serve` has a socket harness.
- `docs/spec/01-core-library.md` § 15 has no `[context] spec` selector in `loop-goal.toml`.
