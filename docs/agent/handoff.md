# Handoff

## State

**Goal 20's Stage 3 is landed as code; three of its proofs are not.** `[cache.shared] url` takes
`unix:/path/to.sock` beside `redis://host[:port]`: `endpoint` in `crates/nvs-stdlib/src/cache.rs`
answers a `Target` — `Tcp(SocketAddr)` or, `#[cfg(unix)]`, `Socket(PathBuf)` — which is the per-core
key `open_shared` compares to decide reuse-or-replace, so a build with no `AF_UNIX` transport cannot
hold a path to dial rather than refusing one at dial time. `cache/redis.rs` is transport-agnostic
through one `Transport` enum carrying the deadline, the write and the read; every line above it is
written once.

`E0635` (`E_NO_UNIX_TRANSPORT`) is declared at the end of its band and raised by
`nvs_config::store::validate`, hung off `resolve` immediately before `store::advise`. **The goal's
prose asked for `E0627`, which was already `E_UNSPELLED_EXPORTER`** — the prose now says `E0635`, and
the playbook holds the trap. The scheme string has one home, `nvs_config::store::UNIX_SCHEME`, which
`cache.rs`'s `UNIX` is an alias of.

Passing on both platforms, each asserting the side its build is on:
`a_unix_url_is_refused_where_the_platform_has_no_transport` (`nvs-stdlib`, the door) and
`a_unix_url_is_refused_at_boot_only_where_there_is_no_transport` (`nvs-config`, the boot). Nothing is
blocked. The `unix:` and `[db.<name>] host` rules stay `designed` until Stage 4 lands the driver half.

## Next group

**Stage 3: the three proofs, one of which the acceptance list cannot pass as written** — one file
set: `crates/nvs-stdlib/src/cache/redis.rs`, `crates/nvs-stdlib/src/cache.rs`, `examples/`,
`nvs.toml`.

- [ ] **`a_record_written_through_a_socket_is_read_back_through_it`** —
      `crates/nvs-stdlib/src/cache/redis.rs:642`, a `UnixListener` sibling of `listening()` driving
      one `SET`/`GET` exchange over `Target::Socket`, per
      `rule:config/unix-scheme-in-a-url-and-a-bare-path-in-a-host`. The `#[test]` must **exist** on
      Windows or the `cargo-named` check reads it as unwritten on that leg, so gate the body and not
      the item — `a_unix_url_is_refused_where_the_platform_has_no_transport` in
      `crates/nvs-stdlib/src/cache.rs` is the shape.
- [ ] **`a_program_supplied_socket_path_is_never_a_target`** —
      `crates/nvs-runtime/src/capability.rs:218`, `pinned_address`, which is the door every
      program-supplied endpoint passes: a path is refused **as a target this deployment cannot
      authorize** and never as a file that would not open, per
      `rule:config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it`. The acceptance list
      files the name under `-p nvs-stdlib`, so it is asserted through a `Core` door and not on
      `nvs-runtime`'s own function.
- [ ] **`examples/cache-shared-socket.nvs` and the config that points it at a socket** —
      `docs/agent/loop-goal.toml:6006`. The check is `kind = "exact"` and program checks run on
      **every** leg, so as written it can never pass on Windows, which has no `AF_UNIX` transport;
      a fixture printing "across the socket" over loopback TCP instead is exactly the dishonesty
      `rule:config/a-unix-spelling-with-no-af-unix-transport-refuses-at-boot` refuses. Prefer giving
      a `[[check]]` a leg scope — `tools/loop.py:1688` is the per-kind field table and
      `tools/loop.py:2229` the runner — over weakening the fixture. A second store cannot be spelled
      in the repo's own `nvs.toml`, so the example needs its own directory and config;
      `examples/capability/` is that shape.

## Backlog

- Stage 4, the drivers' socket transport — `docs/agent/goals/20-unix-sockets.md` § *Stage 4*.
- `docs/agent/loop-goal.toml:140`'s comment still names `E0627`; left alone deliberately, because
  editing that file invalidates the driver's whole green cache (`tools/loop.py:2151`).
- `nvs.toml`'s `[cache.shared]` block documents only the `redis://` spelling —
  `crates/nvs-config/src/tree.rs:949` now documents both.
- The pack has no field naming the *current stage's own* acceptance checks; the failing one it prints
  was enough to find the rest in `docs/agent/loop-goal.toml`.
