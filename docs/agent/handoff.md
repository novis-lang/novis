# Handoff

## State

**Goal 24 — stage 2 is closed. All five `Core\Net` entry points are on disk**, and the three
stage-2 `cargo-named` checks (nvs-config, nvs-host, nvs-stdlib's five names) are green.
`Core\Net::connectLocal` and `Core\Net::listenLocal` ask `net.local` at `Scope::Path`, one grant
governing both ends (`rule:config/net-local-is-named-and-not-on-the-roster`, now `shipped`).

`Connected` and `Bound` are enums over two transports rather than newtypes, so one
`Core\Net\Stream` and one `Core\Net\Listener` serve both — a match and not a `dyn`, so the parking
path takes no vtable. `nvs_host::NvsAcceptor<L: Accepting>` is the generic accepting half behind
them: `NvsListener` is its TCP alias (no caller changed) and `NvsUnixListener` its Unix one.

Two decisions this landed, both recorded in `crates/nvs-stdlib/src/net.rs`'s module doc § *one
handle class per shape, two transports inside it*: `Core\Net\Listener::port` **throws** for a
listener bound at a path rather than answering `?uint` every TCP caller would unwrap, and a build
with no `AF_UNIX` transport refuses both local doors *after* the grant is asked, so an ungranted
program learns nothing about the host.

Windows cannot compile the `cfg(unix)` half at all and `verify.py` has no Unix leg — the playbook's
new bullet under *Running things* is how this session checked it. Nothing is blocked.

## Next group

**Stage 3: `Core\Os`, five facts about the host** — one file set: `crates/nvs-stdlib/src/os.rs`
(new), `crates/nvs-stdlib/src/lib.rs`, `crates/nvs-stdlib/src/registry.rs`, `examples/os-facts.nvs`.

- [ ] **`Core\Os` does not exist in the tree — the class, its five members, and its registration.**
      `rule:core-api/tier-roster` places it (ADR 0051 § 3, "`Core\Os` (`posix`, minus fork)") and
      `docs/decisions/0148.md` is the settled design the goal's § *Standing decisions* says this
      stage builds rather than decides; `python tools/brief.py --where os` finds anything else that
      owns a member. Five edits per member, `docs/agent/conventions.md` § *A `Core` member*: the
      module goes in the list at `crates/nvs-stdlib/src/lib.rs:188` (alphabetical), the class in the
      roster beside `crate::net::CLASS` at `crates/nvs-stdlib/src/registry.rs:1705`, and its
      capability rows — probably all `None`, since reading this host's own facts reaches nothing —
      at `crates/nvs-stdlib/src/registry.rs:2036`. `crates/nvs-stdlib/src/net.rs:842` is the
      neighbour to copy a handle-free class's shape from.
- [ ] **The four `-p nvs-stdlib` names the stage-3 check wants**, listed at
      `docs/agent/loop-goal.toml:6174`: `pid_hostname_cpu_count_memory_usage_and_load_average_all_answer`,
      `cpu_count_answers_the_number_serve_fans_out_over`,
      `load_average_throws_on_windows_with_a_message_naming_the_platform` and
      `no_core_os_member_is_a_sink_or_carries_tainted`. The third names the platform split, so the
      member is `cfg`-split like the local doors at `crates/nvs-stdlib/src/net.rs:1420` — and the
      playbook's new *Running things* bullet is how to compile the other half.
- [ ] **`examples/os-facts.nvs`**, the acceptance fixture listed at `docs/agent/loop-goal.toml:10`
      and the check the driver has been failing on. Its output is frozen once written, so nothing
      in it may print a number this host decides — a load average or a pid in the expected output
      is a fixture that passes only here. `examples/net-echo.nvs` is stage 2's and shows the shape.

## Backlog

- Stage 4 `Core\Signal` — graceful shutdown and nothing else, per `docs/decisions/0148.md`.
- Stage 5 — `every_part_two_spec_class_is_registered` and `every_migration_member_is_registered`,
  then the two `nvs-suite` trees (`docs/agent/loop-goal.toml:6200`).
- `docs/spec/01-core-library.md` § 16 has a roster row for `Core\Net` and no member table, so
  nothing in this class is covered by `spec_registry_coverage.rs`. Not a gate today.
- `examples/net-echo.nvs` demonstrates the TCP half only; a local-socket line would run on Unix
  alone, so the fixture stays TCP on purpose.
- **`cargo clippy -- -D warnings` is red on the Unix leg and green here**, in `nvs-db`: two
  `large_enum_variant` errors on the connection enums that carry `Local(NvsUnix)`
  (`crates/nvs-db/src/pg.rs:381`, `crates/nvs-db/src/mysql.rs:330`), whose variant only exists
  under `cfg(unix)`. Not this session's — nothing here touches `nvs-db` — and `verify.py` cannot
  see it. Boxing the `Tls` variant is what the lint asks for.
