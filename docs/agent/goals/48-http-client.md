---
milestone: M8
---
# Loop goal 48 — a program talks to a real API: bodies, headers, streams and pooled connections

`Core\Http\Client` stops being a fetcher and becomes a client an API is driven through. A request carries
a body — JSON, a form, raw bytes or multipart — `patch` joins the verbs, and `Client::request` takes a
`Core\Http\Method` for a verb chosen at run time. A reply is read through its headers, as a `tainted`
shape through `jsonAs<T>`, or as bytes, and a long or large one is **streamed** — SSE events, lines,
chunks, or saved to a file — rather than held whole. Connections are reused from a per-core pool that
keeps every address pinned, a gzip reply is decoded under the bomb bound `Core\Compress` already has, a
redirect to another origin drops the caller's credentials, a call can present a client certificate, the
deployment decides which certificates are trusted and for which hosts a program may relax that, and a
test answers outbound calls from a table
instead of standing up a listener. Everything the REST package (`nvs/rest`, not on this chain) needs from
`Core`'s HTTP half is then on disk.

## Why here

Directly after goal `webcrypto`, on the user's call: the REST client with OAuth was split into three goals
— `webcrypto` for the crypto, this one, and goal `process-cache` for where a token lives between requests
— in that order. It needs one thing goal `webcrypto` builds, `Crypto\KeyPair`, which stage 7's client
identity is made from, and goal `process-cache` needs nothing from this one. Before goal
`gap-zero` for that goal's standing reason — a register is emptied after everything that adds to it has
run.

What it needs already built, all on disk: the five request rows and their one options bag
(`crates/nvs-stdlib/src/http.rs:343-386`, `:423-470`); the transport's exchange, framing and header guard
(`crates/nvs-stdlib/src/http/transport.rs:290-375`, `:543-548`); the reply's parsed header list the class
never exposed (`transport.rs:113-126`); gzip under a bound (`crates/nvs-stdlib/src/compress.rs:522-533`);
`Core\Http\Method`'s eight cases (`crates/nvs-stdlib/src/router.rs:123-151`); `Core\Request::jsonAs`'s
decode-site check (`crates/nvs-types/src/expr/args.rs:1517-1528`); the in-process request `Core\Test`
already answers (`crates/nvs-stdlib/src/test.rs:357-373`); and the SSE framing goal `event-streams` fixed
on the writing side (`docs/agent/goals/42-event-streams.md:100-109`).

## Stage 0 — the catch-up

Sentences on disk this goal makes wrong. Each is corrected in the stage that makes it wrong. Re-grep before
editing: these are anchors, and files move.

- `crates/nvs-stdlib/src/http.rs:102-111` — § *What is not here yet*: no request body and no reply
  headers. Stages 4 and 5, each removing its own sentence.
- `crates/nvs-stdlib/src/http.rs:416-422` — "`patch`, `options` and `trace` are the verbs … this class does
  not — the spec's own `send(Core\Http\Request)` row is where a method chosen at run time belongs". Stage 4.
- `crates/nvs-stdlib/src/http/transport.rs:22-30` — § *One connection per attempt*: "A pool is a later
  slice". Stage 7 rewrites it whole.
- `crates/nvs-stdlib/src/http/transport.rs:67-73` — `REPLY_CEILING` is "the point where a caller should be
  streaming instead, which is the member this class does not have yet". Stage 6.
- `crates/nvs-stdlib/src/http/transport.rs:113-118` — "The header list is not handed to a program".
  Stage 5.
- `docs/spec/01-core-library.md:1167` — `send(Request)`, `stream` over a URL alone, and the seven-key
  options shape. Stage 2.
- `docs/rules/http-server/a-non-idempotent-retry-needs-an-idempotency-key.md` — "`Client::send($request)`
  with a runtime method". Stage 2, by the record's `changes.modifies`. [0074](../../decisions/0074.md)
  § 7 says the same and is frozen: the record says it changed, and 0074 is not edited.
