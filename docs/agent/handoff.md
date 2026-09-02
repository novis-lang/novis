# Handoff

## State

**ADR 0067 § 13's `max` is enforced, and a `Lease` is the thing that counts.**
`crates/nvs-runtime/src/pool.rs:282` is `admit`: it grants one slot per live connection under a
key, refusing at `max`, and the lease it answers with is carried beside the connection for the
request's whole life. `crates/nvs-runtime/src/pool.rs:325`'s `release` consumes it, and
`Lease`'s own `Drop` gives the slot back on every other path — including a handshake that failed
after admission — so no caller has a decrement to remember.

**`take` draws against a lease rather than a bare key**, and that is what makes § 13's
`cores × max` literally true rather than approximately: an idle entry is only ever a connection
that was live under a lease, so `live + idle` under a key can no more exceed `max` than `live`
can. `pool.rs`'s module doc § *What `max` counts* is that argument's only home, and the handbook
row (`docs/novis.md:18575`) now states the deployment number in those terms.

**At the ceiling, `Core\Db::connect` throws immediately** (`crates/nvs-stdlib/src/db.rs:2376`) —
`ThrownClass::Io`, beside the handshake that "did not open", because § 8's `Db\DbError` is for a
refusal the *server* made. That is exactly `acquire = 0` semantics; what is missing is the
waiting, not the refusal.

**`acquire` is the last of the four bounds, and it needs the core's scheduler.**
`nvs_host::timer::park_until` (`crates/nvs-host/src/timer.rs:251`) is the primitive that exists;
what does not is a way for a dropped lease to wake a request parked on that key, so the choice
between a waiter list woken from `Lease::drop` and a bounded poll is open and is the next
session's first decision. It also has to say which deadline wins when `connect`'s own `timeout`
option (`crates/nvs-stdlib/src/db.rs:2218`) is shorter than `acquire`.

**A matrix test has no trust anchor yet.** `crates/nvs-db/src/matrix.rs:91`'s `endpoint` has
**no caller at all** — the five test files `crates/nvs-db/src/lib.rs:93` names do not exist — and
`matrix.rs` carries no CA field, while the compose PostgreSQL is reached over TLS and the CA is
still not in git. Docker is up and healthy on this machine, so the wall is the trust anchor and
nothing else.

**Unchanged and still true.** The driver's acceptance line names `examples/queue.nvs` — Stage 8's
unlanded `Core\Queue` (ADR 0084), not a regression; its `[[check]]` is
`docs/agent/loop-goal.toml:2927`. § 7's backoff is still blocked on `nvs-runtime`'s known gap 3
(`crates/nvs-stdlib/src/db.rs:149` argues it). Stage 5's `args = ["test", "-p", "nvs-db"]`
(`docs/agent/loop-goal.toml:2830`) still cannot see the two `nvs-stdlib` tests, and is still the
user's call.

**`orient.py`'s pack was right for the slice.** The one thing it could not have printed is the
scheduler's parking surface, which the next item needs — `[context] modules` naming
`nvs-host/src/timer.rs` and `nvs-host/src/scheduler.rs` would close that.

## Next group

**§ 13's last bound and the proof that reuse is real. The file set is
`crates/nvs-runtime/src/pool.rs`, `crates/nvs-stdlib/src/db.rs` and `crates/nvs-host/src/timer.rs`.**

- [ ] **`acquire` is how long a request waits at the ceiling before it throws** — decide the wait
      first: a waiter list per key woken from `Lease::drop`, or a bounded poll over
      `nvs_host::timer::park_until` (`crates/nvs-host/src/timer.rs:251`). `admit` is
      `crates/nvs-runtime/src/pool.rs:282` and its refusal is
      `crates/nvs-stdlib/src/db.rs:2376`; `Lease::drop` is `crates/nvs-runtime/src/pool.rs:224`.
      Zero is legal and already means what it says. ADR 0067 § 13.
- [ ] **Which deadline wins: `acquire` or `connect`'s `timeout`** — one paragraph in the same
      slice, decided where `deadline_of` is read (`crates/nvs-stdlib/src/db.rs:2218`) and stated
      in `pool.rs`'s module doc. ADR 0067 §§ 2, 13.
- [ ] **A matrix test that reuse is real and clean** — two requests on one core, the second
      getting the first's connection, reset. First answer how a `-p nvs-db` case trusts the
      compose server: `crates/nvs-db/src/matrix.rs:91` has no CA field and no caller yet.
      ADR 0067 § 13 and its *Verification*.

## Backlog

- § 7's exponential backoff, blocked on `nvs-runtime` known gap 3 — `crates/nvs-stdlib/src/db.rs:149`.
- Stage 5's `-p nvs-db` args cannot see the two `nvs-stdlib` tests — `docs/agent/loop-goal.toml:2830`.
- `Core\Db::open`'s shape-parameter settings — ADR 0067 § 2, `docs/plan/m8.md`.
- The other four drivers — ADR 0132 § 1, `crates/nvs-db/src/conn.rs`.
- `Core\Db\Connection::close`, which is what would take a connection out mid-request — spec § 18.
