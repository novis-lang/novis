# Handoff

## State

**ADR 0067 § 13's four bounds are all on disk, and `acquire` is a queue rather than a scramble.**
`crates/nvs-runtime/src/pool.rs:425`'s `queue` joins a task to a per-key line and `Waiting::slot` is
what it re-asks after every wake. A `Lease` that ends hands its slot to the head of that line instead
of freeing it, so the count never dips across the hand-over and `admit` still finds the key full — a
newcomer cannot barge past a waiter, and that needs no separate rule. `pool.rs`'s module doc
§ *`acquire` is a queue* is that argument's only home.

**The parking is the caller's, and it had to be.** `nvs-runtime` cannot name `nvs-host`, so the pool
owns the line and `Core\Db::connect` owns the loop, reaching its core through `nvs_runtime::host` —
the same inversion `Core\Channel` already waits on. `Host::park` now takes an `Option<Instant>`
(`crates/nvs-runtime/src/host.rs:366`) rather than being unbounded, backed by
`nvs_host::timer::wait_until` (`crates/nvs-host/src/timer.rs:305`): `park_until` without the
re-arming loop, so a wake ends it and so does the instant. `Core\Channel`'s wait passes `None`.

**Which deadline wins is settled and recorded.** Whichever is earlier, and the refusal names which
one it was. `crates/nvs-stdlib/src/db.rs:3159`'s `wait_for_slot` doc comment is that decision's home:
`acquire` is the operator's ceiling and `timeout` is the program's, neither may spend the other, and
because the handshake below is measured against the same `timeout` instant a wait that ate most of it
leaves the rest for opening. `acquire = 0` and a call with no task beneath it both refuse without
parking, which is § 13's own reading of zero.

**A matrix test still has no trust anchor**, unchanged from last session. `crates/nvs-db/src/matrix.rs:91`'s
`endpoint` has no caller at all — the five test files `crates/nvs-db/src/lib.rs:93` names do not exist
— and `matrix.rs` carries no CA field, while the compose PostgreSQL is reached over TLS. Docker is up
and healthy here, so the wall is the trust anchor and nothing else.

**Unchanged and still true.** The driver's acceptance line names `examples/queue.nvs` — Stage 8's
unlanded `Core\Queue` (ADR 0084), not a regression; its `[[check]]` is `docs/agent/loop-goal.toml:2927`.
§ 7's backoff is still blocked on `nvs-runtime`'s known gap 3 (`crates/nvs-stdlib/src/db.rs:149`
argues it). Stage 5's `args = ["test", "-p", "nvs-db"]` (`docs/agent/loop-goal.toml:2830`) still
cannot see the two `nvs-stdlib` tests, and is still the user's call.

**`orient.py`'s pack was short the scheduler, exactly as the last handoff predicted.** `[context]
modules` names neither `nvs-host/src/timer.rs`, `nvs-host/src/group.rs` nor `nvs-runtime/src/host.rs`,
so all three were read from scratch to answer "what can a `Core` member wait on". Adding them pays
for itself the next time a member has to wait.

## Next group

**The proof that § 13's reuse and its queue are real, against a live server. The file set is
`crates/nvs-db/src/matrix.rs`, `crates/nvs-db/src/lib.rs` and a new `crates/nvs-db/tests/` file.**

- [ ] **A matrix test has a trust anchor** — `crates/nvs-db/src/matrix.rs:91`'s `endpoint` needs a CA
      field beside the `NVS_DB_MATRIX_*` ones it already reads, and `tools/db-matrix.py` needs to set
      it; `crates/nvs-db/src/lib.rs:93` names the five test files that do not exist yet, so decide
      whether that list is the plan or is stale. Nothing can connect from `-p nvs-db` until this
      lands. ADR 0067 § *Verification*.
- [ ] **Two requests on one core share one connection** — the first releases at teardown, the second
      draws it warm through `crates/nvs-runtime/src/pool.rs:495`'s `take` and its reset ran. Assert
      the *identity* of the connection, not that a second query worked. ADR 0067 § 13.
- [ ] **A third request at the ceiling waits and then throws naming `acquire`** — `max = 1` with a
      short `acquire`, two tasks, and the refusal from `crates/nvs-stdlib/src/db.rs:3159`. The queue's
      own unit cases are in `crates/nvs-runtime/src/pool.rs:905`; what this adds is a real park.
      ADR 0067 § 13.

## Backlog

- Stage 8's `Core\Queue` (ADR 0084) is the standing acceptance failure — `docs/agent/loop-goal.toml:2927`.
- § 7's backoff waits on `nvs-runtime`'s known gap 3 — `crates/nvs-stdlib/src/db.rs:149`.
- `Core\Db::open`'s shape-parameter type — `docs/implementation-plan.md`, Open now.
- Stage 5's `-p nvs-db` check cannot see two `nvs-stdlib` tests — `docs/agent/loop-goal.toml:2830`.
- `[context] modules` is missing the three scheduler files above — `docs/agent/loop-goal.toml`.
