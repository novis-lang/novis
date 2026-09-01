# Handoff

## State

**Spec § 14's metadata pair, its listing and its resolver are registered and cased.**
`Core\IO::isFile`, `::isDir`, `::list` and `::canonicalize` are on disk over five new
conformance cases, and the ratchet
`crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` is down from 59 keys to
55. Two new doors carry them: `nvs_runtime::capability::metadata_if_present`, whose
`None` is what lets `isFile`/`isDir` answer `false` for absence in one `stat` instead of
losing a race between an `exists` and a `metadata`, and `::resolve_existing`, which is
`capability::canonicalize`'s walk plus an existence question so that
`Core\IO::canonicalize` refuses a missing name without answering a second *spelling* of
the paths `within` already answers. Both doors' own doc comments own that reasoning.

**Stage 10's conformance check was failing on a directory layout the corpus never
adopted**, not on unwritten work; it now names the thirteen landed cases that pin the
claims it was drafted for, in `docs/agent/loop-goal.toml` and in
`docs/agent/goals/4-core-part-ii.toml` alike. The playbook bullet is the whole triage.

**Stage 10's remaining open check is still the goal's own measure**: `check-migration
--min 74` reads 37%.

Nothing was missing from this session's pack.

## Next group

**Finish § 14. One file set: `crates/nvs-stdlib/src/io.rs`,
`crates/nvs-runtime/src/capability.rs`, `crates/nvs-stdlib/src/registry.rs` and
`tests/conformance/core/`.**

- [ ] **Register `append` over the door that already exists.** `capability::open` takes
      `Access::Append` and `Core\IO::open` already reaches it, so this is the row, the card, the
      body and the cases and no new door — the one remaining § 14 member with nothing to decide.
      `crates/nvs-stdlib/src/io.rs:99`, `crates/nvs-runtime/src/capability.rs:226`.
- [ ] **Register `copy`, `move` and `makeDir`, and add the three `fs.write` doors they need.**
      Each door asks `require` twice where two paths are involved — `copy` and `move` are a read of
      one and a write of another, and the table in `registry::CAPABILITIES` names one cap per
      member, so it names the stronger, exactly as `open`'s row already does and for the reason its
      comment gives. `crates/nvs-runtime/src/capability.rs:483`,
      `crates/nvs-stdlib/src/io.rs:169`.
- [ ] **Register `modifiedAt` and `stat`, and decide what each answers with.** `modifiedAt` wants
      `Core\Time\Instant`, which is `pub` already and needs a constructor reachable from `io.rs`;
      `stat` needs either a `Core\IO\Stat` instance or a fixed-key shape, and the ADR 0063 R-rules
      plus `instance.rs` decide which. `crates/nvs-stdlib/src/time.rs:1111`,
      `crates/nvs-stdlib/src/io.rs:157`.
- [ ] **Register `isReadable` and `isWritable`, and record what they measure.** They were left out
      of this session deliberately: Rust's std has no `access(2)`, so the honest answers are
      *effective access measured by attempting the open* or *the permission bits*, and the two
      differ for a directory and for a process running as root. Decide it in the door's doc comment
      and case the boundary. `crates/nvs-runtime/src/capability.rs:340`,
      `crates/nvs-stdlib/src/io.rs:139`.

## Backlog

- `walk` — § 14's streaming listing, the `Iterable<string>` half of `list`; `io.rs`'s `LINES` is the
  shape a lazily-answered `Core` iterable takes.
- `check-migration --min 74` reads 37% and is stage 10's last open check — the goal's own measure,
  and no single slice moves it.
- § 15's request-facing half and § 18's driver surface stay goal 6's and goal 5's, per the ratchet
  file's own header.
