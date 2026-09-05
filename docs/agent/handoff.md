# Handoff

## State

**Stage 8's acceptance check is closed** — all five `-p nvs-types` tests exist and pass. The E08xx
band is what unblocked them, exactly as `nvs_stdlib::html`'s gap note predicted, and the two rules
landed as two passes rather than one:

- **`E0805`, ADR 0133 § 3's written reason** — `crates/nvs-types/src/reasons.rs`, a new sibling of
  `intrinsics`/`links`/`retrieval`/`program` reached from the same two places in `expr::calls`. Its
  module doc is the home of why it is *not* a `crate::intrinsics` row: that pass refuses nothing for
  being dynamic and this pass refuses only that. A `const` folds and compiles; `$why`, a
  concatenation and a computed `reason:` named argument do not. **An empty reason stays the body's
  throw** — `tests/conformance/core/to-source-refuses-an-empty-reason-and-takes-any-written-one.nvst`
  pins it there, and refusing it at compile time made that case unreachable.
- **`E0806`, ADR 0067 § 9's map asked at a `queryAs<T>`** — `derive::check_row_sites`, a deferred
  pass beside `resolve_field_types` and for its reason (the row class is routinely declared in a
  later file). Three conditions, one code: the list form, no `#[Db\Derive]`, and a mapping that
  cannot fill the constructor.

**"An opaque field" resolved to the third of those, and the reading matters** — do not redo it.
`CodecTy::Opaque` is the wrong predicate for a Db codec: `db_reachable` deliberately accepts
`decimal` and `bytes` (§ 9's `DECIMAL` and `BLOB` rows) and both erase to `Opaque`, so a rule over
the erasure would refuse exactly what § 9 promises. What *is* opaque to a row mapping is a property
`#[Db\Field(skip: true)]` took off it while the constructor still demands it —
`crates/nvs-types/src/derive.rs`'s own `#[Db\Field(skip: true)]` case had flagged that consequence
as deferred to "the `queryAs<T>` that would later run", and this is it.

**Stage 7's group was not taken**, because the acceptance failure outranked it; its three slices are
unchanged and are the group below. Nothing is blocked on a decision.

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
      the `None` and the comment saying why, and `nvs-cli` is the one crate naming both sides — the
      same reason it is `Fires`'s only implementor. The `arm` call above it takes the
      `Option<&dyn Leases>` already.
- [ ] **A fleet fire, driven through the ticker rather than through the gate.**
      `crates/nvs-server/src/schedule.rs:333` is the `arm` the three landed `-p nvs-server` tests
      drive directly, `crates/nvs-server/src/schedule.rs:301` is the `Leases` trait they stub, and
      that module's doc § *Where a `fleet` entry's lease comes from* is the decision. What is
      missing is a test that reaches a fire through the ticker with a real store behind it —
      `crates/nvs-cli/src/serve.rs:582` is where `Fires` is already implemented for one.

## Backlog

- `check_row_sites` has no `Core\Json::decodeAs` half — `crates/nvs-types/src/derive.rs` gap 3 owns
  the question and says why it is ADR 0071's to answer.
- The plan's `Open now` still reads "`fleet` needs a lease"; the field is 1996 B of its 2000 B
  ceiling, so correcting it needs a sentence dropped in the same edit.
- `E0805`'s roster is one row. `Core\Taint::assertTrusted` and `Core\Secret::reveal` are the
  obvious neighbours and each needs its own ADR's say-so — `crates/nvs-types/src/reasons.rs`.
- `crates/nvs-types/src/reasons.rs` gap 1: a spread argument is not read.
