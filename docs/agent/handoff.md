# Handoff

## State

**Goal 6, and the driver's failed acceptance check is closed.** `nvs-types (everything from outside is
tainted)` named three tests and only the first was ever a type question; all three are now filed under
the crate that owns the rule, and two of them have landed.

**The request tree's taint is asserted as a partition** —
`every_request_member_returning_outside_data_returns_it_tainted` at
`crates/nvs-types/src/core_lib.rs:1262`. Every member of `Core\Request` and of the four classes under it
falls into marked, walks-a-marked-element, hands-on-to-a-class-in-this-sweep, or plain; `plain` is
asserted as an exact four, so a row added tomorrow answering a bare `string` off the wire lands there.
`query` is one of the four and is the documented hole — § 9's brackets, and no tainted array.

**ADR 0105 § 3 is whole**: `a_part_is_consumed_by_read_all_by_iteration_or_by_save_to` at
`crates/nvs-stdlib/src/request.rs:3702`. The three consumers are read off `PART`'s rows by return type
rather than written out, and the same wire's octets come back equal through all three.

**`Core\Request::post()` still has not landed**, which is the next group's whole content —
`crates/nvs-stdlib/src/multipart.rs:231` says so in the `#[allow(dead_code)]` reason on `fields()`.

`[context] adrs` gained `0105` §§ 2-4 this session rather than only being asked for. ADR 0138 § 1 was
asked for by the last handoff and is deliberately **not** added: the connection-seam work it was needed
for has landed, so it would cost every future pack for nothing.

## Next group

**ADR 0105 § 2's form fields — `Core\Request::post()`, the member and then its test.** One file set:
`crates/nvs-stdlib/src/request.rs`, `crates/nvs-stdlib/src/multipart.rs`,
`tests/conformance/core/`.

- [ ] **`Core\Request::post()` — the five edits** (§ 2). A non-file part is a form field and is
      buffered rather than streamed, so this is the member that reads them. The parse half already
      exists and is unreachable: `crates/nvs-stdlib/src/multipart.rs:231` is `fields()`, dead-coded
      pending exactly this. `crates/nvs-stdlib/src/request.rs:216` is `query`'s row and the shape to
      copy — same `mixed` answer and the same § 9 bracket convention, so the same reason there is no
      qualifier on it; `crates/nvs-stdlib/src/request.rs:270` is where the row goes, beside `files`;
      `crates/nvs-stdlib/src/request.rs:336` is `QUERY_DOC`, the card to copy; and
      `crates/nvs-stdlib/src/request.rs:933` is the `address()` arm a miss turns into a runtime panic.
      Three `.nvst` cases, each asking a different question. **Decide first** whether a body already
      claimed by `body()`/`files()` refuses `post()` — `nvs_runtime::Inbound::claim_body` is the rule
      and `a_body_is_claimed_by_the_member_that_read_it_and_refused_to_the_other` is the existing
      shape.
- [ ] **`a_non_file_part_is_buffered_into_post`** (§ 2). The claim under the member: a part with no
      `filename` never reaches `files()` and does reach `post()`, and it is held whole rather than
      walked. `crates/nvs-stdlib/src/request.rs:3777` is the `saving`/`scratch`/`save_to` fixture
      block and `crates/nvs-stdlib/src/request.rs:3013` is the walk fixture — `UPLOAD` carries two
      *file* parts today, so this slice adds the mixed body beside it.
- [ ] **The bound on what `post()` holds** (§ 2, § 5). A buffered field is resident memory, so the
      total has a cap and the cap is asserted on both sides — the last accepted body and the first
      refused one. `crates/nvs-stdlib/src/request.rs:2785` is
      `the_request_body_cap_is_the_last_body_read_and_the_first_one_refused`, the shape to follow, and
      `crates/nvs-stdlib/src/request.rs:1374` is `REQUEST_BODY`.

## Backlog

- `client_ip_and_scheme_come_from_the_peer_unless_a_trusted_proxy_asserted` — ADR 0097 § 6's forwarded
  walk, unlanded; `crates/nvs-server/src/serve.rs:538` is the one line it changes.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024 *Revisiting*, narrowed by
  `docs/plan/m7.md`.
- ADR 0105's load-bearing M7 case: a multipart body far larger than any in-memory bound received at
  bounded resident memory, asserted against a high-water mark — `docs/plan/m7.md` § *Verify*.
