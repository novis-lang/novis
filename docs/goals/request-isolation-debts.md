---
looper: 1
title: "The request-isolation rules that are designed are built"
summary: "Every rule this goal owns is `shipped` with a guard test, and `bun nv owners --closes request-isolation-debts` passes."
context: []
stages:
  - id: 1
    title: host matching
    status: done
    context: [crates/nvs-server/src/mount.rs, crates/nvs-config/src/mount.rs, crates/nvs-cli/src/serve.rs, crates/nvs-stdlib/src/request.rs]
    extra:
      context:
        rules: [http-server/host-matching-is-on-the-host-part-only, errors/ambiguous-input-refused, http-server/a-mount-table-expands-at-boot]
        shapes: ["A `.nvst` test case"]
  - id: 2
    title: a queue payload keeps its qualifiers
    status: open
    context: [crates/nvs-stdlib/src/queue.rs, crates/nvs-stdlib/src/script.rs]
    extra:
      context:
        rules: [concurrency/a-payload-refuses-secret-and-keeps-its-qualifiers, security/secret-qualifier, security/tainted-qualifier]
        shapes: ["A `.nvst` test case", A diagnostic]
  - id: 3
    title: "the artifact cache is read, not mapped"
    status: open
    context: [crates/nvs-cli/src/cache.rs]
    extra:
      context:
        rules: [packaging/the-artifact-cache-is-read-not-mapped, errors/panics-bypass-user-code]
        shapes: []
  - id: 4
    title: "a guest runs under the request's own budget"
    status: open
    context: [crates/nvs-cli/src/extensions.rs, crates/nvs-ext/src/call.rs]
    extra:
      context:
        rules: [packaging/a-guest-runs-under-the-requests-budget, packaging/a-guest-call-yields-on-its-core, errors/on-limit]
        shapes: []
  - id: 5
    title: a deadline sends the driver-side cancel
    status: open
    context: [crates/nvs-db/src/pg.rs, crates/nvs-db/src/conn.rs, crates/nvs-stdlib/src/db/execute.rs, crates/nvs-stdlib/src/db/pool.rs]
    extra:
      context:
        rules: [concurrency/a-deadline-bounds-the-cancel-not-the-return, concurrency/nothing-is-still-running-when-a-call-returns]
        shapes: []
  - id: 6
    title: admission subtracts a wedged core
    status: open
    context: [crates/nvs-host/src/watchdog.rs, crates/nvs-server/src/admit.rs]
    extra:
      context:
        rules: [http-server/a-wedged-core-is-shed-never-killed, http-server/admission-is-arithmetic-not-a-number]
        shapes: []
  - id: 7
    title: "the `ImagePath` encoder has a fuzz target"
    status: open
    context: [crates/nvs-cli/src/service.rs, fuzz/Cargo.toml]
    extra:
      context:
        rules: [packaging/the-argv-lives-in-imagepath]
        shapes: []
  - id: 8
    title: "a test holds the image manifest's absence"
    status: open
    context: [extensions/image/manifest.json, crates/nvs-ext/tests/manifest.rs]
    extra:
      context:
        rules: [security/image-component-declares-nothing, security/extension-declares-sink-or-source]
        shapes: []
