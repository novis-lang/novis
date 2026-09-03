# Handoff

## State

**Goal 6, Stage 2 is closed and Stage 3 is next: `nvs serve <file>` starts a core and answers.**
The subcommand resolves the tree, reads ADR 0097 § 5's `[server] listen` and the four waits,
compiles the entry **before** the socket is bound, and hands `nvs_server::serve_on_this_core` a
handler that answers every request with that program as ADR 0006's isolate. `nvs serve
examples/serve.nvs --port 8123` answers `200` with the file's own `echo` output, over as many
requests as are made of it.

**On disk.** `crates/nvs-cli/src/serve.rs` is the whole command; its module doc owns the two
decisions in it — the entry named on the command line is § 2's enumerated set, of one, and the
first `listen` entry is what one core binds, with `--listen`/`--port` as § 5's last word.
`nvs_config::server::listen_on` (`crates/nvs-config/src/server.rs:228`) classifies § 5's overload
— an entry beginning with a path separator is `Listen::Unix`, everything else is a literal
`SocketAddr` — and `E0620` refuses a host *name*, an unparseable entry and a written empty array.
`server::validate` (`crates/nvs-config/src/server.rs:87`) now runs both halves, so `nvs config
check` refuses a `listen` the server could not bind.

**Two limits that are the next slices' rather than defects.** A Unix-domain entry classifies and
is then refused in the CLI, because `nvs_host::NvsListener` accepts on TCP alone. And a served
request's context carries **no configuration snapshot**: `serve_on_this_core` gives each
connection a bare `Ctx::new(OutputSink::Sink)` (`crates/nvs-server/src/serve.rs:317`), so
`Core\Config` inside a served program is empty until the mount slice threads the snapshot through.

**The driver's acceptance sweep is truncated, and it is not a regression.** `native
examples/upload.nvs` is checked against stage 5's frozen `want`. The playbook's bullet on a
`loop-goal.toml` fixture check frozen ahead of the frontier owns it.

**`[context] adrs` is missing `0097 §3`** — the mount globs' own section. § 4 alone does not say
what a `scan` glob may match or how far it may reach, and the first slice below cannot be written
without it.

## Next group

**The mount table: ADR 0097 § 3's globs expanded at boot, and § 4's five steps.** One file set:
`crates/nvs-server/src/mount.rs` (new), `crates/nvs-server/src/serve.rs`,
`crates/nvs-config/src/tree.rs`, `crates/nvs-cli/src/serve.rs`.

- [ ] **A `[[server.mount]]` glob is expanded against disk at boot into a literal table.** ADR 0097
      §§ 2-3. The block is `crates/nvs-config/src/tree.rs:797`'s `Mount` under
      `crates/nvs-config/src/tree.rs:758`'s `root`; refuse a bad one beside
      `crates/nvs-config/src/server.rs:87`'s `validate`, next code `E0621`. Pins
      `a_mount_globs_is_expanded_against_disk_at_boot`.
- [ ] **The handler selects a mount rather than being handed one file.** ADR 0097 § 4's five steps,
      in order. `crates/nvs-server/src/serve.rs:286` grows the table beside the handler, and
      `crates/nvs-cli/src/serve.rs:141`'s single-entry handler and
      `crates/nvs-cli/src/serve.rs:116`'s boot compile are what it replaces. Pins
      `a_request_resolves_through_the_five_steps_in_order` and
      `a_prefix_is_stripped_and_the_module_is_relocatable`.
- [ ] **Static serving is one policy in both deployments.** ADR 0097 § 4's own paragraph — exact
      file, never a listing, `no-cache` with a strong `ETag` over `(size, mtime_nanos)`, one
      `Range`, and a `.nvs` never served as source. Switched on
      `crates/nvs-config/src/tree.rs:769`'s `serve_static`, and a file's bytes become
      `crates/nvs-server/src/serve.rs:93`'s `Answer` exactly as a request's do. Pins
      `static_files_are_one_policy_in_both_deployments`.

## Backlog

- The served request's `Ctx` has no snapshot — thread one through `serve_on_this_core`; `serve.rs`'s
  § *What this module does not decide yet*.
- `max_in_flight` as ADR 0106 § 13's arithmetic, with the core count — same section's own note.
- A graceful drain: `keep_serving` is `Continue` forever, and ADR 0078 § 6's control socket is what
  would ask it to stop.
- A Unix-domain listener in `nvs-host`, which retires the CLI's refusal — ADR 0097 § 5.
- Raw body access for an arbitrary content-type — ADR 0024 *Revisiting*, narrowed by m7.md.
