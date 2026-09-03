# Handoff

## State

**Goal 6, Stage 3 is half landed: the mount table exists on both sides of the seam.**
`nvs_config::mount` reads `[[server.mount]]` into the literal set of entry files a server may
execute, with every `scan` glob already walked against the disk; `nvs_server::mount` is ADR 0097
§ 4's five steps over that table. `nvs serve <file>` now *selects* through those steps instead of
being handed one file.

**On disk, the boot half.** `crates/nvs-config/src/mount.rs` — `check` is everything a block can be
wrong about with no disk and runs inside `server::validate`, so `nvs config check` refuses a
malformed mount on a machine holding none of the files; `expand` walks the globs and is the server's
own boot step. `E0621` is every refusal, and the module doc owns why the split is where it is.
`expand` has no production caller yet: the next group's second item is what gives it one.

**On disk, the request half.** `crates/nvs-server/src/mount.rs` — `Table::select` runs steps 1-5 in
order and answers `What::Static` (a file to send) or `What::Run` (a file to run), or `None` for
step 1's 404. Steps 3 and 4 are the only place in the crate where a remainder meets a filesystem;
the module doc § *What a remainder may be* owns the lexical refusal and the canonical containment
check that make that keep § 2 rather than spend it. A handler now answers `nvs_server::Reply` —
`Run(Isolate)` or `Done(Response)` — because step 1's 404 and step 3's file are not programs.

**Two decisions recorded in module docs rather than an ADR.** An unwritten `dispatch`/`static` reads
as the *production* pair, not development's, until the mode slice resolves ADR 0091 § 3a's defaults
(`crates/nvs-server/src/mount.rs`'s § *Decision*). And `nvs serve <file>` is § 4's table with one
row in it, mounted at `/` with the file's own directory as the mount root
(`crates/nvs-cli/src/serve.rs`'s § *Decision*).

**Unchanged limits.** A served request's context still carries no configuration snapshot
(`crates/nvs-server/src/serve.rs:374` hands each connection a bare `Ctx::new(OutputSink::Sink)`), a
Unix-domain `listen` entry still classifies and is then refused in the CLI, and the driver's
`native examples/upload.nvs` failure is stage 5's frozen `want` ahead of the frontier — the
playbook's bullet on that owns it.

## Next group

**The static file policy and the deployment table.** One file set:
`crates/nvs-server/src/mount.rs`, `crates/nvs-server/src/serve.rs`,
`crates/nvs-cli/src/serve.rs`, `crates/nvs-config/src/mount.rs`.

- [ ] **Static serving is one policy in both deployments.** ADR 0097 § 4's own paragraph — exact
      file, never a listing, `no-cache` with a strong `ETag` over `(size, mtime_nanos)`,
      `If-None-Match`, one `Range` and a refused multi-range, a fixed extension table with
      `application/octet-stream` for an unknown one. The selection already exists:
      `crates/nvs-server/src/mount.rs:255`'s `What::Static`, turned into
      `crates/nvs-server/src/serve.rs:99`'s `Answer` and handed back as
      `crates/nvs-server/src/serve.rs:151`'s `Reply::Done`; the CLI's temporary fallback to the
      mount's entry is `crates/nvs-cli/src/serve.rs:188`. Pins
      `static_files_are_one_policy_in_both_deployments`.
- [ ] **`nvs serve` boots the whole `[[server.mount]]` table, not a table of one.**
      `crates/nvs-config/src/mount.rs:207`'s `expand` is the call; `crates/nvs-cli/src/serve.rs:293`'s
      `one_mount` is what it replaces or falls back to, and the boot compile above it has to become
      one per mounted entry so § 2 still holds before the socket is bound.
- [ ] **A served request's context carries the configuration snapshot.** `Core\Config` inside a
      served program is empty today: `crates/nvs-server/src/serve.rs:374` is the bare `Ctx`, and the
      selected mount's `origin` and `captures` (`crates/nvs-server/src/mount.rs:244`) are what
      `Core\Router::urlAbsolute` and `Core\Request::mount()` read off it.

## Backlog

- § 6's forwarded-header walk replaces `Table::select`'s host — ADR 0097 § 6.
- `max_in_flight` as ADR 0106's arithmetic, and the accept backoff — ADR 0097 § 5.
- § 3's last paragraph: a literal `urlAbsolute` in a mount with no `origin` is a boot error, and it
  needs the compiled unit — `crates/nvs-config/src/mount.rs`'s § *What is not here yet*.
- One listener fanned out over several cores — `nvs_host::NvsListener::from_std`.
- A Unix-domain listener, which is what unblocks `listen`'s already-classified `Listen::Unix`.