checks:
  - id: host-matching-server
    stage: 1
    name: "nvs-server (the host part is matched with its port, its case and one trailing dot folded, a non-ASCII host is refused, and an unknown host on a host-mounted table is a `404`)"
    run: [cargo, test, --lib, "mount::"]
    tests: [a_host_with_a_port_a_capital_and_a_trailing_dot_matches_its_mount, a_non_ascii_host_reaches_no_mount, an_unknown_host_on_a_host_mounted_table_reaches_no_mount]
  - id: host-matching-config
    stage: 1
    name: "nvs-config (a declared host is one key in any case to the duplicate check and to an override)"
    run: [cargo, test, --test, mount]
    tests: [two_mounts_whose_hosts_differ_only_in_case_are_a_duplicate, a_declared_host_is_held_in_ascii_lower_case]
  - id: host-matching-door
    stage: 1
    name: "an HTTP/1.1 request with no `Host` and one with a non-ASCII `Host` are a `400` at the door"
    run: [cargo, test, --test, request]
    tests: [an_http_1_1_request_with_no_host_is_a_400, a_request_with_a_non_ascii_host_is_a_400]
  - id: payload-qualifiers
    stage: 2
    name: "a `tainted` payload value comes back `tainted` from `Core\\Script::args`, and a `secret` one does not compile"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/queue-a-tainted-payload-value-comes-back-tainted.nvst, --case, tests/conformance/core/queue-a-secret-payload-value-does-not-compile.nvst]
    tests: [tests/conformance/core/queue-a-tainted-payload-value-comes-back-tainted.nvst, tests/conformance/core/queue-a-secret-payload-value-does-not-compile.nvst]
  - id: cache-is-read
    stage: 3
    name: "nvs-cli (a cached unit is read into memory, and a file truncated after the load leaves the loaded unit whole)"
    run: [cargo, test, --bin, nvs, "cache::"]
    tests: [a_cached_unit_is_read_into_memory_and_never_mapped, truncating_a_cached_file_after_its_load_leaves_the_loaded_unit_whole]
  - id: guest-budget
    stage: 4
    name: "nvs-cli (a guest's CPU time and memory are the request's own, and a guest past either limit ends the request with a resource-limit `FATAL`)"
    run: [cargo, test, --bin, nvs, "extensions::"]
    tests: [a_guest_spends_the_requests_own_cpu_deadline, a_guest_growth_is_charged_to_the_requests_memory_count, a_guest_past_the_requests_limit_ends_it_with_a_resource_limit_fatal]
  - id: guest-budget-on-limit
    stage: 4
    name: "a looping guest under a request's CPU limit reaches `onLimit` and the server keeps serving"
    run: [cargo, test, --test, serve_isolation]
    tests: [a_looping_guest_reaches_on_limit_and_the_next_request_is_served]
  - id: deadline-cancel
    stage: 5
    name: "nvs-stdlib (a deadline cancels a running statement through its driver, waits for the connection, and closes one that misses its own timeout)"
    run: [cargo, test, --test, db_stream]
    tests: [a_deadline_cancels_a_running_statement_through_the_driver, a_cancelled_connection_returns_to_the_pool_in_a_known_state, a_connection_that_misses_its_timeout_after_a_cancel_is_closed_and_not_returned]
  - id: wedged-core-shed
    stage: 6
    name: "nvs-server (a wedged core stops taking work, `max_in_flight` drops by its share, and the share returns when it turns again)"
    run: [cargo, test, --lib, "admit::"]
    tests: [a_wedged_core_takes_no_new_work, max_in_flight_drops_by_a_wedged_cores_share, a_core_that_turns_again_gets_its_share_back]
  - id: argv-round-trip
    stage: 7
    name: "the `ImagePath` encoder round-trips trailing backslashes, embedded quotes and a directory before a closing quote"
    run: [cargo, test, --lib, "argv::"]
    tests: [an_image_path_round_trips_through_command_line_to_argv, the_binary_path_is_quoted_with_no_space_in_it]
  - id: argv-fuzz-target
    stage: 7
    name: "the fuzz workspace declares an `image_path` target"
    file: fuzz/Cargo.toml
    output_contains: ["name = \"image_path\""]
  - id: image-manifest-declares-nothing
    stage: 8
    name: "nvs-ext (the image manifest declares no qualifier deviation and no capability)"
    run: [cargo, test, --test, manifest]
    tests: [the_image_manifest_declares_no_qualifier_and_no_capability]
  - id: owners-closes
    stage: 8
    name: "no designed rule or gap names this goal"
    run: [bun, nv, owners, --closes, request-isolation-debts]
    reach: true
---

# The request-isolation rules that are designed are built

## Why

Every rule in the rulebook is `shipped` or `designed`, and every `designed` one names the goal or the
milestone that builds it. These eight rules guard the first priority, security and request isolation,
and are not built: a request can reach the wrong mount, a queue launders `tainted`, a truncated cache
file can kill a worker with `SIGBUS`, a guest spends a budget beside the request's rather than the
request's own, a deadline abandons a running statement, a wedged core still counts as capacity, and two
security claims have no test. This goal builds them, in that order, and flips each rule to `shipped`.

It is a chain goal after `pdf` and `spreadsheet`, and before `server-rule-debts` and
`language-rule-debts`, because isolation outranks the rest.

## Standing decisions

**A stage ships its rule.** The stage that builds a rule sets its record's `status` to `shipped`,
removes its `goal` owner, fills `guardedBy` with the tests the stage's checks name
(`data/rules/<topic>/<slug>.json`), and rewrites the fragment whole so it says what is built, with no
"Not on disk" paragraph left. Home: the rule's own fragment.

