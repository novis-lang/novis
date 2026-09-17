# Handoff

## State

**Goal `decided-closures`, stage 4 — the library — is under way.**
`crates/nvs-stdlib/src/mime.rs` has no `# Known gaps` section left, so
`python tools/owners.py --closes decided-closures` names 26 gaps where it named 27. `zip.rs`
(session 0001), `path.rs` (0002), `random.rs` (0003) and `uuid.rs` (0004) were cleared the same way,
and `docs/agent/carried-gaps.md`'s EBML bullet is gone with the gap it indexed.

**Stage 4's first acceptance check is green**: `Core\Mime\Type::Ebml` is the case an EBML container
answers, spelled `video/matroska` — the one IANA type true of a Matroska file and of a WebM one,
WebM being a Matroska profile. It never narrows to `video/webm`, because which of the two a file is
lives in the `DocType` element, which is a parse and not a prefix.

Checks 2 and 3 of the stage are still red, both for the first reason — the member does not exist yet.

## Next group

**Stage 4: `Core\Queue`'s two gaps, which are two of stage 4's second acceptance check** — one file
set: `crates/nvs-stdlib/src/queue.rs`, its `.nvst` cases under `tests/conformance/core/`, and § 6's
`stats` prose plus § 2's schema in `docs/spec/01-core-library.md`. No rule owns the counter;
`rule:concurrency/enqueue-commits-with-your-write` owns the queue the boot check refuses to serve.
Both gaps' own `Decided:` sentences are the specification, and both move the same schema converge.

- [ ] **`crates/nvs-stdlib/src/queue.rs:75` — a fifth counter on `Core\Queue\Stats`**, the column
      added through the `nvs queue migrate` converge in `schema` and summed in `COUNTS_POSTGRES`
      beside the four § 6 already names. The sheet's answer is the spec § 6 amendment, so § 6's
      table gains the row in the same slice. The acceptance name is `queue_stats_has_a_fifth_counter`.
      Delete the numbered gap when it lands.
- [ ] **`crates/nvs-stdlib/src/queue.rs:64` — refuse to serve a queue whose schema is behind,
      checked at boot**, which is what is left of gap 1: `nvs_jobs_dedupe` makes the statement
      race-free wherever the schema carries it, and the one case it does not is a deployment that
      never ran `nvs queue migrate`. Costs no request time by construction — the check is at boot.
      The acceptance name is `a_queue_whose_schema_is_behind_is_refused_at_boot`.

## Backlog

- `crates/nvs-stdlib/src/xml.rs:128` gap 1, an element's namespace URI — stage 4 check 2.
- `crates/nvs-stdlib/src/json.rs:229` gap 2, the encoder's heap stack — stage 4 check 2.
- `crates/nvs-stdlib/src/regex.rs:83` gap 2, the step budget as a `[limits]` directive — stage 4
  check 3, and the one ADR slot this goal may open.
- `crates/nvs-stdlib/src/cldr.rs:212` gap 1, a literal pattern prepared once — stage 4 check 3.
- `Core\Mime` has four conformance cases where the floor is three; a fifth asking the same question
  as one of them is not owed.
- `python tools/owners.py --closes decided-closures` lists the other 22.
