# Handoff

## State

**Stage 10's three gates are on disk and green.**
`crates/nvs-stdlib/tests/spec_registry_coverage.rs` now walks spec §§ 14-19 once, in
`part_two_members()`, and asks the result two questions: is it registered
(`every_part_two_spec_member_is_registered`, over the 59-key ratchet
`tests/spec-members-part-two-outstanding.txt`), and does a conformance case call it
(`every_part_two_member_has_a_conformance_case`, over the 27 members that *are* registered).
`crates/nvs-stdlib/tests/tier_boundary.rs` is ADR 0051 § 5's
`no_class_outside_tier_zero_registers_a_core_name`, over 53 module `CLASS` consts and both rosters.

**How the conformance corpus is read now has one home**, `crates/nvs-stdlib/tests/corpus/mod.rs`,
because Cargo compiles each `tests/*.rs` as its own crate and both gate files ask the same corpus
the same question from opposite ends. `Attribution::asked` is the rule — every class a case holds,
mapped to every member it names on it — and the floor gate and the Part II case gate share it
rather than each spelling it out.

**The Part II case gate is implied by two gates that already pass, and its own doc says so.**
Spec → registry → corpus is closed by `every_part_two_spec_member_is_registered` plus
`conformance_coverage.rs`'s pair; what the new one adds is the *direction*, so a member registered
and then left uncased fails named by its spec section rather than by a class.

**§§ 16 and 17 are still unguarded by any registry walk**, on the same line § 13 is: both are
`| Class | Surface | ADR |` tables with the members inside an English cell. Ten classes are gated
only by `conformance_coverage.rs`, which walks the registry and so cannot see a spec row that was
never implemented. `part_two_members`'s own doc owns that exclusion.

Nothing was missing from this session's pack.

## Next group

**Stage 10's own remaining checks, then § 16's unguarded half. One file set:
`crates/nvs-stdlib/tests/` and `tools/`.**

- [ ] **Take stage 10's check to a verdict.** All six of its `cargo-named` tests now exist, so the
      stage's other three checks are what it fails on next: the C-dependency ledger, the
      conformance floor of 1250, and `check-migration --min 74`. The ledger is the one no session
      has touched — `python tools/gen-attribution.py --check-c-deps` — and it is the likeliest.
      `docs/agent/loop-goal.toml:2576`, `tools/gen-attribution.py:1`.
- [ ] **Give § 16 the fifth of six sections.** Its `| Class | Surface | ADR |` cell is English, but
      the *class* column is a code span, so a class-level gate is available where a member-level
      one is not: every class § 16 names has a `registry::CLASSES` row or a ratchet key. Same shape
      as the member ratchet, and it catches a whole class going missing.
      `crates/nvs-stdlib/tests/spec_registry_coverage.rs:592` (`part_two_members`),
      `crates/nvs-stdlib/tests/spec_registry_coverage.rs:209` (`classes_in`).
- [ ] **Strike ratchet keys by registering § 14's directory and metadata half.** 59 keys remain and
      § 14's need no external dependency — `append`, `canonicalize`, the directory members.
      `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:17`,
      `crates/nvs-stdlib/src/io.rs:1`.

## Backlog

- § 17's documents and formats — `docs/agent/loop-goal.md` item 29, stage 9.
- The five-driver `Core\Db` matrix needs a Docker daemon — `docs/plan/m8.md` *Verify*.
- `Core\Session`, `Core\Metrics`, `Core\Router::match` and `Core\Queue` are goals 5 and 6's.
- `BELOW_THE_FLOOR` and `OWED_A_CASE` are both closed ratchets — `conformance_coverage.rs`.
