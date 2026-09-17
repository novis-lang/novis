# Handoff

## State

**Goal `decided-closures`, stage 4 — the library — is under way.**
`crates/nvs-stdlib/src/uuid.rs` has no `# Known gaps` section left: the octet pair is built, so
`python tools/owners.py --closes decided-closures` names 27 gaps where it named 28. `zip.rs`
(session 0001), `path.rs` (0002) and `random.rs` (0003) were cleared the same way.

`Core\Uuid::fromBytes(bytes $b): Uuid` and `$uuid->toBytes(): bytes` carry the sixteen octets, most
significant first, and spec § 11's second table now writes both rows. A **width** is the whole of
what `fromBytes` refuses — every 128-bit pattern is a UUID — and the throw names the width it was
handed, which is the literal `conformance_coverage.rs`'s error-path gate reads and
`uuid-from-bytes-holds-sixteen-on-both-sides.nvst` freezes.

**Stage 4's three acceptance checks are still red, and all of them for the first reason** — the
member does not exist yet. Of the first check's six tests only `an_ebml_container_is_reported_as_ebml`
is still missing, which is the group below; the other five are on disk.

## Next group

**Stage 4: `Core\Mime`'s shared EBML case, which closes the stage's first acceptance check** — one
file set: `crates/nvs-stdlib/src/mime.rs`, its `.nvst` cases under `tests/conformance/core/`, and
§ 17's detection prose in `docs/spec/01-core-library.md`. No rule owns it; the gap's own `Decided:`
sentence is the specification, and it is the same shape the class's existing `Zip` case already
takes — one honest answer about the octets rather than a guess between two containers.

- [ ] **`crates/nvs-stdlib/src/mime.rs:89` — add the `Ebml` case to the `Core\Mime\Type` enum, with
      its `CaseDoc`, and the `\x1a\x45\xdf\xa3` row to `crates/nvs-stdlib/src/mime.rs:421`'s
      `SIGNATURES`.** The case answers `video/webm` for neither container: telling WebM from
      Matroska means reading the `DocType` element, which is a parse and not a prefix, so the case
      is named for the *format* the magic identifies exactly as `Zip` is. The acceptance name is
      `an_ebml_container_is_reported_as_ebml`. Delete the numbered gap at
      `crates/nvs-stdlib/src/mime.rs:52` when it lands, and check whether § 17's own sentence about
      what deliberately has no case still reads true — `crates/nvs-stdlib/src/mime.rs:44` says that
      row defers to this module doc.
- [ ] **Three `.nvst` cases, one asking a different question each.** The floor is
      `crates/nvs-stdlib/tests/conformance_coverage.rs:155`'s
      `every_core_class_has_a_conformance_floor_of_three`, and an enum case is reached by writing
      `Core\Mime\Type::Ebml` in one of them. A `.nvst` builds the magic with
      `Core\Encoding::fromHex`, as `uuid-octets-and-canonical-text-are-one-value.nvst` does.

## Backlog

- `crates/nvs-stdlib/src/json.rs:229` gap 2, the encoder's heap stack — stage 4 check 2.
- `crates/nvs-stdlib/src/queue.rs:75` gap 2, `Core\Queue\Stats`'s fifth counter — stage 4 check 2.
- `crates/nvs-stdlib/src/xml.rs:128` gap 1, an element's namespace URI — stage 4 check 2.
- `crates/nvs-stdlib/src/regex.rs:83` gap 2, the step budget as a `[limits]` directive — stage 4
  check 3, and the one ADR slot this goal may open.
- `crates/nvs-stdlib/src/cldr.rs:212` gap 1, a literal pattern prepared once — stage 4 check 3.
- `python tools/owners.py --closes decided-closures` lists the other 22.
