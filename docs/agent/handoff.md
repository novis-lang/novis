# Handoff

## State

**Spec § 1 is whole.** `Core\Str::normalize` is registered, implemented and pinned by a conformance
case; `Core\NormalForm { Nfc, Nfd, Nfkc, Nfkd }` is the enum beside it in
`crates/mwl-stdlib/src/str.rs:49`, listed in `registry::ENUMS`. Its integers are ABI — the helper
indexes on them — and `the_normal_form_enum_and_its_rust_twin_are_one_roster` is what says so. The
member's own doc comment at `str.rs:2019` owns the two-axes reading (composed/decomposed ×
canonical/compatibility) and why only the canonical pair round-trips; the ASCII fast path is stated
there too.

**`unicode-normalization` is now a named dependency** rather than `caseless`'s transitive one:
`[workspace.dependencies]` line with its reasoning, plus `crates/mwl-stdlib/Cargo.toml`.
`cargo deny check` green, `THIRD-PARTY-LICENSES.txt` regenerated and unchanged (it already carried
the crate), no licence identifier new to `deny.toml`.

**The loop's gate is the ratchet in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`**, now at
**35 keys** for §§ 2-12; when it is empty, Part I is registered whole. Conformance is **394** of 600;
differential is 86 of 150 and has not moved.

**`examples/collect.mwl`'s frontier is unchanged** — `Core\Uri::parseQuery` at `collect.mwl:36`, then
`Core\Csv::parse`, `Core\Validate::isEmail`, `Core\Out::capture`.

## Next group — `Uri`'s query pair, then `Arr`'s set half

**Shared file set for `[1]`/`[2]`:** `crates/mwl-stdlib/src/uri.rs` (`:122` `CLASS`'s rows, `:126`-`:147`
the four percent-encoding members already built, `:161` `address()`, `:413` the test module),
`crates/mwl-stdlib/tests/spec-members-outstanding.txt`, `tests/conformance/core/`, and
`examples/collect.mwl` + its frozen output. Take both — they are one file and one convention, and the
round trip is the case that pins either of them honestly. `[3]` is a different file; leave it.

- [ ] **`Core\Uri::parseQuery`** (§ 12; `loop-goal.md` § *Standing decisions* amends the spec row —
      PHP's bracket convention **in full**: `a[]=1&a[]=2` is a list, `a[b]=c` is a map, nesting to
      arbitrary depth, so the return type is not `array<string>`). Pick the spelling the type system
      can state and **write it into the spec table** in `docs/spec/01-core-library.md` § 12; say there
      that this also settles what `Core\Request::query` answers at M8. `array<mixed>` is the safe
      default — `mwl_ir::Ty::Tagged` is settled, so it is statable now.
- [ ] **`Core\Uri::buildQuery`** (§ 12) — the inverse, same file, same convention. The conformance
      case is `buildQuery(parseQuery($q)) == $q` over a nested subject plus the flat rows.
- [ ] **`Core\Arr::diff` / `intersect`** (§ 2) — `crates/mwl-stdlib/src/arr.rs`; both need the one
      strict-identity comparison `mwl_runtime::identity` already defines (`loop-goal.md`
      § *Standing decisions*). A different file set: do not pull it into the group above.

## Backlog

- `Core\Csv::parse` needs a dependency picked under [ADR 0051 § 4](../adr/0051-standard-library-tiers.md).
- `Core\Uri::parse`/`isValid` still owe an RFC 3986 dependency — see the plan's *Open now*.
- § 9 owes `Core\Heap` and the `Iterable` its three rows declare (`mwl_stdlib`'s module doc).
- § 10 owes the constructor's `{previous: $e}` options shape and `$e->location` (ADR 0071 § 5).
- § 11 owes `Random::bytes` and `Hash::stream`; the runtime `bytes` tag they wanted exists now.
- ADR 0088's registry-wide qualifier classification is unbuilt and lands with M4S.
