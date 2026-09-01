# Handoff

## State

**Spec § 14's four remaining writers are registered and cased.** `Core\IO::append`, `::copy`,
`::move` and `::makeDir` are on disk over twelve new conformance cases, and the ratchet
`crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` is down from 55 keys to 51. What is
left of § 14 is `modifiedAt`, `stat`, `isReadable`, `isWritable` and `walk`.

`append` needed no door — `nvs_runtime::capability::open` already takes `Access::Append`, so the
member is the row, the card, the body and the cases. **Three doors are new, and each owns its
decision in its own doc comment**: `capability::copy` asks `fs.read` for the source and `fs.write`
for the destination and checks both before it uses either, replacing a destination that is taken
exactly as `write` replaces; `capability::rename` asks `fs.write` for **both** ends, because taking
the source away is destroying it, and refuses a cross-filesystem move rather than becoming a copy
and a delete; `capability::create_dir` creates every missing parent and treats an existing directory
as success, which is the mirror image of `remove_dir`'s refusal to recurse — a recursive creation
makes empty directories under the path just checked, a recursive removal is one check standing in
for a tree of destructions. `capability::pair` is the `from -> to` spelling a two-path `IOError`
names, since either end can be the one at fault.

**Stage 10's remaining open check is still the goal's own measure**: `check-migration --min 74`
reads 37%, unmoved by these four.

Nothing was missing from this session's pack.

## Next group

**Finish § 14. The same file set: `crates/nvs-stdlib/src/io.rs`,
`crates/nvs-runtime/src/capability.rs`, `crates/nvs-stdlib/src/registry.rs` and
`tests/conformance/core/`.**

- [ ] **Register `modifiedAt` and `stat`, and decide what each answers with.** `modifiedAt` wants
      `Core\Time\Instant`, which is `pub` already and needs a constructor reachable from `io.rs`;
      `stat` needs either a `Core\IO\Stat` instance or a fixed-key shape, and the ADR 0063 R-rules
      plus `instance.rs` decide which. One `capability::metadata` call answers both, which is that
      door's own stated reason for handing back the whole `Metadata`.
      `crates/nvs-stdlib/src/time.rs:1111`, `crates/nvs-stdlib/src/io.rs:169`,
      `crates/nvs-runtime/src/capability.rs:405`.
- [ ] **Register `isReadable` and `isWritable`, and record what they measure.** Rust's std has no
      `access(2)`, so the honest answers are *effective access measured by attempting the open* or
      *the permission bits*, and the two differ for a directory and for a process running as root.
      Decide it in the door's doc comment and case the boundary; `metadata_if_present`'s `None` is
      the shape for "nothing is there", which both members have to answer `false` for.
      `crates/nvs-runtime/src/capability.rs:430`, `crates/nvs-stdlib/src/io.rs:160`.
- [ ] **Register `walk`, § 14's streaming listing.** The `Iterable<string>` half of `list`, over
      `capability::read_dir` recursively — `LINES` and `nvs_core_io_lines` are the shape a lazily
      answered `Core` iterable takes, including its `LINES_ITERATE_SYMBOL` arm in `address`.
      `crates/nvs-stdlib/src/io.rs:1587`, `crates/nvs-stdlib/src/io.rs:229`,
      `crates/nvs-runtime/src/capability.rs:557`.

## Backlog

- `check-migration --min 74` reads 37% and is stage 10's last open check — the goal's own measure,
  and no single slice moves it.
- § 15's request-facing half and § 18's driver surface stay goal 6's and goal 5's, per the ratchet
  file's own header.
- `Core\IO::copy` is file-only, as `std::fs::copy` is; a directory copy is a walk the program
  writes, and nothing yet says so where a developer would look.