**Host matching** follows `rule:http-server/host-matching-is-on-the-host-part-only` exactly: the port
is stripped, the host is ASCII-lowercased and one trailing dot is removed, and a declared mount host is
held in ASCII lower case. A `Host` that is not ASCII is a `400` at the door; today it becomes `None`
and falls to the host-less mounts (`crates/nvs-server/src/mount.rs:215-220`), which is the bug. An
absent `Host` on HTTP/1.1 is a `400` at the door as well, so the comment at
`crates/nvs-stdlib/src/request.rs:2691` becomes true; nothing in `nvs-server` does it yet. HTTP/2's
`:authority` is the host there. The `proxy_set_header Host $host;` note goes on the reverse-proxy page
(`website/src/content/docs/guides/production/reverse-proxy.mdx`), in the voice AGENTS.md § *Text an
end user reads* sets.

**Queue qualifiers** are static: every text read out of `Core\Script::args()` is `tainted`, whether it
was enqueued `tainted` or clean, and nothing is recorded in the row. A `secret` in a payload is a
compile error. Home: `rule:concurrency/a-payload-refuses-secret-and-keeps-its-qualifiers`.

**The artifact cache** reads a unit with one read into an owned buffer
(`crates/nvs-cli/src/cache.rs:1320`), and the `unsafe` mapping block goes. The executable pages stay an
anonymous mapping, which no file can truncate. Memory: one copy of each loaded unit, per process, as
the rule says.

**A guest's budget is the request's own.** The `Spent` meter (`crates/nvs-cli/src/extensions.rs:38-40`)
stops being a second allowance: its CPU deadline is the request's deadline, including the time spent
before the first call, and each growth is charged to the memory count the runtime keeps for the
request. A guest past either limit ends the request with the resource-limit `FATAL` that reaches
`onLimit`. Memory: none new; the guest's linear memory moves from its own count to the request's.

**A deadline cancels through the driver.** Every driver `nvs-db` has sends its own cancel when a
deadline fires on a running statement: PostgreSQL's `CancelRequest` over a second connection with the
key it already keeps (`crates/nvs-db/src/pg.rs:345,724`), `KILL QUERY` over a second connection on
MySQL and MariaDB, `sqlite3_interrupt`, and the TDS attention signal. The call then waits for the
connection to reach a known state, bounded by that connection's own `timeout`; past it the
connection is closed and never returned to the pool. Fallback for a driver whose cancel cannot be
built: close the connection at the deadline, never return it.

**A wedged core is shed.** `nvs_server::admit` subscribes to the watchdog's report
(`crates/nvs-host/src/watchdog.rs:25-36`) and grows no detector of its own. A reported core takes no
new work and `max_in_flight` drops by its share; when the core's deadline clears, the share returns.
Nothing is killed.

**The `ImagePath` encoder** and its `CommandLineToArgvW` decoder move from `nvs-cli`
(`crates/nvs-cli/src/service.rs:617-663`) into a library crate the fuzz workspace can depend on, since
`nvs-cli` is a binary. The session picks the crate; the safe default is a small `argv` module in the
lowest crate that has no platform dependency. `nvs-cli` keeps calling it.

**The image manifest's absence** is held by a test that reads `extensions/image/manifest.json` through
the manifest parser, never by a string search.

## Stages

### 1. host matching

**Does:** the server matches a mount on the host part alone, and refuses an absent or non-ASCII `Host`.

- [x] `select` folds case and one trailing dot after the port is stripped, and a non-ASCII `Host` is refused, not `None` (`rule:http-server/host-matching-is-on-the-host-part-only`). `crates/nvs-server/src/mount.rs:215`
- [x] A declared host is held in ASCII lower case for the duplicate check and an override. `crates/nvs-config/src/mount.rs:58`
- [x] The door answers `400` to an HTTP/1.1 request with no `Host` and to a non-ASCII one. `crates/nvs-cli/src/serve.rs:935`
- [x] The comment on `host_named` checked against the door, and the fragment rewritten. `crates/nvs-stdlib/src/request.rs:2691`
- [x] The proxy note on the reverse-proxy page. `website/src/content/docs/guides/production/reverse-proxy.mdx:1`
- [x] `Core\Request::host`'s feature proofs updated where their text names the old matching (`bun nv proofs --id 'Core\Request::host'`). `docs/examples/core/Request/host/about.md:1`

### 2. a queue payload keeps its qualifiers

**Does:** a value enqueued `tainted` comes back `tainted`, and a `secret` cannot be enqueued.

