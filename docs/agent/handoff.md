# Handoff

## State

**Goal 6, Stage 3 is closed on the request path: the table selects, and the server answers.**
ADR 0097 § 4's static paragraph is `crates/nvs-server/src/statics.rs` — `send` at
`crates/nvs-server/src/statics.rs:149` takes a file, the request's headers and a [`Source`], and
answers the exact file with `no-cache`, a strong `ETag` over `(size, mtime_nanos)`,
`If-None-Match`, one `Range`, a refused multi-range and a fixed extension table. **Nothing in it
takes a mode or a switch**, which is how "one policy in both deployments" is kept rather than
asserted; that module's own docs own the reasoning, including why every `Range` it cannot honour
exactly is a `416` and never a silent `200`.

**§ 4's sole default document is in the selection, not the sending.**
`crates/nvs-server/src/mount.rs:358`'s `default_document` gives step 3 an `index.html` for a
remainder that spells a directory *below* the mount root, and never at the root itself, where step
5's entry is what a mount means (`statics.rs`'s § *Decision*).

**`nvs serve` boots the whole `[[server.mount]]` table.** `nvs_config::mount::expand` has its first
production caller (`crates/nvs-cli/src/serve.rs:135`): a tree that writes mounts is served through
all of them, every mounted entry is compiled before the socket is bound, and a `<file>` the table
does not mount is refused rather than silently serving another application. A tree that writes none
still gets `one_mount`'s table of one. `crate::config::boot_origins` now keeps the origins map, so
`waits_for`, `listen_on` and `expand` all resolve and refuse against the file a value was written
in rather than against the cwd.

**Unchanged limits.** A served request's context still carries no configuration snapshot
(`crates/nvs-server/src/serve.rs:387` is the bare `Ctx::new(OutputSink::Sink)` each connection
gets), a Unix-domain `listen` entry still classifies and is then refused in the CLI, and the
driver's `native examples/upload.nvs` failure is stage 5's frozen `want` ahead of the frontier —
the playbook's bullet on that owns it. `orient.py` printed no map line for `crates/nvs-cli/src/serve.rs`
although the item named it: `[context] modules` in `docs/agent/loop-goal.toml` wants an `nvs-cli`
pattern.

## Next group

**The `[server]` block's bounds — the four names stage 3's check still misses.** One file set:
`crates/nvs-server/src/serve.rs`, `crates/nvs-server/src/io.rs`, `crates/nvs-config/src/tree.rs`,
`crates/nvs-config/src/server.rs`.

- [ ] **`max_in_flight` is an arithmetic against the memory budget, and an over-capacity request is
      refused with `503` before it is allocated.** ADR 0097 § 5 as amended by ADR 0106 — the
      directive is `crates/nvs-config/src/tree.rs:775`, the place a request becomes an isolate is
      `crates/nvs-server/src/serve.rs:248`, and `crates/nvs-server/src/serve.rs:55` is the module-doc
      bullet that currently says neither is here. Pins
      `max_in_flight_is_derived_from_the_memory_budget` and
      `an_over_capacity_request_is_refused_with_503_before_it_is_allocated`.
- [ ] **The four idle timeouts are finite, asserted where the check looks for it.** The boot refusal
      of an unbounded one is `crates/nvs-config/src/server.rs:57`'s `Waits`, but the acceptance check
      names the test under `-p nvs-server`, whose holder is `crates/nvs-server/src/io.rs`'s `Phase` —
      the playbook's bullet on a check naming a crate applies before writing it. Pins
      `the_four_idle_timeouts_are_finite`.
- [ ] **A mount carries no policy of its own.** ADR 0097 § 10: `crates/nvs-config/src/tree.rs:797`'s
      `Mount` has routing fields alone, and a limit key written in a `[[server.mount]]` block is
      `E0621` rather than a second per-app block. Pins `a_mount_carries_no_policy_of_its_own`.

## Backlog

- A served request's context carries the configuration snapshot, and the selection's `origin` and
  `captures` (`crates/nvs-server/src/mount.rs:251`) — docs/plan/m7.md.
- `is_draining_answers_during_a_graceful_shutdown` — needs ADR 0078 § 6's control socket first.
- `[server] static` and `dispatch` defaults come from the mode, not from what was written —
  ADR 0091 § 3a, `crates/nvs-server/src/mount.rs:146`.
- § 6's forwarded-header walk replaces the `Host` `Table::select` reads — ADR 0097 § 6.
- A static response is one buffer per in-flight request; a byte cache goes behind `Source` —
  `crates/nvs-server/src/statics.rs`'s § *What it spends*.
- ADR 0105's uploads, stage 5 — the driver's frozen `want` for `examples/upload.nvs`.
