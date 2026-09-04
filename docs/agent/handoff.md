# Handoff

## State

**Goal 6, Stage 5: `Core\Request` answers six of spec § 15's fifteen members** — `method`, `path`,
`query`, and now `header`, `headers` and `cookie`. Everything a request has before its *body* is
readable. `crates/nvs-stdlib/src/request.rs`'s `//!` owns the nine still owed and what each waits
on, and its three new sections own the join rule, the lower-cased keys and the cookie parse.

**`nvs_runtime::Inbound` now carries one entry per header *field line*** — name verbatim, value
bytes — with `push_header` the writer and `headers()` the reader. A list and not a map: a repeated
name keeps every value, which `DeclaredHeader`'s outbound half decided the other way round for the
same reason. **Nothing writes one yet**; `nvs_server` is still the first writer, and that is the
whole of the next group.

**Three design calls, recorded in the module doc rather than in an ADR.** `header` joins a repeated
field with `, ` (RFC 9110 § 5.3's own equivalence, so nothing is dropped or invented) and `headers()`
answers `array<array<tainted string>>` keyed by the lower-cased name so the lines stay apart — the
qualifier has to ride on the innermost value, there being no `tainted array<T>`. `cookie` matches
byte for byte (ADR 0095 § 3, CVE-2024-2756) and hides a `__Host-` name that arrived twice, since a
browser holds at most one per host. **The other half of § 3 — that the connection was secure — is a
known gap waiting on `scheme()`**, because the carrier does not hold what the door concluded about
the connection.

**The failing acceptance check is `examples/upload.nvs`, and it is open work rather than a
regression**: it wants ADR 0105's `files()`, which no slice has started.

**`[context]` gaps.** `adrs` printed 0074 § 5 and 0097 §§ 2 and 4; this group needed **0095 § 3**
(the cookie rule, read by hand). `spec` still has no selector, so § 15 costs a `grep` and a read a
session. `modules` names no `nvs-cli` pattern, and the next group's production seam is in that
crate.

## Next group

**The server writes the carrier.** One file set: `crates/nvs-cli/src/serve.rs`,
`crates/nvs-server/src/serve.rs`, `crates/nvs-host/src/isolate.rs`. The design question is in the
first item and the other two rest on it.

- [ ] **The server writes the carrier, and an unrecognized verb is a `501` at the door** — ADR 0097
      §§ 2 and 4. The seam the last handoff named is a test fixture; the production one is
      `crates/nvs-cli/src/serve.rs:280`, where a resolved mount becomes an `Isolate` and the
      `Request` is in hand. `crates/nvs-server/src/serve.rs:437` is where that isolate meets the
      `Ctx` — the only place `crates/nvs-runtime/src/ctx.rs:4312`'s `set_inbound` can be called —
      so **the `Inbound` has to ride from the handler to the runner**, through
      `crates/nvs-host/src/isolate.rs:128`'s `Isolate::new` or beside it. Build it with
      `crates/nvs-runtime/src/ctx.rs:4428`'s `push_header`, one call per `hyper` field line, mount
      strip already done by step 2.
- [ ] **A served request answers, end to end, in `nvs-server`'s own tests** — spec § 15.
      `crates/nvs-server/src/serve.rs:1067` is `echo_the_path`, the fixture every case builds a
      handler from; a sibling that echoes `Core\Request::method()`, `::path()`, `::header()` and
      `::cookie()` is what turns every `.nvst` case above from a refusal into a claim about an
      answer. No conformance case can do this — a case is a program with no request in front of it.
- [ ] **`isHead`, once a verb arrives** — spec § 15, `crates/nvs-stdlib/src/request.rs:96` is
      `CLASS` and `crates/nvs-stdlib/src/request.rs:279` is `method_ordinal`, where `HEAD` already
      answers `Get`'s ordinal. `isHead` is the truth that mapping drops, and the door has to know
      it too: a `HEAD` response carries the headers and no body.

## Backlog

- ADR 0105's uploads — `files()`, `body()`, `bodyStream()`; `examples/upload.nvs` is the acceptance
  check that fails on them. `docs/plan/m7.md`.
- `clientIp`, `scheme` and `host`, with `[server] trusted_proxies` and the forwarded-header walk —
  ADR 0097 § 6. Unblocks ADR 0095 § 3's secure half in `crates/nvs-stdlib/src/request.rs`.
- `route()` and `mount()` — the match the server makes once before the handler, ADR 0102.
- A memoized parse on the carrier, for `query` and `cookie` alike, once a request reads more than a
  few — priced in `crates/nvs-stdlib/src/request.rs`'s `//!`.
- `Core\Request::header`'s answer is `?tainted string` and nothing pins the qualifier reaching a
  sink; the corpus has no `--EXPECTF-ERROR--` case for it.
