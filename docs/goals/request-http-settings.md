---
looper: 1
title: "a request's own [http] settings shape its response"
summary: "Every [http] setting a request changes with Core\\Config::set reaches that request's response: its security headers, its CORS answer and its cookie defaults. A request that sets nothing costs nothing extra, and a test per setting reads the setting list itself."
context: [crates/nvs-config/src/request.rs, crates/nvs-config/src/http.rs, crates/nvs-server/src/secure.rs, docs/rules/http-server/policy-headers-are-runtime-class-and-setheader-wins.md]
stages:
  - id: 1
    title: "a request's [http.headers] and [http.cors] settings reach its response"
    status: done
    context: [crates/nvs-runtime/src/ctx/output.rs, crates/nvs-server/src/cors.rs, crates/nvs-server/src/csp.rs]
  - id: 2
    title: "cookie defaults and every other [http] reader take the request's settings"
    status: done
    context: [crates/nvs-stdlib/src/response.rs, crates/nvs-config/src/directive.rs, docs/rules/config/three-changeability-classes.md]
  - id: 3
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: headers-follow-the-request
    stage: 1
    name: "a request's own content_security_policy, frame_ancestors and CORS origins reach its response, and a request that set none gets the file's"
    run: [cargo, test, --lib, --, a_requests_own_header_settings_reach_its_response, a_request_that_sets_no_http_setting_gets_the_files_policy]
    timeout: 1800
    tests:
      - a_requests_own_header_settings_reach_its_response
      - a_request_that_sets_no_http_setting_gets_the_files_policy
  - id: every-http-setting
    stage: 2
    name: "every [http] key a request may set changes that request's response, and Core\\Config::set returns false for every other one"
    run: [cargo, test, --test, request_http_settings]
    timeout: 1800
    tests:
      - every_http_setting_a_request_may_set_reaches_its_response
---

# a request's own [http] settings shape its response

## Why

`[http]` is a `Runtime` directive (`crates/nvs-config/src/directive.rs:126`). So a request may change
any of its keys for itself with `Core\Config::set`, and `set` returns `true`. Both
`rule:http-server/policy-headers-are-runtime-class-and-setheader-wins` and
`docs/examples/config/http/about.md:10` promise the change shapes that request's response.

It does not. The server builds its header set and its CORS answer once per configuration snapshot
(`crates/nvs-server/src/serve.rs:1128`, `crates/nvs-server/src/secure.rs:122`,
`crates/nvs-server/src/cors.rs:207`), and nothing in `nvs-server` reads what a request set. Cookie
defaults read the snapshot too (`crates/nvs-stdlib/src/response.rs:1693`). Seen on the release binary:
`Core\Config::set('http.headers.content_security_policy', "script-src 'self'")` returned `true`, and
the response carried `frame-ancestors 'none'` alone.

The per-request set matters most for a strict `Content-Security-Policy`. A page that needs a nonce
per response can only write the whole header with `setHeader` today, which replaces the configured
`frame-ancestors`, so the page repeats that value by hand and the two drift. With the set working,
the page sets `content_security_policy` alone. The server adds the configured `frame-ancestors`, and
on a slotted page the two script hashes, exactly as it does for the file's policy.

It fails closed: the configured policy stays, and no request reaches another. It is a bug, not a
vulnerability.

## Standing decisions

**The request's settings travel with its response, and only when it set one.** The runtime hands the
server the request's overlay entries under `http.headers.` and `http.cors.` beside its declared
headers (`crates/nvs-runtime/src/ctx/output.rs:1248`). A request that set none hands over nothing, and
the server uses its cached `Policy` as now. A request that set one gets a header set built for it
from the snapshot's `Http` with its overlay folded over it. **What it spends:** nothing for a request
that set no `[http]` key. A request that did pays one render of its own header set, a few hundred
bytes, per request and O(in-flight). Safe default if folding into `Http` proves awkward: build the
`Secure` and `Cors` from a cloned `Http` with the overlay assigned, which is the same cost.

**One fold, in `nvs-config`.** `crate::http::Inbound::assign` (`crates/nvs-config/src/http.rs:158`)
already folds an overlay over the snapshot for `[http.cors]` and `[http.cookies]`
(`crates/nvs-config/src/request.rs:479`). The new reader uses one function there for all three
blocks. No second copy of the fold in `nvs-server` or `nvs-stdlib`.

