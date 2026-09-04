# Handoff

## State

**Goal 6, Stage 4 item 2 is closed: `Core\Response::setStatus` sets a status and it crosses to
the peer.** Five edits and one channel, and each end owns its own reasoning:
`crates/nvs-stdlib/src/response.rs`'s module doc has a new section — *A status crosses that same
channel, and is not a body* — that is the home for every decision below.

**The channel is `content_type`'s, twice over rather than widened.** `Ctx::status` is a second
`Option<u16>` field beside `Ctx::content_type` (`crates/nvs-runtime/src/ctx.rs:889`), taken by
`take_status` on the same finish path, carried on `Completion::status`
(`crates/nvs-runtime/src/host.rs:274`) and turned into a status line by `nvs_server`'s `answer`.
Two fields and not one struct: every body member sets a media type and exactly one member sets a
status, so a single declaration would let a body member reach a status it has no business setting.

**A status is not a body, so `E0801` does not reach it.** `nvs_types::response`'s `BODY_MEMBERS`
is five names and `setStatus` is not among them, so `setStatus` beside an `echo` is the ordinary
spelling and needed no edit in `nvs-types` at all.

**Two decided-and-recorded calls.** The parameter is `CoreTy::Uint` — ADR 0007 § 4's type refuses
a negative at compile time — and the admitted range is **100 to 599**, `LogicError` outside it:
RFC 9110 § 15 gives a status a class from its first digit and there are five, so a `6xx` is three
digits nothing downstream has a rule for. `hyper` would carry up to `999`; refusing at the member
is `spellable`'s direction for a media type, at the same layer. **A request that failed answers
`500` whatever it declared** — `answer` reaches `failed()` before it reads the field.

**Unchanged limits.** A status set off a request declares onto a context nobody asks — module gap
1's reasoning, now stated for this member too. A `.nvst` case cannot observe a status line, so the
crossing is pinned by `crates/nvs-server/src/serve.rs`'s
`a_declared_status_is_the_responses_status_and_a_failure_outranks_it`, beside the two content-type
tests that exist for the same reason.

**`[context]` gaps still open**, now reported four times: `adrs` prints `0097 §2`, `§4` and
`0106 §13` but not **`0097 §5`**, and **`0088 §§ 3-4`** is still not printed. `modules` still names
no `nvs-cli` pattern. New this session: `shapes` prints the `Core` member's five edits but nothing
about **spec § 15**, which is prose rather than a `| Member |` table — so a § 15 member has no
signature column and `every_registry_rows_names_are_the_specs_signature_column` never sees it. A
`[context] spec` selector for § 15 would have saved two reads.

## Next group

**Stage 4 item 3 and its neighbour: the headers a response carries** — spec § 15 and ADR 0074.
One file set, and it is exactly this session's, all of it already anchored:
`crates/nvs-stdlib/src/response.rs`, `crates/nvs-runtime/src/ctx.rs`,
`crates/nvs-host/src/isolate.rs`, `crates/nvs-server/src/serve.rs`.

- [ ] **`setHeader`, a header sink over that same channel** — spec § 15 says it overrides a
      policy-owned header on one response, so unlike a status it is a *map* and the crossing is a
      list rather than a word. `crates/nvs-runtime/src/ctx.rs:4128` is `declare_content_type` and
      `crates/nvs-runtime/src/ctx.rs:4141` is `declare_status`, the two neighbours a header store
      sits beside; `crates/nvs-runtime/src/host.rs:274` is where the `Completion` field goes;
      `crates/nvs-server/src/serve.rs:445` is `answer`, which must apply a program's header
      **after** the policy headers for § 15's override to mean anything; and
      `crates/nvs-stdlib/src/response.rs:216` is `spellable`, which already answers what a field
      value may carry and is the check a name and a value both owe. ADR 0074's secure-header set is
      what "policy-owned" names — decide and record whether `Content-Type` is reachable this way,
      since `bytes` already owns it.
- [ ] **`addCookie`, whose options shape defaults every field from `[http.cookies]`** — spec § 15,
      ADR 0074. `crates/nvs-stdlib/src/response.rs:97` is the row block and
      `crates/nvs-stdlib/src/response.rs:216` is `spellable` again; a cookie is a `Set-Cookie`
      header, so this rides whatever `setHeader` builds rather than opening a third channel.
      `SameSite` is an enum and never a string, which needs a registry enum in the row.
- [ ] **A `.nvst` case that a status and a header survive a throw** — the completion carries both
      out of `finish` on the throwing path (`crates/nvs-host/src/isolate.rs:473`), and nothing yet
      asks whether it should. `crates/nvs-server/src/serve.rs:452` is where `answer` decides it
      does not, which is the half a case can reach only through the server's own tests.

## Backlog

- `html` and `sendFile`, § 4's remaining two body members — `crates/nvs-stdlib/src/response.rs`'s
  module doc names what each is blocked on.
- Two typed body members in one handler are not refused — `nvs_types::response`'s module doc.
- The entry-script half of § 4's sixth row — `nvs_types::response`'s module doc.
- Stage 5's uploads: the driver's `native examples/upload.nvs` check is that stage's frozen `want`,
  not a regression — ADR 0105 is its specification.
- Raw/unparsed body access for an arbitrary content-type — `docs/plan/m7.md`, ADR 0024's
  *Revisiting*.
