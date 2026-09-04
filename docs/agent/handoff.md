# Handoff

## State

**Goal 6, Stage 4's headers landed: `Core\Response::setHeader` sets one, and the list crosses to
the peer.** `crates/nvs-stdlib/src/response.rs`'s module doc has a new section — *A header is a
list on that same channel, and `Content-Type` is not one* — and it is the home for every decision
below.

**Same channel as the status, a third time, but a list.** `Ctx::declare_header`
(`crates/nvs-runtime/src/ctx.rs:4219`) holds `Vec<(Box<str>, Box<str>)>`, `take_headers` runs on the
finish path beside `take_status`, `Completion::headers` carries it, and `nvs_server`'s `answer`
applies each pair **last**, after everything the server wrote for itself — which is the whole of
what ADR 0074 § 4 means by an override. Set, not add: one name written twice is one header, keeping
the first call's position and spelling and the last call's value, compared case-insensitively.

**Three decided-and-recorded calls.** `Content-Type` is **refused** whatever its case — ADR 0088
§ 4's argument is that a body member owns one shape *and* its media type, so admitting it here would
let a handler answer `json` and relabel it `text/html`; `bytes` is the member that takes a media
type. **Both parameters are sinks**, a header line being two instructions. **The byte rules are
narrower than RFC 9110**: a name is a non-empty token, a value is printable ASCII, so a tab and an
`obs-text` byte are refused — `nameable` and `carriable` in `response.rs`, the second now also
`spellable`'s body.

**What is not asserted, and why.** `answer`'s ordering is not yet observable: the only header the
server writes for itself is `Content-Type`, which the member refuses, so the override becomes
testable when ADR 0074 § 1's policy set lands. A `.nvst` case cannot see a header line at all, so
the crossing is pinned by `crates/nvs-server/src/serve.rs`'s
`a_declared_header_reaches_the_response_and_a_failure_drops_it` and by `nvs-host`'s
`a_declared_status_and_header_cross_even_when_the_child_threw`, which is the group's third item:
`finish` carries both out on the throwing path and `answer` is where they stop.

**The acceptance check that fails is Stage 5's, not a regression.** `native examples/upload.nvs`
wants ADR 0105's whole surface and `crates/nvs-stdlib` has no `request.rs`; it has been the reported
failure for sixteen sessions and holds the run to 55 of ~137 checks. The playbook's new *Tooling*
bullet owns it. Nothing in Stage 4 can close it and the frozen `want` is correct as written.

**`[context]` gaps, reported again.** `adrs` prints `0074 § 5` but not **`§§ 1 and 4`**, which are
the two this item is written against; **`0105 §§ 1-4`** is not printed either and is what the failing
check needs. `spec` still has no selector, so § 15 — prose, not a `| Member |` table — costs two
reads a session. `modules` still names no `nvs-cli` pattern.

## Next group

**What a response says beside its body** — spec § 15 and ADR 0074 §§ 1-4. One file set, this
session's plus one config file: `crates/nvs-stdlib/src/response.rs`,
`crates/nvs-runtime/src/ctx.rs`, `crates/nvs-server/src/serve.rs`, `crates/nvs-config/src/tree.rs`.

- [ ] **ADR 0074 § 1's secure-header set, applied by `answer` with nothing configured** — the half
      that makes `setHeader` an override rather than an addition. `crates/nvs-server/src/serve.rs:489`
      is the loop that must run *after* it, `crates/nvs-server/src/serve.rs:445` is `answer` itself,
      and `crates/nvs-config/src/tree.rs:393` is the `[http.headers]` block that already
      deserializes — `content_type_options`, `frame_ancestors`, `referrer_policy`, `hsts`. § 1's
      three decisions are HSTS only on an `https` effective scheme, `hsts_subdomains` false, and no
      `default-src`.
- [ ] **`redirect`, which is a status and a `Location` in one member** — spec § 15, and it rides
      both channels already built, so it opens no third one. `crates/nvs-stdlib/src/response.rs:238`
      is the `setHeader` row to put it beside and `crates/nvs-stdlib/src/response.rs:501` is the
      helper to model; the decisions are which 3xx it defaults to and whether the target is a sink
      (it is a header value, so `carriable` at `crates/nvs-stdlib/src/response.rs:405` is the check
      it owes).
- [ ] **`addCookie`, whose options shape defaults every field from `[http.cookies]`** — spec § 15,
      ADR 0074 § 3. A cookie is a `Set-Cookie` header, so it rides `Ctx::declare_header`
      (`crates/nvs-runtime/src/ctx.rs:4219`) — except that `Set-Cookie` is the one header a response
      may carry twice, which the replace-by-name store above does not admit, and that is the slice's
      real question. `SameSite` is an enum and never a string:
      `crates/nvs-stdlib/src/io.rs:1232` is the `EnumDoc` shape to copy and
      `crates/nvs-config/src/tree.rs:435` is the `String` in the config that pairs with it.

## Backlog

- `html` and `sendFile`, ADR 0088 § 4's other two body members — `response.rs`'s known gaps.
- Stage 5 whole: `Core\Router::match`, `Core\Session`, and ADR 0105's uploads, which is what
  `examples/upload.nvs`'s frozen output is waiting for — `docs/agent/loop-goal.md` items 12-16.
- ADR 0074 § 2's closed CORS, unread by anything today — that ADR's own *Verification*.
- A `[context] spec` selector so § 15's prose reaches the pack — `docs/agent/loop-goal.toml`.