**Order on the response is unchanged.** The request's own header set fills first, then a header the
program wrote with `setHeader` still wins (`rule:http-server/policy-headers-are-runtime-class-and-setheader-wins`),
then the slot-script hashes and the `[debug] inline` nonce are added
(`crates/nvs-server/src/serve.rs:2592`).

**A CORS preflight is the file's.** An `OPTIONS` preflight is answered before any program runs
(`crates/nvs-server/src/serve.rs:2035`), so no request setting can reach it. Only the actual
response's CORS headers take the request's values. The rule fragment says so.

**A key nothing reads is not settable.** Every `http.*` key a request may set either changes that
request's response, or `Core\Config::set` returns `false` for it. Where a reader cannot take the
overlay without a change across crates that is out of this goal, the key returns `false` and the rule
fragment says so. `http.csrf_key` and `http.csrf_key_file` stay not settable.

**`Core\Test::request` does not change.** The in-process path never builds a response head
(`rule:testing/in-process-request`). The proofs are Rust tests on the wire: a lib test in
`nvs-server` for stage 1, and an integration test `crates/nvs-cli/tests/request_http_settings.rs` for
stage 2 that serves a program and reads the response.

**No new example and no new `Core` API.** `docs/examples/config/http/about.md` already states the
behaviour, and an example cannot show a server header, since `Core\Test::request` returns only the
program's.

**A list setting crosses as comma-separated text.** `Core\Config::set` passes values as text, so
`http.cors.origins`, `methods`, `headers` and `expose` take entries separated by commas, and
`frame_ancestors` takes sources separated by commas or spaces (`crates/nvs-config/src/http.rs:188`).
This lets a request set its own CORS origins and keeps the API one string. `set` then asks the
wildcard-with-credentials refusal of the folded block, so a request's `origins = "*"` beside the
file's `credentials = true` returns `false`.

**The carrier is `Completion::http`, not `output.rs`.** The folded block itself travels, built by
`Ctx::response_http` in `wiring.rs` when the request ends. `nvs-host`'s own code reads no
configuration, and `output.rs` was claimed by another session.

**Four `Runtime` keys under `[http]` are refused by name, not by class.** `http.cors.methods`,
`headers` and `max_age` reach only a preflight, and `http.errors.detail` only the page of a request
that never got a frame, so no request-time reader exists for any of them.
`nvs_config::request::UNREAD_BY_A_REQUEST` lists them, and `set` returns `false` for each. Their
registry rows stay `Runtime`, because `http.errors.detail` is a mode-table row and a mode's derived
rows must stay settable (`every_derived_default_names_a_directive_the_registry_holds`); a mode flip
still derives it into the overlay. `set` also refuses a `same_site` outside `Lax`, `Strict` and
`None`, as the boot does with `E0624`. **What it spends:** a scan of four keys per `set`.

**Lanes:** one lane. Stage 2 builds on stage 1's carrier, and both touch `serve.rs`.

## Stages

### 1. a request's [http.headers] and [http.cors] settings reach its response

**Does:** carry the request's `http.headers.` and `http.cors.` overlay out with its response, and
build that response's header set and CORS headers from it.

- [x] `nvs-config`: one function that returns the snapshot's `Http` with a request's overlay folded
  over it, used by `stays_meaningful` too (`rule:http-server/policy-headers-are-runtime-class-and-setheader-wins`,
  `crates/nvs-config/src/request.rs:479`, `crates/nvs-config/src/http.rs:158`).
- [x] `nvs-runtime`: the response carries the request's `http.headers.`/`http.cors.` overlay entries
  when there is at least one (`crates/nvs-runtime/src/ctx/output.rs:1248`).
  - [x] A finished response: `Completion::http`, filled by `Ctx::response_http`
    (`crates/nvs-runtime/src/host.rs:640`, `crates/nvs-runtime/src/ctx/wiring.rs:463`).
  - [x] A streamed response: `stream::Opened` gains `http`, set at each open site from
    `ctx.response_http()` (`crates/nvs-runtime/src/stream.rs:403`, `crates/nvs-runtime/src/stream.rs:646`,
    `crates/nvs-runtime/src/later.rs:397`, `crates/nvs-stdlib/src/response.rs:2229`).
