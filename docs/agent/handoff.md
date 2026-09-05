# Handoff

## State

**Stage 7's fleet lease is closed, all three checks.** ADR 0073 § 3's lease reaches the ticker as
**`nvs_server::Leases`** — one method, `take(key, ttl) -> bool` — which is `Fires`'s sibling and for
the same reason: `crates/nvs-server/Cargo.toml` names no `nvs-stdlib`, so the store a fleet entry is
held in is reachable from `nvs-cli` and nowhere else. `crates/nvs-server/src/schedule.rs`'s module
doc § *Where a `fleet` entry's lease comes from* is that decision's home. `arm` now takes
`Option<&dyn Leases>`: a fleet entry arms when one is present and each of its fires takes the key
`name@scheduled-instant` for the interval; a `None` leaves it unarmed with the note it always had.

**`nvs serve` still passes `None`, deliberately.** `Core\Cache`'s shared tier is `put` and `get`
(ADR 0059 § 2) and neither is a set-if-absent, so there is nothing to implement `Leases` with yet and
§ 3's fallback holds — `crates/nvs-cli/src/serve.rs:472`'s comment is why, and it is the goal's own
standing decision 13. That primitive is the next group's first slice and is **not** a new `Core`
member.

Nothing is blocked on a decision. The next acceptance failure the driver reports outranks the group
below.

## Next group

**§ 3's lease, made real end to end — the store gains the operation and the binary joins the two.**
One file set: `crates/nvs-stdlib/src/cache.rs`, `crates/nvs-cli/src/serve.rs`.

- [ ] **A set-if-absent with an expiry on the shared tier, and not a `Core` member.** The goal's
      standing decision 13 says so out loud, so this is Rust-visible only: a `SET key NX PX` over the
      same connection `store_put` already holds at `crates/nvs-stdlib/src/cache.rs:722`, beside
      `crates/nvs-stdlib/src/cache.rs:567`'s thread-local. No registry row, no card, no `.nvst` case
      — the roster is unchanged, which is the whole point of keeping it off `Core\Cache`'s surface.
      Its own test is that two callers over one store get two different answers.
- [ ] **`nvs serve` implements `nvs_server::Leases` over it.** `crates/nvs-cli/src/serve.rs:472` is
      the `None` and `crates/nvs-cli/src/serve.rs:492` is the second one; the trait is
      `crates/nvs-server/src/schedule.rs:301` and its contract paragraph is the specification —
      atomic or do not implement it, and an unreachable store is a `false`. Delete the comment above
      the `arm` call that says why there is none, rather than reconciling it.
- [ ] **A fleet fire, driven through the ticker rather than through the gate.** The three landed
      cases assert `took_the_lease` (`crates/nvs-server/src/schedule.rs:699`), which is the whole
      decision but not the wiring. `crates/nvs-server/src/schedule.rs:1146`'s `drive` takes a scope
      and an `Option<Rc<Store>>`, and the case asserts that a losing host starts **no** fire and
      notes it — the `else if` at `crates/nvs-server/src/schedule.rs:545`'s neighbourhood.

## Backlog

- Renewing a lease while its run is in flight (§ 3) — `crates/nvs-server/src/schedule.rs`'s module
  doc § *Not here yet* owns the gap and says what it costs.
- The per-entry `limits` and `grants` sub-caps (§ 5), same module doc section.
- Eight spec §§ 16–17 classes with no owner on the chain — `docs/agent/carried-gaps.md`.
