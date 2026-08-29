# Handoff

## State

**Corpus at 944, all passing** (941 → 944). The Stage 8 acceptance check wants 950 and still fails:
a growth floor, not a regression — no case fails and no check names a test that disappeared. It
needs roughly two more sessions of this shape.

**The whole next group landed, and it closed Stage 5 item 10.** `Core\Router::urlAbsolute` goes from
2 cases to 5 — its descriptions were all refusals until now, so nothing said what it *answers* — and
it was the last name on `BELOW_THE_FLOOR` in `crates/nvs-stdlib/tests/conformance_coverage.rs`. That
list is **empty**: every `Core` member is at or above the floor of three, and the gate is now purely
the ratchet that stops the next thin one arriving. **No `nvs-stdlib` source changed** otherwise:
every member the three cases reach already answered correctly.

**What made the group possible, and nothing else records it.** A `.nvst` case configures its own run
through a `--FILE nvs.toml--` auxiliary section, so § 6's configured origin is reachable off the
command line after all — `router.rs`' gap 2 and the existing refusal case both read as though it
were not. There is a playbook bullet under *Writing a test case*, including the second half of the
trap: the repo root's own `nvs.toml` sets an origin, so a scratch probe run there passes for the
wrong reason.

**Three facts the new cases pin.** (1) `urlAbsolute` is exactly `origin . url` on all eleven row
shapes the seven `router-url-*` cases build — three values of a closed set, a computed value outside
it, an escaped value, a `{rest...}`, the root, an optional capture absent and present, a leftover
`params` key and a nested one beside a dropped `null` — counted, with the complement that a
configured origin changes nothing about `url`. (2) The origin has exactly one source: four near
misses in the same file (`origin` before any table, one under `[server]`, one commented out inside
`[app]`, one under `[app.dev]`) are all walked past, and the scheme and explicit port cross over
verbatim. (3) A trailing `/` on the configured origin is dropped exactly once, so the join leaves one
separator in every cell — asserted on both sides, since the first case's origin has no trailing `/`
and this one's does.

**What the three new cases are**, all in `tests/conformance/core/`:
`router-url-absolute-is-url-with-the-configured-origin-in-front.nvst`,
`router-url-absolute-takes-its-origin-from-app-origin-and-from-nowhere-else.nvst`,
`router-url-absolute-joins-the-origin-and-the-path-with-exactly-one-slash.nvst`.

**Untouched:** item 12's roster (10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526`),
a `bytes` array key ICE in `nvs-ir`, and `catch (Core\Error $e)` panicking — the last two have
playbook bullets under *Writing a test case*.

**Orientation.** The pack was complete for the item it was given. `[context] modules` now carries
`crates/nvs-stdlib/src/arr.rs`, `crates/nvs-test/src/case.rs` and `crates/nvs-runtime/src/ctx.rs` —
this session added them rather than asking a fifth time; the last two are what any depth session
needs and the first is the next group's file. Still owed: `hash.rs`, `test.rs`,
`nvs-types/src/links.rs`, `routes.rs`, `validate.rs`, `docs/spec/01-core-library.md`,
`docs/spec/02-php-migration.md`, `tools/check-migration.py`, the stage-7 comment header's per-goal
floor table, and a selector printing the failing check's own `cases` block.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/arr.rs` and `tests/conformance/core/`. `Core\Arr` is the
thinnest class left in `python tools/gaps.py --coverage` — floor **3** over 55 members and 230 cases
— and its three thinnest members are the three below. Each slice names the cases that already exist
for its member, so the session starts by reading those rather than re-deriving them; what is left in
every one of them is a shape, not a row.

- [ ] **`column`'s answer is one cell per row that has one, over a whole table** (`nvs_core_arr_column`
      at `crates/nvs-stdlib/src/arr.rs:2278`) — existing: `arr-column-takes-one-cell-out-of-every-row.nvst`
      and `arr-column-appends-under-the-next-free-integer-and-both-key-spellings-agree.nvst`. Left is
      *invariance over a sweep*: rows missing the named key, rows whose cell holds `null`, and an
      index key that repeats across rows — counted, rather than read off a line.
- [ ] **`fillKeys` is `combine` with one value repeated** (`nvs_core_arr_fill_keys` at `:2404`) —
      existing: `arr-fill-and-fill-keys-repeat-one-value.nvst` and
      `arr-fill-is-the-shared-run-and-fill-keys-is-its-start-index.nvst`. Left is the *agreement*:
      one key list asked of `fillKeys` and of `Core\Arr::combine` over the same value repeated, plus
      the degenerate cells — an empty key list, and a key list that repeats a key.
- [ ] **`firstKey`/`lastKey` name the ends of the same order `foreach` walks** (`nvs_core_arr_first_key`
      at `:3422`, `nvs_core_arr_last_key` at `:3437`) — existing:
      `arr-first-and-last-answer-null-over-an-empty-array.nvst` and
      `arr-key-answering-members-agree-on-one-spelling.nvst`. Left is the *agreement* over a script of
      writes and removals, the shape
      `object-collections-are-one-collection-asked-two-ways.nvst` uses, applied to `array<T>`'s own
      insertion order: a write over a held key moves it nowhere, remove-then-write moves it to the
      end, and `keys()`' two ends are these two members at every step.

## Backlog

- Item 12's roster: 10 `UNCLASSIFIED` members at `crates/nvs-stdlib/src/registry.rs:1526` — ADR 0088 § 2.
- A `bytes` value used as an array key ICEs in `nvs-ir` — `docs/agent/playbook.md`, *Writing a test case*.
- `catch (Core\Error $e)` panics rather than diagnosing — `docs/agent/playbook.md`, *Writing a test case*.
- `urlAbsolute` under ADR 0097 § 3's *mount* prefix has nowhere to come from — `router.rs`' known gap 2.
- `Core\Math`'s `lcm`/`gcd`/`hypot`/`atan2` read as floor 3 but are pinned to depth; `gaps.py --coverage`
  counts calls per member and cannot see it — `docs/agent/playbook.md`, *Tooling*.
- `Core\Bytes` and `Core\Uuid` read as thin and are not: their thinnest members each already have a
  case per shape — check the file list before taking either as a group.
