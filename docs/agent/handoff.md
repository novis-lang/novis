# Handoff

## State

**Stage 4 is open and its ADR slot is spent.** [ADR 0118](../adr/0118-a-capability-is-checked-at-the-door-to-the-effect.md)
decides where a capability check sits: **inside the door that performs the effect**, never beside it,
so a member that forgets the check is a member that does not perform the effect. What each member
needs is declared in one table (`nvs_stdlib::registry::CAPABILITIES:1086`), which nothing reads at
run time; a member needing no capability pays nothing at all. The ADR's body is the rule — do not
re-derive it from this paragraph.

On disk and green: `crates/nvs-config/src/capability.rs` (`Cap`, `Scope`, `Capabilities::allows`,
and `canonicalize`, which `Snapshot::build` now calls so § 4's grant side is canonical once rather
than per check), `crates/nvs-runtime/src/capability.rs:34`'s `require`, and
`crates/nvs-stdlib/tests/capability.rs`, whose four cases include both names
`docs/agent/loop-goal.toml:1796` requires. `nvs-stdlib` gained a dependency on `nvs-config` for
`Cap`.

**Two corrections to what the previous handoff said, both found by reading the tree.**

1. `native examples/limits.nvs [4 capabilities]` — the driver's failing check — is item 11's memory
   cap and is **not** closed by the capability group; it was handed forward as if it were. It is
   still an unwritten item and not a regression (the fixture's own header says it exits 0 in a tree
   not enforcing the cap), but the group that closes it is safepoint-driven limit enforcement, whose
   file set is `nvs-host`/`nvs-runtime` and shares nothing with this one.
2. **Item 10's "every syscall-touching stdlib entry point" names an empty set.** There is no
   filesystem class in `nvs-stdlib` — no `Core\File`, no `Core\Dir`, and zero `std::fs` in the whole
   crate. So `examples/capability.nvs`'s frozen `granted: read ok` / `denied: fs.write` cannot be
   written until a `Core\File` exists; that is the next group's first slice, not a fixture edit.
   `CAPABILITIES` is empty for the same reason, and § 2's
   `nvs_stdlib_reaches_the_os_only_through_the_gate` locks that state in while it is still free.

ADR 0118 § 3 deviates from item 10's wording — the declaration is a table keyed by `(class, member)`
rather than a `CoreMethod` field. § 3 is the reason: 346 rows, and a security surface worth reading
on one screen.

`orient.py`'s pack was accurate; the gap was `docs/agent/loop-goal.md`'s item bodies, which the pack
prints one line each for the group but not for the *neighbouring* items a group turns out to depend
on (10 needs 16's shape, and 11 was misattributed above). One `sed -n` on the goal file covers it.

## Next group

**Give the gate something to guard — items 10 and 13's fixture.** File set:
`crates/nvs-runtime/src/capability.rs:34` (`require`, which every slice calls),
`crates/nvs-stdlib/src/registry.rs:984` (`CLASSES`) and `:1086` (`CAPABILITIES`),
`crates/nvs-stdlib/src/lib.rs` (`symbols`/`address`), and `examples/capability.nvs`.

- [ ] **`Core\File::read` and `::write`, each behind a door** — ADR 0118 §§ 2-3. A new
      `crates/nvs-stdlib/src/file.rs` with the five edits `conventions.md` § *A `Core` member* lists,
      plus a new `open_read`/`write` pair in `crates/nvs-runtime/src/capability.rs` that calls
      `require` before touching `std::fs`, plus the two `CAPABILITIES` rows. The registry test
      `every_capability_entry_names_a_member` and `conformance_coverage.rs` both fail until the rows
      and a `.nvst` case exist.
- [ ] **`spawn script` through the same `require`** — ADR 0006 § 5's `script.spawn`, checked at
      `crates/nvs-runtime/src/script.rs:204` (`resolve`, the seam a spawn path becomes a `Program`
      through) or at its caller, which is where the `Ctx` is. `Scope::Path` on the target, so item
      6's canonicalise-then-prefix applies with no second implementation.
- [ ] **`examples/capability.nvs`'s three frozen lines** (`docs/agent/loop-goal.toml:1810`):
      `granted: read ok`, `denied: fs.write`, `denied: script.spawn`. Needs both slices above, and
      `nvs.toml` must grant `fs.read` and nothing else.

## Backlog

- Item 11's memory cap: the driver's one failing check, `examples/limits.nvs` — `docs/agent/loop-goal.md` § Stage 4.
- Item 16's `every_capability_bearing_member_declares_its_capability` — ADR 0118 § 7 has its shape.
- Item 12's isolate governance, which needs `Cap::ScriptSpawn` landed first — `docs/plan/m6.md`.
- Item 18's `Core\Secret::reveal()`, still absent from the registry — `docs/implementation-plan.md`.
- Item 22's `Core\Script` members — `crates/nvs-stdlib/src/script.rs`.
- `Live::admit`'s same-class check asks the answer, not the argument — `crates/nvs-runtime/src/graph.rs`.
