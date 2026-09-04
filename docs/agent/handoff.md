# Handoff

## State

**Goal 6, M7. ADR 0139 § 1's `start` is on disk and green.** `Core\Session::start(?tainted string
$presented = null): void` is registered, implemented and asked by three conformance cases. The
record has somewhere to live: `nvs_runtime::Session` — an id, ADR 0023's byte record and a dirty
flag, and no `Value`, so a context has nothing to release at teardown
(`crates/nvs-runtime/src/ctx.rs:1146`).

**The driver's stage-5 failure is not a regression.** `a_schedule_entry_fires_as_a_root_isolate` is
genuinely unwritten — `grep` over `crates/nvs-server/src` finds no schedule surface at all — so it
is an item still open, which `orient.py` calls this goal's ordinary state.

**One slice of three, deliberately.** Slice 2 is a *hard* slice: three members, a record codec
whose ABI this session never read, and **nine** conformance cases under
`conformance_coverage.rs`'s per-member floor of three. That is past the ~45k the context gate
budgets for one slice, and its two open questions are answered below so the next session does not
re-derive them.

## Next group

**ADR 0139 § 1's six remaining members, over one file set:**
`crates/nvs-stdlib/src/session.rs`, `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt`
and `tests/conformance/core/session-*.nvst`. Nothing else needs touching: the class is registered
(`crates/nvs-stdlib/src/registry.rs:1425`), the address chain has its arm
(`crates/nvs-stdlib/src/lib.rs:417`), and `Ctx` already carries the record.

**Two answers this session paid for, before the first item.**

*The record's encoding.* ADR 0139 § 2 says the record is ADR 0023's byte carrier, which is
`nvs_runtime::encode`/`decode` — `crates/nvs-stdlib/src/cache.rs:816` and `:856` are the two call
sites to copy, including the `ctx.class_desc` resolver `decode` needs. A record is **one encoded
array**, decoded per read and re-encoded per write; empty is zero bytes rather than the encoding of
an empty map, which `crates/nvs-runtime/src/ctx.rs:1146`'s `record` field already states and
`start` already relies on. Decode-per-read is O(record) per member call and that is the accepted
trade: a session record is a handful of keys next to the network round trip `start` already spent,
and holding a decoded `Value` on the context would put an object at teardown that ADR 0017's unit
swap could strand. Say what it spends in the members' own docs.

*What a `.nvst` case can ask.* No conformance case can reach a session store — there is no redis
under `nvs test` — so the round trip belongs in `-p nvs-stdlib` unit tests over the scripted store
already in `crates/nvs-stdlib/src/session.rs:466`'s `serving`, and the nine cases ask the
*language* rule: § 1's "a member called before `start` throws naming it". Three questions that do
not repeat: the refusal itself, the conventions' **agreement** shape (every member refuses
identically before `start`, asserted by counting rather than read off a line), and the class being
catchable at the root of spec § 10's tree.

- [ ] **`get`, `set` and `remove` over the started record** (ADR 0139 §§ 1, 4) — three rows in the
      `CLASS` at `crates/nvs-stdlib/src/session.rs:88`, three cards after `START_DOC` at
      `crates/nvs-stdlib/src/session.rs:110` in row order, three bodies and three arms in
      `address` at `crates/nvs-stdlib/src/session.rs:434`. Each reads the record through
      `Ctx::session`/`session_mut` (`crates/nvs-runtime/src/ctx.rs:1660`,
      `crates/nvs-runtime/src/ctx.rs:1667`) and throws naming `start()` while it is `None`. `set`
      and `remove` set `dirty`; `get` must not. Strike three lines from
      `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:34`.
- [ ] **`clear`, `regenerate` and `destroy`** (ADR 0139 §§ 1, 4) — same five edits at the same
      anchors. `regenerate` is `mint` (`crates/nvs-stdlib/src/session.rs:207`) plus `save` under
      the new id, then the old entry destroyed, **in that order**, plus the cookie —
      `crates/nvs-stdlib/src/session.rs:403`'s `issue_cookie` is written for exactly this second
      caller. § 2's fourth operation, `destroy`, has no implementation yet: it is one `DEL` beside
      `load` at `crates/nvs-stdlib/src/session.rs:230`, and `serving`'s RESP fixture panics on any
      command it does not know, so it needs the arm too.
- [ ] **§ 4's write-back at the end of the request** — the record is written when the request ends
      and only if `dirty`. Nothing calls it yet, so a `set` is currently lost. The hook belongs
      beside the request teardown in `crates/nvs-cli/src/serve.rs` and the door in
      `crates/nvs-server/src/serve.rs:818`; take it only after the two above, since it is the one
      item in this group that leaves the file set.

## Backlog

- `a_schedule_entry_fires_as_a_root_isolate` — ADR 0073, unwritten in `crates/nvs-server`; the
  driver reports it every iteration.
- The `db` backend: `nvs_config::session::Backend::Db` resolves and `start` throws naming the gap
  (`crates/nvs-stdlib/src/session.rs`'s module doc owns which half is on disk).
- `[context]` gained nothing this session — the pack printed everything the item named. It did not
  print `crates/nvs-stdlib/src/response.rs`, which the cookie edit needed; add it to `modules` if
  the next session touches the cookie again.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024's *Revisiting*, `docs/plan/m7.md`.
