# Handoff

## State

**Goal `decided-closures`, stage 4 — the library — is under way.**
`crates/nvs-stdlib/src/path.rs` has no `# Known gaps` section left: gap 1 is built and gap 2 is
struck as a stated bound, so `python tools/owners.py --closes decided-closures` names two fewer
gaps than it did. `crates/nvs-stdlib/src/zip.rs` was cleared the same way in session 0001.

`Core\Path` now parses three root shapes rather than one. `Parts::drive` is `Parts::root`, a
`Root` enum — `Unnamed`, `Drive`, `Unc { server, share }` — and every member that re-renders a
path renders the root back, so `\\server\share\f` survives `normalize`, `split`/`join` and
`dirname` with its doubled separator. `crates/nvs-stdlib/src/zip.rs:609`'s absolute-entry refusal
reads the enum instead of the old `drive` field and refuses exactly what it did before. `C:log` is
now stated as a bound in the module's own prose rather than carried as a gap: it is one relative
component, because reading `C:` as a root would make `Path::split('a:b')` answer `['a:', 'b']`.

**Stage 4's three acceptance checks are still red, and all of them for the first reason** — the
member does not exist yet. `a_unc_path_round_trips_as_a_third_root_shape` is green as of this
session; the rest of that check's tests wait on the classes below.

## Next group

**Stage 4: `Core\Random\Seeded`, the reproducible generator as its own type** — one file set:
`crates/nvs-stdlib/src/random.rs` plus its `.nvst` cases, which `-p nvs-stdlib` runs. The seeded
*engine* is already written and tested — `crates/nvs-stdlib/src/random.rs:429`'s `SplitMix` is what
a `#[Test(seed:)]` isolate draws from — so what is missing is the **type**, not the arithmetic. No
rule owns the class; the gap's own `Decided:` sentence and spec § 11 are the specification, and
`docs/agent/loop-goal.md` § *Standing decisions* names this one as a deliberate divergence from the
sheet's recommendation.

- [ ] **`crates/nvs-stdlib/src/random.rs:65` — register `Core\Random\Seeded` as an instance class
      whose members mirror `Core\Random`'s seven.** Constructed from an explicit `int` seed, each
      member drawing from `SplitMix` rather than `rand::rng()`. The five edits are
      `docs/agent/conventions.md` § *A `Core` member*; the instance half is `CoreClass::instance`
      and `CoreClass::slots` at `crates/nvs-stdlib/src/registry.rs:1363`, with
      `crates/nvs-stdlib/src/cache.rs:323` (`Core\Cache\Store`, `slots: &["tier"]`) as the worked
      example of a `Core`-owned instance and `crates/nvs-stdlib/src/lib.rs:11` for the
      `registry::CLASSES` line a new class adds. The acceptance name is
      `core_random_seeded_gives_the_same_sequence_for_the_same_seed`. Say what one instance spends
      in the module doc's `# What it spends` section, which already prices the thread-local
      generator (`rule:programs/memory-priority`), and delete the numbered gap when it lands.
- [ ] **`crates/nvs-stdlib/src/random.rs:79` — rewrite `# The one exception` for the new type.**
      That section says a declared seed is the *only* way to reach a predictable generator; once
      `Seeded` exists it is the second, and the argument that survives is that the distinction is
      a type a reader can see. Same file, same slice-set; `docs/novis.md`'s `Core\Random` chapter
      says there is no seeded generator under any name and is wrong the moment the row lands.

## Backlog

- `crates/nvs-stdlib/src/uuid.rs` — the bytes pair, a spec § 11 amendment
  (`docs/agent/loop-goal.md` § *Standing decisions*); check
  `a_uuid_round_trips_through_its_sixteen_bytes`.
- `crates/nvs-stdlib/src/mime.rs` — an EBML container is reported as EBML; check
  `an_ebml_container_is_reported_as_ebml`.
- Stage 4's second and third checks — `json.rs`'s heap stack, `queue.rs`'s fifth counter and boot
  refusal, `xml.rs`'s namespace URI, `regex.rs`'s budget directive and the prepared-pattern channel
  (the goal's one ADR slot).
- `python tools/owners.py --closes decided-closures` is the gate the goal ends on; it still names
  the gaps above.
