# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled
connections. Stages 1–7 are on disk.** [ADR 0180](../decisions/0180.md) is the record and the home
of every decision this goal executes.

Stage 7's last half landed: a **client identity**. `Core\Http\Identity::read(bytes $chainPem,
Crypto\KeyPair $key)` is a new Tier 0 class beside `Core\Crypto\KeyPair`, and it refuses a leaf
whose `SubjectPublicKeyInfo` is not the pair's — the one check that can be made without asking a
server anything. `identity?: Core\Http\Identity` is the last of the shared bag keys, so every
request member takes it, and it reaches the handshake through `nvs_host::tls::NvsIdentity`, whose
`ClientConfig` carries a client-cert resolver over the same process-wide anchor set. `pool_key`
carries the leaf's SHA-256, so two identities are two pools and a call under none never draws a
connection that presented one.

**One deviation from the goal's § *Stage 7* prose, deliberate:** the prose says a config is "built
once per `Identity` and held as long as it is"; it is built once per **call** instead and released
with it. A slot holds a Novis value, so holding a built config would mean a process- or core-lifetime
table keyed by private key material — priority 1 spent to buy priority 3, which
`crate::crypto::KEY_PAIR`'s own doc already turned down for the same material. The pool is what makes
it cheap: a reused connection handshakes not at all. The record's *What it spends* already reads
"one `ClientConfig` … released with the call, O(in-flight)".

Nothing is blocked.

## Next group

**Stage 8: credentials on a redirect** — one file set, the same one stage 7 left loaded:
`crates/nvs-stdlib/src/http/transport.rs`, `crates/nvs-stdlib/src/http.rs`.

- [ ] **A `secret` header value is carried as such into the call**, so a hop can tell which headers
      were credentials — `headers_of` flattens a `secret string` into a plain `String` today
      (`crates/nvs-stdlib/src/http.rs:1848`), and `Call::headers` is `Vec<(String, String)>`
      (`crates/nvs-stdlib/src/http/transport.rs:152`). The goal's § *Stage 8* prose is what it
      executes; `rule:http-server/redirects-are-off-and-every-hop-is-re-pinned` is the rule the hop
      runs under.
- [ ] **A hop to another origin drops `Authorization`, `Cookie`, `Proxy-Authorization` and every
      header whose value was `secret`; a hop within one origin keeps them** — the hop loop is
      `crates/nvs-stdlib/src/http/transport.rs:1022`, and the headers are written in `compose`
      at `crates/nvs-stdlib/src/http/transport.rs:1478`. Same origin is scheme, host and port all
      equal.
- [ ] **No header value reaches a trace span, a log record or an error message** — made a test
      rather than a habit, over the guard that already names the header and not the value
      (`crates/nvs-stdlib/src/http/transport.rs:1478`, the `field` calls).

## Backlog

- Stage 9 wants the first end-to-end `https` test through `Core\Http\Client`; today no test crosses
  the `transport.rs` → `tls.rs` seam, because `NvsTls::over` verifies against the compiled-in
  anchors and a loopback origin cannot chain to one. `[http.client.tls] roots` is what unblocks it.
- An identity with an operator-named anchor bundle has no spelling: `NvsIdentity::read` builds over
  the compiled-in set only. Stage 9's `roots` is where the two meet.
- `Core\Http\Identity` is not in `docs/spec/01-core-library.md`; the reference card is its only
  prose home today.
- Parsing `Link`, `Retry-After` for a program, and RFC 9457 problem details are the `nvs/rest`
  package's, not this goal's (§ *Not this goal*).