- `docs/rules/security/one-tls-client.md:8` — "Full verification is the default with no spelling for
  turning it off". Stage 2, by `changes.modifies`: still one client, and the sentence becomes strict by
  default, relaxed only for a host a grant names.
- `crates/nvs-host/src/tls.rs:81-84` — "What is closed permanently is a *program* choosing anchors, or
  turning verification off". Stage 10 rewrites that section whole, and `upgraded`'s doc at `:272-280`
  with it.
- `crates/nvs-host/src/tls.rs:184-191` — `over_bundle`'s "`nvs_config::db` is that reader today".
  Stage 9, where `[http.client.tls] roots` becomes the second reader.
- `crates/nvs-stdlib/src/http/transport.rs:53-54` — "There is no plaintext fallback and no spelling for a
  session that verifies nothing". Stage 10; the plaintext half stays true and is kept.
- `crates/nvs-config/src/default.toml:260-268` — the `[http.client]` comment block. Stage 9 adds
  `[http.client.tls]` beside it, and stages 10 and 11 the grants under `[capabilities.tls]` and
  `[capabilities.net]`.

## Stage 1 — the floor

Goal `webcrypto`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the record

One new record, and no other number. Its body is § *Standing decisions* below, argued: this is
transcription, not design. It creates ten rules, all `designed`:

| Rule | Says |
|---|---|
| `http-server/an-outbound-request-carries-one-body` | at most one of the four body keys, a body on `get` or `head` refused, both while compiling |
| `http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime` | `deadline` ends at the head; `idle` and `maxDuration` bound the body; no spelling for forever |
| `http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned` | the pool's key — pinned address, port, scheme, server name, client identity and TLS policy — when a connection may go back, and its two caps |
| `http-server/a-cross-origin-redirect-drops-credentials` | the three headers and every `secret` value dropped on a hop to another origin |
| `testing/an-outbound-call-is-answered-from-a-table` | `Core\Test`'s table, and that a faked call never connects |
| `http-server/the-client-trust-roots-are-the-operators` | `[http.client.tls]`'s `roots`, `min_version` and `keylog`: the bundled set unless the operator names files, TLS 1.2 unless raised, and no key log in `production` |
| `security/tls-trust-is-relaxed-only-under-a-host-grant` | `tlsCa`, `tlsPin`, `tlsVerifyHost` and `tlsVerify` each need their host in a `capabilities.tls` grant, no grant has a `true` spelling, and every relaxed grant is printed at boot |
| `http-server/an-outbound-call-names-its-address-only-under-a-grant` | `connectTo` under `net.connect_to`: an IP literal, still judged by the address policy, the certificate still checked against the URL's host |
| `http-server/an-https-redirect-never-becomes-plaintext` | a hop from `https` to `http` needs `net.downgrade` for its host and `redirectToHttp: true`; a plain `http` URL asked for directly stays allowed |
| `http-server/a-reply-reports-its-tls-session` | `Response::tls()` — version, cipher, whether the peer was verified, and its chain — and `null` for a plain or a faked reply |

