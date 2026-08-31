# Handoff

## State

**Stage 2's `[2 filesystem]` acceptance check has one test left**: five of its seven named tests
exist now, and `no_member_dispatches_on_a_uri_scheme` (ADR 0052) is the only one that has never
been written. It is the driver's next failure and the next group's first item.

**`Core\IO::writeStream` is on disk** — spec § 14's streaming write and ADR 0105 § 4's two rules.
`crates/nvs-stdlib/src/io.rs`'s `stream_to_disk` is the member and its doc comment is the one home
of both decisions: nothing is materialised (one chunk is held, never the stream), and a failure
part-way removes the partial file. A sibling temporary renamed into place was rejected there, in
writing — it needs a grant for a second name the program never chose, and it turns `overwrite:
false` into a check followed by a rename.

**Two doors are new, one per crate.** `nvs_runtime::capability::create` is the streaming-write door:
`fs.write` on the path, then `create_new` when `overwrite` is false, so the refusal is `O_EXCL` in
the kernel rather than an `exists` call with a window after it. `nvs_runtime::sequence::for_each` is
the one-element-at-a-time drive, and `drain` is now written over it, so there is a single cursor
loop; its doc comment owns the ownership contract, which is that **the sink owns every value handed
to it, including on the call it fails**.

**What stage 2 still owes** after the URI-scheme sweep: § 14's remaining handle roster — `readLine`,
`seek`, `tell`, `truncate`, `flush`, `lock` — and `Core\IO::stdin`/`stdout`/`stderr`, each a
signature over the same slot with nothing new to decide. Stage 8's reflective property write is
untouched and stays in the backlog with its finding intact.

**The orientation pack was missing two things** this session paid a call each for, both worth a
`[context]` selector: spec § 14's own row for the member being written (`docs/spec/01-core-library.md`
is in no manifest field), and `crates/nvs-test`'s module doc, which is where a `.nvst` learns it can
grant a capability with `--FILE nvs.toml--`.

## Next group

**The rest of stage 2, over `crates/nvs-stdlib/src/io.rs`, `crates/nvs-stdlib/src/registry.rs` and
`crates/nvs-stdlib/tests/capability.rs` — one file set, and the same three files this session held.
Spec § 14 is the member list and `docs/agent/loop-goal.toml:2221` is the check.**

- [ ] **`no_member_dispatches_on_a_uri_scheme`** — ADR 0052's closed door on stream wrappers,
      asserted by construction over landed work: no `Core` member takes a scheme, and no path
      argument is parsed as one. A sweep over `crates/nvs-stdlib/src/registry.rs:1340`'s roster
      beside its neighbours in `crates/nvs-stdlib/tests/capability.rs:316`. This closes the
      driver's failing check and outranks the two below.
- [ ] **`Core\IO\File::readLine` and `::flush`** — spec § 14's handle roster, two rows on
      `crates/nvs-stdlib/src/io.rs:750`'s `FILE`, cards beside
      `crates/nvs-stdlib/src/io.rs:789`, bodies beside `crates/nvs-stdlib/src/io.rs:1134`. Each
      needs three conformance cases; `tests/conformance/core/io-write-stream-*.nvst` is the shape,
      `--FILE nvs.toml--` and all.
- [ ] **`Core\IO\File::seek`, `::tell` and `::truncate`** — the same five edits over
      `crates/nvs-stdlib/src/io.rs:750`, `crates/nvs-stdlib/src/io.rs:789` and
      `crates/nvs-stdlib/src/io.rs:1134`, and the one decision between them is what a seek past the
      end answers.

## Backlog

- `Core\IO::stdin`/`stdout`/`stderr` answer `Core\IO\File` too — `crates/nvs-stdlib/src/io.rs:750`.
- Stage 8's reflective property write — `ClassInfo::call` landed; the write half is untouched
  (ADR 0019).
- `Core\IO\File::lock` — the one handle member with a capability question of its own to settle.
- Spec § 14's `append`, `copy`, `move`, `makeDir`, `list`, `walk` are unwritten (`docs/spec/01-core-library.md` § 14).
- `docs/agent/loop-goal.toml`'s `[context]` gains `docs/spec/01-core-library.md` and
  `crates/nvs-test/src/lib.rs`'s format docs.
