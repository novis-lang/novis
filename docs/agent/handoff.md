# Handoff

## State

**Stage 2's `[2 filesystem]` acceptance check is closed** — all seven of its named tests exist and
pass. `no_member_dispatches_on_a_uri_scheme` is the last of them, in
`crates/nvs-stdlib/tests/capability.rs`, and it is three claims rather than a blocklist of PHP's
scheme names: no member of a *path-taking* class names a scheme, a wrapper, a protocol or a URL in
its signature; those classes' own modules carry no `://` on a shipped line; and
`php://filter/resource=registry.rs` resolves to no file at the launderer or at the grant. The
path-taking set is derived — every class with an `fs.read`/`fs.write` row in
`registry::CAPABILITIES`, plus `Core\Path`, which has no row precisely because it touches nothing.

**`Core\IO\File` is five members**: `read`, `readLine`, `write`, `flush`, `close`. `readLine`'s doc
comment in `crates/nvs-stdlib/src/io.rs` owns both decisions — it reads a `LINE_CHUNK` and seeks the
tail back rather than reading a byte at a time (there is no `BufReader` between calls, because the
descriptor lives in the request's table), and it consumes the terminator and never returns it, so
`crate::str::line_pieces` stays the one place that decides what a line is. `flush` is
`Write::flush` and says so: Novis buffers nothing in front of the descriptor, and the member is
deliberately not `sync_data`, because durability is a stronger promise than `fflush`'s and would
price every port at an `fsync`.

**What stage 2 still owes**: § 14's remaining handle roster — `seek`, `tell`, `truncate`, `lock` —
and `Core\IO::stdin`/`stdout`/`stderr`, each a signature over the same slot with nothing new to
decide. Stage 8's reflective property write is untouched and stays in the backlog.

**The orientation pack still does not print `docs/spec/01-core-library.md`**, which is in no
`[context]` field; § 14's roster line is what a handle member's signature comes from, and this
session paid a call for it as the last one did.

## Next group

**The rest of § 14's handle roster, over `crates/nvs-stdlib/src/io.rs`,
`crates/nvs-stdlib/src/registry.rs` and one new `.nvst` — the same file set this session held, and
the five edits of *A `Core` member* per row. Spec § 14's *Handles* bullet is the member list.**

- [ ] **`Core\IO\File::seek` and `::tell`** — the pair that makes a handle random-access, and the
      two that `readLine`'s seek-back already depends on internally. `seek` takes a `uint` offset
      from the start (no `whence` flag — ADR 0063 R3 refuses a mode argument); `tell` answers the
      position. Rows and cards beside `readLine`'s at `crates/nvs-stdlib/src/io.rs:770`, bodies
      beside it at `crates/nvs-stdlib/src/io.rs:1253`, the `address()` arms at
      `crates/nvs-stdlib/src/io.rs:984`, and the `None` rows at
      `crates/nvs-stdlib/src/registry.rs:1374`.
- [ ] **`Core\IO\File::truncate`** — the same five edits over the same anchors, and the one member
      of the four whose capability is arguably not `open`'s: decide in the row's comment whether
      shortening a file the handle already holds for writing needs anything the descriptor did not
      already prove. `crates/nvs-stdlib/src/io.rs:770` and
      `crates/nvs-stdlib/src/registry.rs:1374`.
- [ ] **`Core\IO::stdin`, `::stdout` and `::stderr`** — three statics answering the same
      `Core\IO\File`, which is the first handle in the table that was never `open`ed; the decision
      is what key they are filed under and whether closing one is refused.
      `crates/nvs-stdlib/src/io.rs:225` is `open`'s row, and `crates/nvs-runtime/src/ctx.rs`'s
      `hold_open_file` owns the table.

## Backlog

- `Core\IO\File::lock` — spec § 14's last handle member, and the only one that blocks; needs a
  decision about a request that holds a lock when it ends.
- Stage 8's reflective property write — `docs/agent/loop-goal.toml`'s stage 8 comment, finding intact.
- `docs/agent/loop-goal.toml`'s `[context]` gains `docs/spec/01-core-library.md` as a selector, and
  `crates/nvs-test`'s module doc for `--FILE nvs.toml--`.