- [x] Every text read out of `Core\Script::args()` is `tainted`, so a value enqueued `tainted` comes back `tainted`, and the rule is shipped. `crates/nvs-stdlib/src/script.rs:158`
- [x] A `secret` argument to `push` does not compile, with a diagnostic naming configuration as the place for a credential. `crates/nvs-stdlib/src/queue.rs:2019`
- [ ] The conformance cases, and the feature proofs of `Core\Queue::push` and `Core\Script::args`: an attack that pushes a `tainted` value and echoes it in the job. `docs/examples/core/Queue/push/about.md:1`

### 3. the artifact cache is read, not mapped

**Does:** a cached unit is read into memory, so no file change after the load can raise a signal.

- [ ] `load` reads the file into an owned buffer and verifies it, and `Verified` holds the buffer. `crates/nvs-cli/src/cache.rs:1313`
- [ ] The `unsafe` block and the `memmap2` file mapping go; the anonymous executable mapping stays. `crates/nvs-cli/src/cache.rs:191`

### 4. a guest runs under the request's own budget

**Does:** a guest spends the request's CPU deadline and memory count, and a limit reaches `onLimit`.

- [ ] `Spent`'s meter reads the request's own deadline and charges the runtime's memory count. `crates/nvs-cli/src/extensions.rs:362`
- [ ] A guest past a limit raises the resource-limit `FATAL`, which reaches `onLimit`. `crates/nvs-ext/src/call.rs:106`
- [ ] The module doc and the fragment say what is built. `crates/nvs-cli/src/extensions.rs:38`

### 5. a deadline sends the driver-side cancel

**Does:** a deadline that fires on a running statement cancels it through the driver and never pools a dirty connection.

- [ ] PostgreSQL sends `CancelRequest` with the kept key. `crates/nvs-db/src/pg.rs:724`
- [ ] MySQL and MariaDB send `KILL QUERY`, SQLite calls `sqlite3_interrupt`, and TDS sends attention. `crates/nvs-db/src/mysql.rs:1693`
- [ ] The call waits for the connection under its own `timeout`, and closes one that misses it. `crates/nvs-stdlib/src/db/execute.rs:70`

### 6. admission subtracts a wedged core

**Does:** a core the watchdog reports takes no new work, and the ceiling stops counting it.

- [ ] `Admission` subscribes to the watchdog's report and drops the core's share of `max_in_flight`. `crates/nvs-server/src/admit.rs:201`
- [ ] The share returns when the core turns again, and the watchdog's module doc says what admission does. `crates/nvs-host/src/watchdog.rs:25`

### 7. the `ImagePath` encoder has a fuzz target

**Does:** the encoder and its decoder live in a library crate, and a fuzz target holds their round trip.

- [ ] The encoder and decoder move to a library module, and `nvs-cli` calls it. `crates/nvs-cli/src/service.rs:617`
- [ ] The `image_path` fuzz target asserts the round trip on any input. `fuzz/fuzz_targets/image_path.rs:1`
- [ ] The property tests beside the module. `fuzz/Cargo.toml:1`

### 8. a test holds the image manifest's absence

**Does:** a test fails if the image manifest ever declares a qualifier deviation or a capability.

- [ ] The test, through the manifest parser. `crates/nvs-ext/tests/manifest.rs:1`
- [ ] The fragment's "no test holds the absence" sentence rewritten. `docs/rules/security/image-component-declares-nothing.md:1`

## Handoff

### State

Stage 1 is done. In stage 2 the user chose the static result: every text read out of
`Core\Script::args()` is `tainted`, a clean value included, and nothing is recorded in the row.
`rule:concurrency/a-payload-refuses-secret-and-keeps-its-qualifiers` is shipped on the two
`payload-qualifiers` conformance cases, and the per-value recording is its Declined entry. Stage 2's
one open item is the feature proofs. The macOS warm-start budget is still *not checked*.

### Next

**Stage 2's feature proofs**, file set: `docs/examples/core/Queue/push/`, `docs/examples/core/Script/args/`, `tests/hostile/`.

- [ ] The feature proofs of `Core\Queue::push` and `Core\Script::args` (`bun nv proofs --id 'Core\Queue::push'`): an attack that pushes a `tainted` value and uses it in the job, and its about.md says a payload text comes back `tainted` (`rule:concurrency/a-payload-refuses-secret-and-keeps-its-qualifiers`). `docs/examples/core/Queue/push/about.md:1`

Stage 3 does not depend on this and can run first: `crates/nvs-cli/src/cache.rs:1313`.

### Backlog

- Stages 3 to 8 — this file's § *Stages*.
- The macOS warm-start budget in the nightly cost guards, once `origin/main` is pushed — `tools/nv/cmd/bench.ts`.
