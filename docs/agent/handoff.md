# Handoff

## State

**Spec § 5 is whole.** `Core\Regex::replaceWith` landed: the callback is handed one `Core\Regex\Match`
rather than PHP's positional array — the spec's own `callable(Match): string`, stated at
`docs/spec/01-core-library.md:562` — and what it answers is inserted **literally**, so a `$1` in it is two
characters unlike `replace`'s template. Every match is collected before the first call rather than stepping
an engine's iterator across user code; `crates/mwl-stdlib/src/regex.rs`'s helper doc owns that reasoning
and what it spends. That module's gap list is now three, renumbered: the ADR 0056 § 4 sink, the budget
constant, and `matchAll`'s per-match offset conversion.

The ratchet (`crates/mwl-stdlib/tests/spec-members-outstanding.txt`) is down to **4 keys**: `§2 from`,
`§11 Hash::stream`, `§11 Random::bytes`, `§12 Out::capture`. Conformance is 414 of 600 and differential
89 of 150. `examples/collect.mwl`'s frontier is still `Core\Out::capture` at `collect.mwl:47`, which lands
with M4S; `§2 from` waits on an `Iterable`/`Iterator` argument. So the two keys that are registerable
*today* are both § 11's, and they are the next group.

Two open temporary-lifetime gaps of one family are named where they live: `mwl_ir::lower::Lowering`'s
`owned_temporaries` field doc holds the transferred-argument case, and `landing_block`'s *Known gap* holds
the producers that still release inline. `mwl-ir`'s crate doc gap 2 is the index of both.

## Next group — § 11's two `bytes`-valued rows

**Shared file set:** `crates/mwl-stdlib/src/hash.rs` and `crates/mwl-stdlib/src/random.rs`, plus one
`.mwlt` case each under `tests/conformance/core/`. [1] and [2] are the pair worth taking together — both
are § 11, both answer in `bytes`, and both assert through `Core\Encoding::toHex`. [3] is a separate
sitting and shares no file with them.

- [ ] **`Core\Random::bytes`** — spec row at `docs/spec/01-core-library.md:739`,
      `bytes(uint $count): bytes`. All in `random.rs`: the row goes beside `token` (`random.rs:99`, which
      already draws the same entropy and hex-renders it), `CLASS` is at `random.rs:81`, the `address()`
      arm at `random.rs:141`. `mwl_runtime::Tag::Bytes` exists now, so this is `token`'s body without the
      hex step. A case cannot assert the value — assert `Core\Bytes::length` of it, and that two calls
      differ.
- [ ] **`Core\Hash::stream`** — spec row at `docs/spec/01-core-library.md:760`,
      `stream(Digest $digest): Hash\Stream`. This one **registers a second class**: `Hash\Stream` is a
      *mutable* handle (`update(bytes|string): void`, `finish(): bytes` — read the spec's own paragraph
      under that table before fixing the roster), where `Core\Regex\Pattern` was an immutable one. Copy
      `Pattern`'s shape from `regex.rs:181`-onwards; `hash.rs:167` is `CLASS`, `hash.rs:119` is the
      `DIGEST` enum the argument takes, `hash.rs:199` is `address()`. Strike both keys as they land.
- [ ] **Stage 4's counts are their own work** — conformance 414 of 600, differential 89 of 150. Neither
      grows as a side effect of member slices any more; `docs/implementation-plan.md`'s *Open now* says so.

## Backlog

- `§2 Core\Arr::from` — needs an `Iterable`/`Iterator` argument shape (`docs/spec/01-core-library.md` § 2).
- `§12 Core\Out::capture` — lands with M4S's sink work, ADR 0092 (`docs/implementation-plan.md` *Open now*).
- `§6 Core\Json::decodeAs<T>` — `crates/mwl-stdlib/src/json.rs` gap 2; a written call-site type argument
  works now.
- `§9 Core\Heap` and the `Iterable` its three rows declare — same blocker as `Arr::from`.
- ADR 0088's registry-wide qualifier classification — no `Core` member row can say `tainted`/`secret` yet;
  `regex.rs` gap 1 is the sharpest instance.
- `do`/`while` does not lower — `mwl-ir`'s crate doc.