- [x] `nvs-server`: at the fill, a response carrying settings gets `Secure::of` and the CORS answer
  built from the folded `Http`. Every other response uses the cached `Policy`
  (`crates/nvs-server/src/serve.rs:2591`, `crates/nvs-server/src/serve.rs:2035`,
  `crates/nvs-server/src/secure.rs:122`, `crates/nvs-server/src/cors.rs:207`).
  - [x] The fill reads the `RequestHttp` extension `finished` inserts
    (`crates/nvs-server/src/serve.rs:2599`, `crates/nvs-server/src/serve.rs:2976`).
  - [x] `streamed` inserts `RequestHttp` from `Opened::http` (`crates/nvs-server/src/serve.rs:3065`).
- [x] Tests `a_requests_own_header_settings_reach_its_response` and
  `a_request_that_sets_no_http_setting_gets_the_files_policy` in `crates/nvs-server/src/serve.rs`
  tests: a set `content_security_policy` with the configured `frame_ancestors` appended and the slot
  hashes added on a slotted page, a set `frame_ancestors`, a set CORS `origins`, and a request that set
  nothing (`crates/nvs-server/src/serve.rs:1`).
  - [x] Both tests exist and pass for a finished response: policy, `frame_ancestors`, `origins`, and
    nothing set (`crates/nvs-server/src/serve.rs:11297`).
  - [x] A slotted page's case: a set `content_security_policy` gains the two slot hashes.
- [x] The rule fragment gains the preflight sentence and the cost line
  (`docs/rules/http-server/policy-headers-are-runtime-class-and-setheader-wins.md:1`).

### 2. cookie defaults and every other [http] reader take the request's settings

**Does:** make the cookie defaults read the folded `Http`, then hold every `http.*` key to the
standing decision with one test that reads the key list.

**Split:** an `Explore` subagent lists every reader of an `[http]` value outside `nvs-config`, as
`file.rs:NN` and the key, before the first edit.

- [x] `configured_cookies` reads the folded `Http` (`crates/nvs-stdlib/src/response.rs:1693`,
  `rule:http-server/cookies-are-secure-httponly-and-lax`).
  - [x] `Request::cookies` folds the request's `http.cookies.` settings through `http::folded`.
  - [x] `configured_cookies`, and so the session cookie, reads it.
- [x] Every other `[http]` reader the list names, `[http.errors] detail` among them, reads the
  request's value, or its key returns `false` from `set` (`crates/nvs-config/src/request.rs:300`).
  - [x] The outbound-call keys already read `Request::get`; TLS, proxy, pool and CSRF are `System`.
  - [x] `UNREAD_BY_A_REQUEST` refuses the three preflight keys and `http.errors.detail`.
  - [x] The `[http]` example, its `about.md`, the attack's step 6 and `default.toml` follow.
- [x] Test `every_http_setting_a_request_may_set_reaches_its_response` in
  `crates/nvs-cli/tests/request_http_settings.rs:1`. It enumerates the `http.*` keys from the
  configuration's own key list, not a list in the test, and fails naming a key that `set` accepts and
  the response ignores.
- [x] `rule:config/three-changeability-classes` names any `http.*` key that is not settable
  (`docs/rules/config/three-changeability-classes.md:1`).

### 3. the goal is verified once, and every failure is fixed

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
- [x] Run `looper verify` once more, and it is green.

## Handoff

### State

All three stages are done. The first `looper verify` failed on rustfmt and two stale rule chapters,
fixed in `e24b070633`; the second was green, the examples and hostile suites included. The
goal-end gates are green: `bun nv verify --doc`, `bun nv owners --closes request-http-settings` and
`bun nv ci --check`. That last one needed the `[http]` `about.md` cut under 200 words, the page's
questions restamped, the stale perf of the response, script, sse and crypto members recorded, and a
roadmap theme for this goal.

The acceptance sweep's `env-secret-proofs` red is stale: `looper check --name env-secret-proofs`
finds no such check, and its log does not exist.

### Next

Nothing in this goal. The retire hooks rebuild `target/release/nvs`. The roadmap theme in
`website/src/data/roadmap.json` goes when the goal leaves the chain. No test is expected red.

### Backlog
