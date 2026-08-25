# Handoff

## State

**Spec § 1 is one row from whole.** `Core\Str::compare` and `Core\Str::fold` are registered,
implemented and pinned by a conformance case each. `compare` answers `-1`/`0`/`1` — never a byte
difference — and folds four PHP functions into one member with two options; `{natural: true}` is a
*port* of `strnatcmp` rather than a fresh reading of "sort digit runs as numbers", quirks included,
and `natural_order`'s doc comment at `crates/mwl-stdlib/src/str.rs:1576` owns all three of them.
`fold` is Unicode's full case folding — a comparison key, not a rendering — and it is deliberately
what `compare`'s per-character `{caseInsensitive: true}` is not: `ß`/`SS` needs the fold.

**The tree gained one dependency**, `caseless` (the folding table at Unicode 16.0, pure Rust), with
`unicode-normalization` under it — which is the crate § 1's last row wants, so `normalize` now owes a
`CoreEnum` and a binding rather than a dependency argument. `cargo deny check` green,
`THIRD-PARTY-LICENSES.txt` regenerated, no licence identifier new to `deny.toml`.

**The loop's gate is the ratchet in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`**, now at
**36 keys** for §§ 1-12; when it is empty, Part I is registered whole. Conformance is **393** of 600;
differential is 86 of 150 and has not moved.

**`examples/collect.mwl`'s frontier is unchanged** — `Core\Uri::parseQuery` at `collect.mwl:36`, then
`Core\Csv::parse`, `Core\Validate::isEmail`, `Core\Out::capture`.

## Next group — close § 1, then open `Uri`'s query half

Three slices, and **they do not share one file**: take `normalize` alone unless you are far under the
gate, because `[2]` opens a file you have not loaded. `[2]` and `[3]` are the pair that shares a file
set — a session starting there should take both.

**Shared file set for `[2]`/`[3]`:** `crates/mwl-stdlib/src/uri.rs` (`:122` `CLASS`'s rows, `:126`-`:147`
the four percent-encoding members already built, `:161` `address()`, `:413` the test module),
`crates/mwl-stdlib/tests/spec-members-outstanding.txt`, `tests/conformance/core/`, and
`examples/collect.mwl` + its frozen output.

- [ ] **`Core\Str::normalize`** (`01-core-library.md` § 1 *Transformation*, `:166`; the enum at `:196`).
      `normalize(string $s, NormalForm $form): string`, over `NormalForm { Nfc, Nfd, Nfkc, Nfkd }`.
      The enum is a `CoreEnum` declared beside the member — copy `crate::encoding::CHARSET`'s shape —
      and listed in `registry::ENUMS` at `crates/mwl-stdlib/src/registry.rs:706`. The binding is
      `unicode_normalization::UnicodeNormalization`'s four iterators, already in the tree under
      `caseless`; add it to `[workspace.dependencies]` and `crates/mwl-stdlib/Cargo.toml` explicitly
      rather than relying on the transitive edge. Member anchors in `str.rs`: `:311` the `fold` row
      (`normalize`'s neighbour), `:527` `address()`, `:1985` `fold`'s helper, `:2230` the test module's
      `s()`. Strike `§1 normalize` and § 1 is whole.
- [ ] **`Core\Uri::parseQuery`** (§ 12; `loop-goal.md` § *Standing decisions* amends the spec row).
      PHP's bracket convention **in full** — `a[]=1&a[]=2` a list, `a[b]=c` a map, nesting to any
      depth — so the return type is not `array<string>`; pick the spelling the type system can now
      state (`?T`/`mixed` landed, `mwl_ir::Ty::Tagged`), write it into the § 12 table, and say in that
      table that this settles what `Core\Request::query` answers at M8. `decodeFormValue` at
      `uri.rs:147` is the per-value half, already written. Strike `§12 Uri::parseQuery`.
- [ ] **`Core\Uri::buildQuery`** (§ 12) — the inverse, same file, same convention, and the round trip
      with `[2]` is the conformance case both want. `encodeFormValue` at `uri.rs:140` is its per-value
      half. Strike `§12 Uri::buildQuery`.

## Backlog

- § 2's `Arr::diff`/`intersect` and ADR 0069's combination members — plan § *Open now*.
- § 12's `Uri::parse`/`isValid`, which still owe an RFC 3986 dependency picked under ADR 0051 § 4.
- § 9's `Core\Heap` and the `Iterable` its three rows declare — plan § *Open now*.
- § 10's `{previous: $e}` constructor options shape and `$e->location` — ADR 0071 § 5.
- ADR 0088's registry-wide qualifier classification for member rows — plan § *Open now*.
- `do`/`while` does not lower — `mwl-ir`'s own module doc.
