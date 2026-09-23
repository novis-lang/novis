# Handoff

## State

Goal `core-io-file-and-1-more` is complete: all thirteen members carry every proof of
`rule:testing/feature-proofs`. The nine `Core\IO\File` members landed earlier. The four
`Core\IO\Metadata` members (`size`, `modifiedAt`, `isFile`, `isDir`) landed this session: the
class card (`METADATA_CARD`, and `Core\IO\Metadata` struck from `CLASSES_STILL_OWING_A_CARD`), one
Rust test per member, a `// covers:` line on the stat-snapshot `.nvst` case, three examples and an
`about.md` each, one attack each and one bench each. The root `nvs.toml` grants `fs` to the three
`IO-Metadata` proof trees. `docs/perf/members.ndjson` holds their first records, all at 0
allocations. `owners.py --closes` and `playbook.py --closes` name nothing for this goal.

## Next group

**Goal complete** — the driver picks the next goal in the chain; its first session overwrites
this file.

- [x] **`Core\IO\Metadata` proofs** — `crates/nvs-stdlib/src/io.rs:1811`, done.

## Backlog

- `nvs agent show 'Core\IO\File'` and `'Core\IO\Metadata'` print the member list but not the class
  card's `short` text. Not checked whether another class's card prints there; owner:
  `crates/nvs-stdlib/src/registry.rs`.
