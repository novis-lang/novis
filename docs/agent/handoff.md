# Handoff

## State

**Goal 6, M7. ADR 0105 § 2's form fields have landed: `Core\Request::post()` is on disk.** The
member reads the body **to its end** — the rule the rest of it follows from, because § 2's
*Verification* promises *every* field of a mixed form and a form may write a text input after a
file input. `crates/nvs-stdlib/src/request.rs`'s `form_of`/`multipart_form`/`urlencoded_form` are
the reading, `crate::multipart::Multipart::drain` walks the rest of a parse, and
`crate::uri::place` is now the one home of § 9's bracket convention for a query string and a form
alike.

**The one decision this needed, recorded on `claim_form`
(`crates/nvs-stdlib/src/request.rs:1027`):** `post` is a fourth reader on spec § 15's exclusivity
and it **joins** a `files` claim rather than being refused by it — that walk sets the non-file
parts aside, while `body`/`bodyStream` hand the bytes over uninterpreted and leave nothing behind,
so both of those still refuse. `post` with nothing holding the body claims it under its own name,
and `files()` afterwards is refused honestly: draining to the last field consumed the uploads.
`nvs_runtime::Inbound` gained one field for it — `form`, the urlencoded body held so a second
`post()` on one request does not read an exhausted supplier.

**The driver's failed acceptance check is closed, and it was a filing bug.**
`a_mount_carries_no_policy_of_its_own` is landed and green at
`crates/nvs-config/tests/tree.rs:105`; the check ran it under `-p nvs-server`. Both copies of the
goal file now name it under `nvs-config (the mount table)`, beside
`a_mount_globs_is_expanded_against_disk_at_boot`.

`clientIp`/`scheme`/`host`/`mount`/`route` remain this module's known gaps, on the carrier and the
match, unchanged — `crates/nvs-stdlib/src/request.rs:18` says what each waits on.

## Next group

**Whether the two remaining § 15 body facts can be reached at all, then ADR 0097 § 6's answer
carried down to `Core\Request`.** One file set: `crates/nvs-runtime/src/ctx.rs`,
`crates/nvs-server/src/serve.rs`, `crates/nvs-stdlib/src/request.rs`.

- [ ] **`Inbound` carries the peer** (ADR 0097 § 6). `nvs_server::forwarded::walk` already answers
      a client address and a `Scheme` per request at `crates/nvs-server/src/serve.rs:558`, and
      nothing carries it down: `crates/nvs-runtime/src/ctx.rs:4375` is the struct that holds a
      method, a path, a query, the header lines and the body and no peer. Add the two facts beside
      `body`, set them where the walk answers, and the three gaps at
      `crates/nvs-stdlib/src/request.rs:18` become three rows.
- [ ] **`Core\Request::clientIp()` and `::scheme()` — the five edits each** (§ 6, spec § 15).
      `crates/nvs-stdlib/src/request.rs:216` is the row shape, the card block after
      `crates/nvs-stdlib/src/request.rs:464` is where theirs go in row order, and
      `crates/nvs-stdlib/src/request.rs:1000` is the `address()` arm a miss turns into a runtime
      panic. Both answers are `tainted` — they are what a peer or a trusted proxy asserted.
- [ ] **§ 6's one `Warn` is computed and unreported** — `Origin::ignored_forwarded` at
      `crates/nvs-server/src/forwarded.rs:406`'s neighbourhood. It waits on a log this loop does
      not have, exactly as a reset peer does; decide whether that stays a gap or becomes an item.

## Backlog
- `Core\Request::post()` has no `.nvst` case that *reads* a form — a conformance program answers no
  request, so the three cases pin the refusal and `crates/nvs-stdlib/src/request.rs`'s own
  `#[test]`s pin the reading. An end-to-end one belongs to `nvs-server`, not here.
- `[limits] max_multipart_parts` is still a constant rather than a directive —
  `crates/nvs-stdlib/src/multipart.rs:61` (ADR 0095 § 4).
- Raw/unparsed body access for an arbitrary content-type — ADR 0024's *Revisiting*, narrowed by
  `docs/plan/m7.md`.
- `docs/agent/loop-goal.toml`'s `[context] adrs` still does not print ADR 0097 § 6; it prints §§ 2
  and 4. Two sessions have now needed § 6.
