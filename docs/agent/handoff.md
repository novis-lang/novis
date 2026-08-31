# Handoff

## State

**Stage 3's `[3 process and terminal]` acceptance check is closed.** `Core\Process::run` waits for
its child through `nvs_host::blocking::run` rather than on the worker — `wait_off_core` in
`crates/nvs-stdlib/src/process.rs` is that call and owns the reasoning, and the module's *Decision:
the wait happens off the core* says why a child is ADR 0106 § 6's pool case and not the reactor's.
This was the tree's only use of the blocking pool; `nvs-stdlib`'s `Cargo.toml` comment on the
`nvs-host` edge is now the two-entry list it should have been, park-on-readiness and hand-to-pool.

**`Core\IO\File` is seven members**: `read`, `readLine`, `write`, `seek`, `tell`, `flush`, `close`.
`seek` takes a `uint` offset from the start and has **no `whence`** — ADR 0063 R3 refuses the mode
argument, and `SEEK_CUR`/`SEEK_END` are spelled out of `seek` and `tell` at the call site, where a
reader can see which origin was meant. `seek`'s doc comment owns both that and the decision that
seeking past the end is not an error. The `uint` on both sides is one reading: a position is a
distance from the start, so the type refuses a negative seek before the file is asked.

**What stage 2 still owes**: `truncate` and `lock`, and `Core\IO::stdin`/`stdout`/`stderr`. Stage 8's
reflective property write is untouched and stays in the backlog.

**The orientation pack still does not print `docs/spec/01-core-library.md`**, which is in no
`[context]` field. § 14's roster line (`docs/spec/01-core-library.md:1018`) is what a handle member's
signature comes from, and this session paid a call for it as the last two did.

## Next group

**The last of § 14's handle roster and the three standard handles, over
`crates/nvs-stdlib/src/io.rs`, `crates/nvs-stdlib/src/registry.rs` and one new `.nvst` — the same
file set this session held, and the five edits of *A `Core` member* per row.**

- [ ] **`Core\IO\File::truncate`** — `truncate(uint $length): void`, spec § 14's roster line. The one
      member that shortens *or* extends a file without moving the handle, which is the decision to
      write down: `File::set_len` does both, and a caller's position is untouched by either, so a
      `tell` past the new end stays where it was. Row and card beside `seek`'s at
      `crates/nvs-stdlib/src/io.rs:792` and `crates/nvs-stdlib/src/io.rs:903`, body beside it at
      `crates/nvs-stdlib/src/io.rs:1430`, the `address()` arm at
      `crates/nvs-stdlib/src/io.rs:1055`, the `None` row at
      `crates/nvs-stdlib/src/registry.rs:1379`.
- [ ] **`Core\IO\File::lock`** — the roster's last handle member. Decide the shape first: PHP's
      `flock` is a mode `int` plus a `&$wouldBlock` out-parameter, and R3 refuses the first while R7
      refuses the second, so this is one member answering `bool` or a pair of them. The same five
      anchors as above, at `crates/nvs-stdlib/src/io.rs:792` and
      `crates/nvs-stdlib/src/registry.rs:1379`.
- [ ] **`Core\IO::stdin`, `::stdout` and `::stderr`** — three statics answering the same
      `Core\IO\File` over the same slot, with the descriptor held rather than opened, so nothing
      passes `capability::open`. Static rows go beside `open`'s at
      `crates/nvs-stdlib/src/io.rs:223` with its card at `crates/nvs-stdlib/src/io.rs:631`, and the
      `open` body they diverge from is at `crates/nvs-stdlib/src/io.rs:1183`.

## Backlog

- Stage 8's reflective property *write* — `docs/adr/0019-reflection-and-ast-parsing-are-core-features.md`.
- `[limits] max_output` bounds neither `Core\Process`'s capture nor `Core\IO::read` — both modules' known gaps.
- `docs/spec/01-core-library.md` is in no `[context]` field of `docs/agent/loop-goal.toml`.
