# Handoff

## State

**Goal 6, M7. ADR 0139 landed: the session mechanics ADR 0012 § 4 deferred are decided.** The
backends are the shared cache tier and the database; `[session] backend = "local"` is `E0626` at
boot, naming ADR 0059 § 4, which is what that section's "enforced rather than documented" now
means. There is no lock (§ 4, last-write-wins over the whole record) and no sweeper (§ 5, the store
expires the entry). ADR 0012 § 4's body was folded to point here, and spec § 15's row gained
`start`.

**§ 2's store is on disk; § 1's class is not.** `crates/nvs-stdlib/src/session.rs` has the key, the
drawn identifier, `load` and `save` over `crate::cache::redis::Connection`, plus the three
directives read off the snapshot. There are **no registry rows, no cards and no `.nvst` cases** —
`Core\Session`'s seven members are the next slice of this feature, and that module's own doc says
so. `Connection::set_expiring` is new beside `set`, because a cache entry has no clock and every
session record has one.

**Both tests the stage-5 acceptance check names now run and pass**, which was the failure that
outranked the previous handoff's group:
`a_session_is_never_backed_by_the_local_cache_tier` and
`a_session_survives_a_request_landing_on_another_core`, both `-p nvs-stdlib`. The second is a
two-thread test against one fake RESP store; the local tier's copy staying invisible to the second
thread is the assertion, not the round trip.

**What ADR 0086 § 6 still owes is corpus, not code** — unchanged, and displaced by the above.
`ArgConv::OneOf` converts and refuses in `crates/nvs-stdlib/src/command.rs`; no `.nvst` case asks it
anything, where the enum half landed with two.

## Next group

**§ 6's union in the corpus, over one file set:** `tests/conformance/core/command-run-*.nvst` and
`crates/nvs-stdlib/src/command.rs`.

- [ ] **A `OneOf` argument converts by its word** (ADR 0086 § 6) — the arm is
      `crates/nvs-stdlib/src/command.rs:543` and the variant it reads is
      `crates/nvs-types/src/commands.rs:184`; the shape to copy is
      `tests/conformance/core/command-run-converts-an-enum-argument-by-its-case-name.nvst`, and
      `crates/nvs-stdlib/src/command.rs:1186` is a table already declaring a two-member union.
- [ ] **A word outside the set is a usage error naming every accepted one** (ADR 0086 § 6) — the
      message is built at `crates/nvs-stdlib/src/command.rs:547`, and
      `crates/nvs-stdlib/src/command.rs:1195` pins its exact wording as a `#[test]` already.
- [ ] **A subset of an enum's cases still refuses to convert** (ADR 0086 § 6) — the same arm,
      `crates/nvs-stdlib/src/command.rs:543`, reached with a `OneOf` naming fewer words than the
      enum has cases; gap 1 in that module's known gaps is the case-subset half.

## Backlog

- `Core\Session`'s seven members over the landed store — ADR 0139 § 1's roster, registry rows,
  cards and `.nvst` cases; `crates/nvs-stdlib/src/session.rs`'s module doc owns which half is on
  disk.
- The `db` backend ADR 0139 § 3 admits and nothing implements — same module doc.
- `tests/conformance/http/` does not exist and the stage-9 `nvs-suite` check names eight cases under
  it; the corpus is flat with a subsystem prefix (`docs/agent/loop-goal.toml:3810`).
- ADR 0059's own *Verification* still files four fixtures under **M8** that goal 6's stage 5 asks
  for — the local-tier eviction and secret-refusal cases.
