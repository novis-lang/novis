# Handoff

## State

**Goal `decided-closures`, stage 4 — the library — is under way.** `crates/nvs-stdlib/src/zip.rs`
has no `# Known gaps` section left: both of its items are built to their `Decided:` sentences, and
`python tools/owners.py --closes decided-closures` now names 31 gaps, down from 33.

**The tree builds again, and session 0010's `BLOCKED` is spent.** The other writer's `nvs run
--count` work landed as `7da961c5c..c5cb15a88`, so `nvs-cli` compiles and the `.nvst` trees run. The
acceptance check that failed after 0010 — `the pipeline operator owes no proof [1 floor]`, stale
because `docs/reference/lang/30-expressions.md` had changed — passes at this commit: `python
tools/dossier.py --only lang:expressions/the-pipeline-operator --gate` answers "nothing owed". It
was stale rather than red, and nothing in this session was needed to clear it.

`Core\Zip` now reads a zip64 archive rather than refusing it as malformed, and checks every entry's
CRC-32 before answering or writing it. Both are the module doc's own paragraphs; `Budget::read` is
the one route, so `read` and `extract` get the checksum from the same place they get the bound.
Stage 3's three checks are green as they were.

## Next group

**Stage 4: `Core\Path`'s two root shapes** — one file set: `crates/nvs-stdlib/src/path.rs` and the
`#[test]`s that pin it, which `-p nvs-stdlib` runs. No rule owns the path grammar (`python
tools/brief.py --where path` finds only the refusal and scope rules), so each gap's own `Decided:`
sentence is the specification, and `docs/agent/loop-goal.md` § *Stage 4* lists gap 1 under *builds
to their sentence* and gap 2 under *struck as stated bounds*.

- [ ] **`crates/nvs-stdlib/src/path.rs:50` — a UNC root is a third root shape beside the drive
      letter.** Gap 1's `Decided:` sentence is the decision: `\\server\share\f` parses today as an
      ordinary absolute path whose components are `server`, `share` and `f`, so re-rendering loses
      the doubled separator that makes it UNC. A third root case beside `Parts::drive`, not a change
      of interface. The acceptance name is `a_unc_path_round_trips_as_a_third_root_shape`. Say what
      the third shape spends in the module doc's own prose (`rule:programs/memory-priority`), and
      delete the numbered item when it lands.
- [ ] **`crates/nvs-stdlib/src/path.rs:58` — strike gap 2 as a stated bound.** The answer was *no*:
      `C:log` stays one relative component, because treating `C:` as a root would make
      `Path::split('a:b')` answer `['a:', 'b']`. Per the goal's § *Standing decisions*, an answer of
      "state it as a bound" is written as the module's own grammar prose and the numbered item and
      its owner tag are deleted — a bound is not a gap.

## Backlog

- `crates/nvs-stdlib/src/random.rs:67` — `Core\Random\Seeded`, registered per spec § 11; acceptance
  `core_random_seeded_gives_the_same_sequence_for_the_same_seed`.
- `crates/nvs-stdlib/src/uuid.rs:104` — the `bytes` round trip, a spec § 11 amendment; acceptance
  `a_uuid_round_trips_through_its_sixteen_bytes`.
- `crates/nvs-stdlib/src/mime.rs:52` — one shared `Ebml` case; acceptance
  `an_ebml_container_is_reported_as_ebml`.
- The rest of stage 4 is `docs/agent/loop-goal.md` § *Stage 4*, which groups every remaining item by
  what its answer was; the ADR slot it reserves is the prepared-pattern channel and nothing else.
- Two slices in one file cannot be two commits: `session.py --wrap` stages by path, so this
  session's zip64 and CRC slices landed as one commit naming both.
