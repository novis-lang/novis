# Handoff

## State

**Goal 23 — stage 3 is open, and one of its five checks is green.** Stages 1 and 2 stay green: the
compiled unit is shared behind one `Arc` with one publisher, and `nvs serve` still accepts on one
core.

**What landed:** `[server] workers` exists. `nvs_config::server::workers_for`
(`crates/nvs-config/src/server.rs:331`) answers this machine's `available_parallelism` with the key
left out and the written count as written in both directions — never raised to the machine's
parallelism and never clamped down to it — with `0` refused under `E0636`. It is part of `validate`,
so `nvs config check` refuses the zero too, and `nvs serve` re-reads it at boot beside the waits and
the valve (`crates/nvs-cli/src/serve.rs:148`) for that refusal alone, because this loop still runs
one core.

**ADR 0161 is this goal's one permitted record, and it is written for the whole of stage 3.** It
creates `rule:http-server/the-accept-fan-out-is-one-worker-per-core` — `designed` until the fan-out
is on disk, and the slice that lands it flips the status — and modifies
`rule:http-server/the-server-block-is-boot-class`, which now carries the `workers` row. The
remaining slices need no second record; they are that rule's own text.

## Next group

**Stage 3: the fan-out itself — every entry bound, one worker per core** — one file set:
`crates/nvs-cli/src/serve.rs`, with `crates/nvs-host/src/net.rs` and `crates/nvs-host/src/lib.rs`
read for their two primitives and `crates/nvs-server/src/serve.rs` for the loop's signature.

- [ ] **The listeners, and `every_entry_of_server_listen_is_bound_rather_than_the_first` plus
      `a_unix_domain_entry_is_refused_once_rather_than_once_per_core`** — bind every entry
      `listen_on` returned rather than `address`'s first: `crates/nvs-cli/src/serve.rs:292`
      (`NvsListener::bind`) is the one binding site and `crates/nvs-cli/src/serve.rs:758`
      (`address`) is what narrows the set to one today. Classification stays
      `nvs_config::server::classify`'s, so the Unix-domain refusal is taken once here before any
      worker exists. `rule:http-server/the-accept-fan-out-is-one-worker-per-core` § 1 is the
      specification.
- [ ] **The workers, and `one_worker_is_spawned_per_core_and_each_takes_its_own_listener_handle`** —
      `nvs_host::Worker::spawn(cpu, body)` (`crates/nvs-host/src/lib.rs:168`) is the pinned thread
      and `NvsListener::from_std` (`crates/nvs-host/src/net.rs:371`) is how each core takes its own
      handle on a bound `std::net::TcpListener`. Everything from the scheduler down —
      `crates/nvs-cli/src/serve.rs:534` (`serve_on_this_core`) and the `run_until_idle` loop at
      `crates/nvs-cli/src/serve.rs:573` — moves into a per-core body that must be `Send + 'static`,
      so `Rc<Table>` and the handler closure are built **inside** each worker out of the `Arc`s
      (`snapshot`, `Compiler`, `Serving`) rather than captured. `workers_for`'s count is what says
      how many. `rule:http-server/the-accept-fan-out-is-one-worker-per-core` § 2 and ADR 0161 § 3
      are the specification; the ticker (`Scheduled`, `crates/nvs-cli/src/serve.rs:606`) is armed by
      one core, not each.
- [ ] **The flags, and `listen_and_port_flags_still_override_the_file_and_still_conflict`** — a
      `--listen` replaces the whole configured set with one address rather than its first entry, and
      `--port` keeps each configured host; the two already conflict at the parser
      (`crates/nvs-cli/src/main.rs:294`, `conflicts_with = "listen"`), so this is the case that pins
      it against the fan-out. `crates/nvs-cli/src/serve.rs:758` (`address`) is the function the
      first item leaves behind or rewrites.

## Backlog

- Stage "3 drain" — the four `-p nvs-server` checks (`is_draining_answers_the_same_on_every_core`
  and its three neighbours) need a fleet to be fleet-wide over; they follow the group above.
  `docs/agent/loop-goal.toml:6073`.
- The `[context] playbook` selector missed `playbook.md:1773` (`cargo test -p nvs-cli --lib` is
  `no library targets`; the unit tests run under `--bin nvs`) even though the item's two paths are
  `nvs-cli`'s — the bullet is filed against `crates/nvs-cli/src/main.rs` and my item named neither.
- `crates/nvs-cli/src/script.rs:63`'s first known gap is single-flighting alone.
