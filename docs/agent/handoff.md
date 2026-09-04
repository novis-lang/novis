# Handoff

## State

**Goal 6, M7. ADR 0097 § 6's answer now reaches the program.** `nvs_runtime::Inbound` carries
the client address and the effective scheme — `set_peer` beside the body, `client()` and
`scheme()` to read them (`crates/nvs-runtime/src/ctx.rs:4586`) — and an `IpAddr` rather than
the text of one, so a request nobody asks pays nothing. `serve_connection`'s handler bound is
now `Fn(Request<Incoming>, Origin) -> Reply`, because the carrier is built in the door and the
accept loop never holds one; `nvs-cli`'s handler sets both facts beside the header lines.
`nvs_runtime::Scheme` is the one home of the two values and `nvs_server::secure` re-exports it.

**The driver's failed acceptance check is closed, and it was a filing bug of a new kind** — the
playbook's new Tooling bullet owns it. ADR 0088 § 4's runtime half is pinned in
`crates/nvs-stdlib/src/response.rs`'s new test module, and its check now runs under
`-p nvs-stdlib`; `mixing_echo_with_a_typed_body_member_is_a_compile_error` stays where a
compile check belongs.

`clientIp` and `scheme` now wait on nothing but their own five edits.
`crates/nvs-stdlib/src/request.rs:20` says what the other three still wait on: `host` on a
decision nobody has made (whether a forwarded host may be believed — § 6 answers an address and
a scheme and deliberately not this), `mount`/`route` on the match.

## Next group

**The two members the carrier was blocking, then § 6's unreported `Warn`.** One file set:
`crates/nvs-stdlib/src/request.rs` (rows, cards, bodies, the `address()` arm),
`tests/conformance/core/`, and `crates/nvs-server/src/serve.rs`.

- [ ] **`Core\Request::clientIp(): tainted string` — the five edits** (spec § 15, ADR 0097 § 6,
      ADR 0024). Row beside `method`'s at `crates/nvs-stdlib/src/request.rs:196`, card beside
      `crates/nvs-stdlib/src/request.rs:310`, body beside `crates/nvs-stdlib/src/request.rs:1424`,
      the `address()` arm at `crates/nvs-stdlib/src/request.rs:985`, three `.nvst` cases. It reads
      `nvs_runtime::Inbound::client` (`crates/nvs-runtime/src/ctx.rs:4593`), and **`None` is a real
      answer** — a Unix-domain peer that forwarded nothing — so what the member says for it is this
      slice's call and belongs in the card's `ret`.
- [ ] **`Core\Request::scheme(): string` — the same five**, reading
      `nvs_runtime::Inbound::scheme` (`crates/nvs-runtime/src/ctx.rs:4598`) and answering `"http"`
      or `"https"` off `nvs_runtime::Scheme` (`crates/nvs-runtime/src/ctx.rs:4369`). The qualifier
      is the one decision here: the value is the *server's* answer about who was allowed to speak,
      not the peer's word, which is the argument for `Qual::Neutral` — record it on the row rather
      than in the handoff.
- [ ] **§ 6's one `Warn` is computed and unreported.** `Origin::ignored_forwarded`
      (`crates/nvs-server/src/forwarded.rs:267`) is read nowhere; the comment naming it as owed is
      at `crates/nvs-server/src/serve.rs:577`, and what it needs is somewhere for this loop's
      reports to go.

## Backlog

- ADR 0088 § 4's `html` and `sendFile` rows are unlanded — `crates/nvs-stdlib/src/response.rs`'s
  module doc owns the gap, and the new test's sweep covers each as it joins.
- `Core\Request::host()` waits on whether a forwarded host may be believed — nothing decides it
  today (docs/adr/0097 § 6).
- `mount`/`route` wait on the match `nvs_server` makes once (ADR 0102 § 1).
- Raw/unparsed body access for an arbitrary content type — `docs/agent/loop-goal.md`'s standing
  decisions, narrowed by `docs/plan/m7.md`.
- **`orient.py` gap:** the pack printed ADR 0088 § 3 but not § 4, and § 4's five-member table is
  what the driver's failed check was about — add `0088` § 4 to `[context] adrs`.
