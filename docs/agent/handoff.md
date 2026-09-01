# Handoff

## State

**Stage 10's first gate is on disk and green.**
`crates/nvs-stdlib/tests/spec_registry_coverage.rs:577`'s `every_part_two_spec_member_is_registered`
walks spec §§ 14-19 against `registry::CLASSES` — § 14's and § 15's bullets, § 18's and § 19's Member
tables — over the ratchet
`crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt`, which holds 59 keys: § 14's
directory/metadata half of `Core\IO`, § 15's three request classes plus `Core\Env::mode`, its three
constants and `Core\Cap::has`, and § 18's `Core\Db` whole. The test's own doc owns the reader (a bullet's
roster is what precedes its em dash) and the exclusions.

**§§ 16 and 17 are unguarded by any registry walk**, on the same line § 13 is: both are
`| Class | Surface | ADR |` tables with the members inside an English cell. Ten classes are therefore
gated only by `conformance_coverage.rs`, which walks the registry and so cannot see a spec row that was
never implemented.

**The acceptance check has advanced, not closed.** Stage 10's `cargo-named` check lists six tests and
two are still absent from every crate — `every_part_two_member_has_a_conformance_case` and
`no_class_outside_tier_zero_registers_a_core_name`. They are the next group, and the check will keep
naming the first missing one.

**The previous handoff's `Core\Uuid` group was already on disk in full**, so nothing was written for it;
the playbook's new *Tooling* bullet owns why `gaps.py` keeps nominating finished classes. Nothing was
missing from this session's pack.

## Next group

**Both remaining stage 10 tests, one file set: `crates/nvs-stdlib/tests/`.** They are what
`docs/agent/loop-goal.toml`'s stage 10 check still names, and `docs/plan/m8.md`'s *Verify* paragraph
names the second one as CI infrastructure rather than as a fixture.

- [ ] **`every_part_two_member_has_a_conformance_case`** — this session's spec reader, asked the other
      way round: every §§ 14-19 member that *is* registered has a `.nvst` case calling it. It is
      `every_core_class_has_a_conformance_floor_of_three`'s question narrowed to Part II, so the case
      index it already builds is the half not to rewrite; the spec walk is the half to lift.
      `crates/nvs-stdlib/tests/conformance_coverage.rs:286`,
      `crates/nvs-stdlib/tests/spec_registry_coverage.rs:577`.
- [ ] **`no_class_outside_tier_zero_registers_a_core_name`** — ADR 0051 § 3's tier boundary as a gate:
      nothing but `registry::CLASSES` may declare a name beginning `Core\`. Decide first what the second
      roster is in this tree — `nvs_hir::errors::TREE` holds `Core\Test\Failure` and is trusted to, per
      the playbook's two-rosters bullet — so the test is a statement about which rosters exist, not a
      scan for a class nobody has written. `crates/nvs-stdlib/src/registry.rs:1079`,
      `crates/nvs-stdlib/tests/capability.rs:278`.

## Backlog

- Spec § 14's `Core\IO` roster is 13 members short — `append`, `copy`, `move`, `list`, `walk`, `stat`,
  `makeDir`, `isDir`/`isFile`/`isReadable`/`isWritable`, `modifiedAt`, `canonicalize`. The ratchet file
  is the list; `docs/spec/01-core-library.md` § 14 is the spec.
- Widening the Part II walk to §§ 16-17 needs a *Replaces* cut inside a Surface cell;
  `spec_registry_coverage.rs`'s test doc says what excluding them costs.
- `tools/gaps.py` ranks by case count, not by depth of question, so it nominates finished classes.
- `Core\Task::afterResponse` is the last member with a PHP twin and no oracle case; `Core\Http\Response`
  and `Core\Mail` are the thinnest classes and both need a fixture (`tools/gaps.py`).
- The 5 unasserted `thrown` paths are each documented unreachable or test-only — `csv.rs:610`'s own doc
  comment says so out loud. Judge before writing one.
