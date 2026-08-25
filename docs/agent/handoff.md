# Handoff

## State

**Spec § 1 is two rows shorter, and the two of them are one window.**
`Core\Str::replaceAll` and `Core\Str::replaceRange` are registered, implemented and pinned by a
conformance case each. `replaceAll` takes `strtr`'s reading — one left-to-right pass, longest needle
first, replacements never rescanned — because that is the half of PHP's two whose answer does not
depend on the order the pairs were written; its doc comment at `str.rs` owns the reasoning.
`replaceRange` and `slice` now read `$offset`/`?$length` through one shared `window()` helper
(`crates/mwl-stdlib/src/str.rs:1200`), so `replaceRange($s, $o, $n, "")` removes exactly what
`slice($s, $o, $n)` returns for every sign of every argument — PHP's own pair does not manage that.
Every row of both cases was checked against `php -r` while authoring.

**The loop's gate is the ratchet in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`**, now at
**38 keys** for §§ 1-12; when it is empty, Part I is registered whole. That test's own module doc says
why it is a ratchet rather than a red assertion. Conformance is **391** of 600; differential is 86 of
150 and has not moved.

**`examples/collect.mwl`'s frontier is unchanged** — `Core\Uri::parseQuery` at `collect.mwl:36`, then
`Core\Csv::parse`, `Core\Validate::isEmail`, `Core\Out::capture`.

**§ 1 now owes three rows**: `compare`, `fold` and `normalize`. Only `compare` needs no new
dependency; the other two are the group below's second half.

## Next group — the last of § 1

Three slices, all in one file. Take `compare` first: it is the only one that needs no dependency, and
landing it leaves § 1's remainder as a single dependency question rather than two.

**Shared file set:** `crates/mwl-stdlib/src/str.rs` (`:49` `CLASS`'s rows — `:120` `countOf` is
`compare`'s neighbour and `:295` `lowerFirst` is `fold`'s; `:451` `address()`; `:1894` the test
module), `crates/mwl-stdlib/tests/spec-members-outstanding.txt`, and `tests/conformance/core/`.

- [ ] **`Core\Str::compare`** (`01-core-library.md` § 1 *Comparison*, `:110`).
      `compare(string $a, string $b, {caseInsensitive?: bool, natural?: bool}): int`, replacing all
      four of `strcmp`/`strcasecmp`/`strnatcmp`/`strnatcasecmp`. `{natural: true}` is a **different
      ordering**, not a variant — the spec's prose under that table states it and gives
      `compare("img12", "img2")` as the sign that flips. Return `-1`/`0`/`1` rather than a byte
      difference, and say so in the doc comment: PHP 8 already normalized `strcmp` that way, and a
      comparator is the only consumer. `match_at` at `str.rs:1001` is the case-insensitive comparison
      already written; ordering itself is `Ord` over `str`, not a collation (`strcoll` is refused by
      the spec's own note). Strike `§1 compare`.
- [ ] **`Core\Str::fold`** (§ 1 *Transformation*, `:165`). `fold(string $s): string` is full Unicode
      case folding for caseless comparison — **not** `lower`, which is why it is its own row. Pick the
      crate under ADR 0051 § 4 (`unicode-segmentation` at `Cargo.toml:62` is the tree's existing
      precedent for a Unicode table dependency) and record the pick in `str.rs`'s module doc. A new
      dependency owes three things — see `playbook.md` § *Adding a `Core` member*. Strike `§1 fold`.
- [ ] **`Core\Str::normalize`** (§ 1 *Transformation*, `:166`). `normalize(string $s, NormalForm $form)`
      needs a `Core\NormalForm` enum registered in `registry::ENUMS` (`registry.rs:706`) and referred
      to as `CoreTy::Enum` (`registry.rs:216`); `unicode-normalization` is the obvious pure-Rust pick,
      same ADR 0051 § 4 test as `fold`'s. Strike `§1 normalize`.

## Backlog

- § 2's `Arr::diff`/`intersect` and ADR 0069's combination members — plan § *Open now*.
- § 12's `Uri::parseQuery`, the fixture frontier — `docs/agent/loop-goal.md` acceptance list.
- § 9's `Core\Heap` and the `Iterable` its three rows declare — plan § *Open now*.
- § 10's `{previous: $e}` constructor options shape and `$e->location` — ADR 0071 § 5.
- ADR 0088's registry-wide qualifier classification for member rows — plan § *Open now*.
- `do`/`while` does not lower — `mwl-ir`'s own module doc.