It modifies `http-server/a-non-idempotent-retry-needs-an-idempotency-key` (`send` becomes `request`) and
`security/one-tls-client` (stage 0's sentence), and the spec row at `docs/spec/01-core-library.md:1167`
moves with it. The record fixes the spellings the
surface leaves open — anything `rule:core-api/verb-lexicon` refuses — and the numbers it tunes stay inside
the bounds given below.

## Stage 3 — the keystone: an outbound call answered from a table

`Core\Test::answerHttp(string $url, uint $status, {json?, body?, headers?})` and
`Core\Test::sentHttp(): array<Core\Test\SentRequest>`, in `crates/nvs-stdlib/src/test.rs`. Once any answer
is registered in a test, **every** outbound call that test makes is answered from the table and an
unmatched one throws a `LogicError` naming the URL — nothing reaches the network. `$url` is exact, or a
prefix ending in `*`. The URL's text is judged exactly as it is today — scheme, grant — and resolution
and the address check are skipped for a faked call, because no connection is made to be pinned.

This is the keystone because every later stage's `.nvst` case is written against it, with no listener and
no outbound grant (`crates/nvs-stdlib/src/test.rs:161`). What it cannot reach is below the table — the
pool, gzip, framing, a redirect hop — and those stages prove themselves with Rust tests against a loopback
origin, as `transport.rs`'s tests already do.

## Stage 4 — bodies and verbs

Four keys join `Core\Http\Options`, flat per `rule:core-api/shape-rules` R2, beside a fifth that names a
raw body's type:

| Key | Sends |
|---|---|
| `json?: mixed` | `application/json`, encoded once per call |
| `form?: array<string, string>` | `application/x-www-form-urlencoded` |
| `body?: string\|bytes` with `contentType?: string` | the octets as given |
| `multipart?: array<string, string\|Core\Http\Part>` | `multipart/form-data`; `Http\Part::file($path, {filename?, contentType?})` streams from disk under `fs.read`, `Http\Part::bytes($data, $filename, {contentType?})` sends what it holds |

- **At most one body key, checked while compiling** — beside `reject_keyless_retry` in
  `crates/nvs-types/src/expr/args.rs`, because the bag is an R2 literal and the verb is the member's name.
  Two body keys, a body on `get` or `head`, or `contentType` without `body` is a diagnostic naming the key.
- **`patch`** joins the rows, and **`Client::request(Core\Http\Method $method, $url, {…})`** replaces the
  spec's `send(Core\Http\Request)`: the same bag, no `Request` class. Where the verb is dynamic, a body on
  `Get` or `Head` and a `Post` or `Patch` retried without a key both throw before the first attempt, which
  is what 0074 § 7 already asks of a dynamic verb.
- **A `secret` value reaches a header, a form field, a JSON body and a raw body** —
  `rule:security/secret-sinks-refuse` exempts all four (`docs/rules/security/secret-sinks-refuse.md:13-16`).
  Today's `headers` row is `array<string>` and a `secret string` does not narrow onto it
  (`crates/nvs-stdlib/src/registry.rs:302-305`), so each of the four positions is declared to admit one.
  The `json` key therefore cannot call `Core\Json::encode`, which refuses a secret anywhere in a value; it
  walks the value with the outbound exemption applied, and the walk is `crate::json`'s, not a second one.
- **A body position admits `tainted`**: posting what a user sent is ordinary, and the qualifier is a
  question about sinks this request does not have. Header names and values stay unqualified, as today.
- The body is built once per call and resent unchanged on every attempt; a file part is re-read per
  attempt. `Content-Length` is always written — every body's size is known before the first byte.

## Stage 5 — reading a reply

`Core\Http\Response` gains `header(string $name): ?tainted string`, `headers(string $name):
array<tainted string>`, `jsonAs<T>()` and `bytes(): tainted bytes`. Names match case-insensitively.
`header` joins repeats with `, ` as RFC 9110 § 5.3 allows, and is a `LogicError` naming `headers` for
`set-cookie`, which RFC 6265 forbids joining. `jsonAs<T>` joins `Core\Request::jsonAs` on the decode-site
roster (`crates/nvs-types/src/expr/args.rs:1517-1528`): `T` a `tainted` shape, or a class whose text fields
declare `tainted`, per `rule:security/derived-codec-qualifiers`. A `4xx` or `5xx` is still an answer
(`crates/nvs-stdlib/src/http.rs:595-598`).

## Stage 6 — streaming

`Client::stream(Core\Http\Method $method, $url, {…})` answers once the head has arrived, with the status
and headers readable and the body not yet read — a streamed reply is usually a `POST`, so the verb is a
parameter. Its body is read one way, once: SSE events, lines, chunks, or `saveTo(string $path, uint $max)`
under `fs.write` with a required byte bound, as `rule:core-classes/io-write-stream`'s `max` is. A second
read throws. `Core\Request::bodyStream` (`crates/nvs-stdlib/src/request.rs:314-320`) is the precedent for
the reader's shape, and the record says whether the four are members of one `Core\Http\Stream`.

- **`events()`** parses the WHATWG EventSource format: a line ends at `\r\n`, `\r` or `\n`; `data`,
  `event` and `id` are read, `retry` is read and ignored — reconnecting is the program's — and a comment
  line is skipped. Each event's fields are `tainted string`. A line and an event are each length-capped.
- **The bound.** `deadline` covers the connection and the head. After that the body runs under `idle`, the
  longest silence allowed, and `maxDuration`, the longest the body may take; both are `Duration`s, both
  inherit `[http.client] idle` and `max_duration` when omitted, and neither has an unbounded spelling. On
  a buffered member either key is a diagnostic.
- **A stream is retried only before its head.** After the first byte of body, a failure ends the stream.

## Stage 7 — the pool and gzip

`crates/nvs-stdlib/src/http/transport.rs`. **HTTP/1.1 keep-alive over a per-core pool**, keyed by the
pinned address, the port, the scheme, the TLS server name and the call's TLS policy (stage 10) — so a
reused connection is the connection
that address's check approved, and `rule:http-server/redirects-are-off-and-every-hop-is-re-pinned` holds
through reuse. A connection goes back to the pool only after its reply was read to the end under known
framing — `Content-Length` or chunked's last chunk — with no `Connection: close` from either side; on any
doubt it is closed. A reused connection that fails before the request's last byte is written is replaced
once without spending an attempt; any later failure is an attempt under the retry rules. The pool holds
at most `[http.client] pool_idle` idle connections per core and closes one idle past
`pool_idle_timeout`. `Connection: close` leaves the request; `Accept-Encoding: gzip` joins it.

**A client identity (mTLS).** `identity?: Core\Http\Identity` joins the bag, built by
`Http\Identity::read(bytes $chainPem, Crypto\KeyPair $key)` over goal `webcrypto`'s key pair, whose kind
is what the handshake is signed with; `read` refuses a chain whose leaf does not match the key. A call
with an identity presents it when the server asks and never otherwise, and **the pool key includes it**,
so two identities never share a connection and a call without one never reuses a connection that
presented one. One `ClientConfig` serves the whole process today (`crates/nvs-host/src/tls.rs:87`,
`:314`); an identity is a second config, built once per `Identity` and held as long as it is.

**gzip is decoded under `crate::compress::decompress_within`'s bound**, the one `Core\Compress` already
applies, so the ratio and the ceiling are one number with one home. A streamed reply decodes
incrementally under the same ratio.

## Stage 8 — credentials on a redirect

`crates/nvs-stdlib/src/http/transport.rs`'s hop, and shares that file with stage 7. A hop to another
origin — scheme, host or port differing — drops `Authorization`, `Cookie`, `Proxy-Authorization` and every
header whose value was `secret`; a hop within one origin keeps them. No header value is written into a
trace span, an access log record or an error message — the guard's message already names the header and
never the value (`transport.rs:369-375`), and that is made a test rather than a habit.

## Stage 9 — the trust roots are the operator's

`crates/nvs-host/src/tls.rs`, `crates/nvs-config/src/tree.rs` (`HttpClient`, `:507`) and `default.toml`.
A `[http.client.tls]` block, `System` class because it is the one `ClientConfig` the process shares
(`crates/nvs-host/src/tls.rs:86-91`), with three keys and no code-side spelling for any of them:

| Key | Shipped | Allows |
|---|---|---|
| `roots` | `["bundled"]` | Each entry is `"bundled"` — the compiled-in Mozilla set — or a PEM file. `["bundled", "/etc/novis/corp-ca.pem"]` adds a company CA; a list without `"bundled"` trusts only its files. Each file is resolved and trust-checked at boot exactly as `[db.<name>] tls_ca_file` is (`crates/nvs-config/src/db.rs:14-21`), and parsed by `nvs_host::tls` alone. |
| `min_version` | `"1.2"` | `"1.3"` raises the floor for every call. There is nothing below `1.2` to write: `rustls` speaks neither 1.0 nor 1.1. |
| `keylog` | unset | A file every session's secrets are appended to in the `SSLKEYLOGFILE` format, so an operator can read their own traffic in Wireshark. **Refused at boot when the server runs in `production`** — the mode's own default (`crates/nvs-config/src/default.toml:32-36`) — with a diagnostic naming the key; in `development` the boot says it is on, every start. |

The same stage lands the client's first end-to-end `https` test: a loopback origin with an `rcgen`
certificate under a `roots` file, fetched through `Core\Http\Client` rather than through `NvsTls` alone.
That is the seam between `transport.rs:264-268` and `tls.rs` that no test covers today.

## Stage 10 — relaxing trust from code, for a host a grant names

`crates/nvs-host/src/tls.rs`, `crates/nvs-runtime/src/capability.rs` and `crates/nvs-stdlib/src/http.rs`.
Four options join the bag, and each is unlocked by a grant under a new `[capabilities.tls]` area. Each
grant is a list of hosts matched as `net.connect` matches them, and **none has a `true` spelling**, for
`net.internal`'s reason (`crates/nvs-config/src/tree.rs:307-309`): what a deployment relaxed is legible
in review, host by host.

| Option | Grant | Does | curl |
|---|---|---|---|
| `tlsCa: string` | `tls.anchors` | trusts exactly the PEM certificates given, for this call, in place of `roots` | `CAINFO_BLOB` |
| `tlsPin: string\|array<string>` | `tls.pin` | accepts a peer whose SubjectPublicKeyInfo hashes to one of the `sha256//<base64>` values, with no chain — the spelling a self-signed origin is reached by | `--pinnedpubkey` |
| `tlsVerifyHost: false` | `tls.any_name` | builds and checks the chain, and skips only the name | `SSL_VERIFYHOST=0` |
| `tlsVerify: false` | `tls.insecure` | checks neither | `-k` |

- **The grant and the option are both owed.** A grant changes nothing about a call that does not ask; an
  option whose host is not in its grant throws before a connection is made, naming the grant — asked of
  the request's own configuration snapshot, per `rule:security/capability-question-is-grant-and-scope`.
- **Every relaxed session still checks the handshake signature** against the key the peer presented, so
  the peer holds that key; what is skipped is only the question of whose key it is. The verifiers plug
  into `rustls`'s custom-verifier seam inside `nvs_host::tls`, so there is still one TLS client, and a
  caller still cannot hand in a `ClientConfig`: it hands in a policy value the module builds one from.
- **`tlsMinVersion: "1.3"` needs no grant**, because it can only tighten; a value below
  `[http.client.tls] min_version` throws.
- A call with any of these five options builds its own `ClientConfig`; a call with none keeps the
  process's shared one. The policy is part of stage 7's pool key, so a connection opened under
  `tlsVerify: false` never serves a call that verifies.
- **The boot prints every relaxed grant, every start**, one line per grant and host, as goal
  `outbound-proxy` does for its own weakening; `nvs config dump` shows them with the rest.

## Stage 11 — the address, the downgrade, and what the handshake reports

`crates/nvs-stdlib/src/http.rs` and `crates/nvs-stdlib/src/http/transport.rs`.

- **`connectTo: string`**, under a new `net.connect_to` grant listing hosts. The value is an IP literal,
  and the call connects there instead of resolving the URL's host, while the certificate is still checked
  against that host (`transport.rs:45-47`). The address is judged by `rule:security/net-address-policy`
  and `net.internal` exactly as a resolved one is, so the option chooses among addresses the deployment
  already allows and widens nothing. A `Core\Http\Target` already carries its approved address, so
  `connectTo` beside one is a `LogicError`.
- **A redirect from `https` to `http` is refused** unless its target host is in a new `net.downgrade`
  grant *and* the call says `redirectToHttp: true`. Today every hop may be `http`
  (`crates/nvs-stdlib/src/http.rs:263`), so a server can drop a call's TLS with one `Location`. A plain
  `http` URL asked for directly stays allowed, and the refusal names the grant.
- **`Response::tls(): ?Core\Http\TlsInfo`**, read through members as every `Core` instance is:
  `version()`, `cipher()`, `verified(): bool` — `true` only when the chain and the name were both checked,
  against `roots` or `tlsCa` — and `peerChain(): array<tainted string>` as PEM, with the leaf's subject,
  issuer and expiry. The record says whether those three come from a parser already in the lock file or
  from a new dependency. `null` for a plain `http` reply and for a reply `Core\Test`'s table answered.

## Stage 12 — the rulebook

Flip stage 2's ten rules to `shipped`, with `guardedBy` filled from this goal's cases and tests, and
`python tools/rules.py --render`.

## Standing decisions

- **Settled with the user, not to re-decide:** bodies are flat keys of the one bag; `request(Method, …)`
  replaces `send(Request)`, and there is no `Core\Http\Request`; a stream is bounded by `idle` and
  `maxDuration` from `[http.client]`; gzip only; the pool is per core; a client identity is an option and
  part of the pool key; the redirect credential rule; outbound calls in a test are answered from a table.
  And TLS: strict by default; the operator widens trust in `[http.client.tls]` and names, in six host-list
  grants with no `true` spelling, the hosts code may relax it for; code relaxes it only there, and only by
  asking in the call. A plain `http` URL stays allowed, and only a redirect from `https` to `http` needs a
  grant. `keylog` is refused at boot in `production`. The design was argued before the goal was written
  and its argument is the record's body.
- **What stays exactly as it is:** the pin and `Core\Http::allowUrl`, one deadline over the whole
  buffered call, jittered opt-in retry and its closed status set, the idempotency key's compile-time
  check, redirects off by default, `traceparent`, and a `4xx`/`5xx` answered rather than thrown. The five
  existing rows keep their names and their bag.
- **Every reply is `tainted`**: `text`, `bytes`, `header`, `headers`, `jsonAs`'s fields and every
  streamed piece.
- **Config.** `[http.client]` gains `idle`, `max_duration`, `pool_idle` and `pool_idle_timeout`. The first
  two are `Runtime`, as `deadline` is (0074 § 5); the pool's two bound a core's memory and so are `System`,
  per `rule:config/three-changeability-classes`. The record fixes each shipped value; a default that is
  unbounded is a defect (`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`).
  `[http.client.tls]` is new — `roots`, `min_version` and `keylog`, all `System` — and so are six grants:
  `tls.anchors`, `tls.pin`, `tls.any_name`, `tls.insecure`, `net.connect_to` and `net.downgrade`.
- **What it spends**: per call, a request body built once and charged to the request, a multipart file
  part streamed at one chunk, and a buffered reply under `REPLY_CEILING` or a streamed one at one chunk
  plus the SSE line cap. Per core, at most `pool_idle` idle connections — a socket and a TLS session each
  — held between requests and charged to the core, which is `rule:security/db-pool-reset-is-a-boundary`'s
  shape: O(cores × pool_idle), never O(requests served). Per call that names a stage 10 option, one
  `ClientConfig` — parsed anchors or a verifier — released with the call, O(in-flight). Per process, the
  `roots` files, parsed once at boot beside or in place of the bundled set.
- **ADR slots**: the one record of stage 2.
- **Not this goal**: the REST package and OAuth; where a token lives between requests (goal
  `process-cache`); HTTP/2 and HTTP/3; brotli and zstd; a forward proxy, which is goal `outbound-proxy`,
  and TLS *to* that proxy, which it excludes; a passphrase-protected client key; CRL and OCSP; the OS
  certificate store as a `roots` entry; cipher-suite and curve selection; HSTS;
  a cookie jar; generating a client from an OpenAPI document, which the user deferred to the package; parsing `Link`, `Retry-After` for a program, or RFC 9457 problem details —
  those are the package's. A session that finds one on its path writes it to the handoff's `## Backlog`.
