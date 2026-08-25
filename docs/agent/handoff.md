# Handoff

## State

**Spec § 11's second table is whole**: `crates/mwl-stdlib/src/uuid.rs` registers `Uuid::v4`, `v7`,
`parse` and `isValid` plus a `$uuid->toString()` row amended into the spec table, over the new
`uuid` dependency. That module's own docs own the pick against ADR 0051 § 4, why entropy still comes
from `rand` and the clock from `jiff`, the two-`uint`-slot layout, the canonical-form-only parse rule
and the three gaps left — none of it is restated here or in the plan.

`python tools/verify.py` green; `mwl test tests/conformance/` is 367 cases, all passing. The two new
ones are `tests/conformance/core/uuid-draws-a-canonical-v4-and-a-time-ordered-v7.mwlt` and
`uuid-parses-only-the-canonical-form.mwlt`. `cargo deny check` is green and
`THIRD-PARTY-LICENSES.txt` is regenerated — it gained thirteen components, twelve of which are
`uuid`'s `cfg(target_arch = "wasm32")` leg that no MWL build links; `tools/gen-attribution.py` walks
every platform on purpose and the file's own header says so.
`tools/leak-check.sh` under WSL is clean over `.agent-tmp/uuid-refcounts.mwl`, which exercises the
built instance, its slots and the `string` each `toString` hands back.

**`Tag::Bytes` is still the one blocker on this fixture**, unchanged: `mwl_runtime::Tag` has no
`Bytes` variant, so `Random::bytes`, `Core\Encoding`, `Core\Bytes` and `Hash::of`/`hmac`/`equals`
cannot construct a value. The next group routes around it a third time.

## Next group — `Core\Uri`, spec § 12's first table

**Shared file set:** `crates/mwl-stdlib/src/uri.rs` (new), `crates/mwl-stdlib/src/lib.rs`
(`pub mod uuid;` at `lib.rs:200`, the `.or_else` at `lib.rs:246`), `crates/mwl-stdlib/src/registry.rs`
(`CLASSES` at `registry.rs:604`), and `tests/conformance/core/`. The spec table is
`docs/spec/01-core-library.md:752-761`, with its prose at `:763`. This is the file set `Core\Uuid`
just used, and `uuid.rs` is the nearest model for every shape: `CLASS` at `uuid.rs:115` for a class
that is both static members and a `Core`-owned instance, `built` at `uuid.rs:181`, the receiver read
at `uuid.rs:248`.

- [ ] **`Uri::encodeComponent`/`decodeComponent`/`encodeFormValue`/`decodeFormValue`, and
      `isValid`.** No dependency at all — percent-encoding is byte arithmetic, and the four members
      are two rules (`rawurlencode` vs `urlencode`'s `+`-for-space) written twice. Land these first
      so the class exists before the parse decision lands on it.
- [ ] **`Uri::parse` and the `Uri` instance**, plus `$uri->with` and `$uri->resolve`. Spec § 12
      calls `parse` an ADR 0057 intrinsic. The dependency is yours under ADR 0051 § 4 and the pick
      has a real fork in it — RFC 3986 or the WHATWG URL algorithm, which disagree about what a URL
      *is*; whichever you take, say so in that module's docs and note that `Core\Http::allowUrl`
      (§ 16, ADR 0058) is the SSRF gate and this class never fetches.
- [ ] **`Uri::parseQuery` and `buildQuery`**, PHP's bracket convention in full — a pre-authorized
      standing decision in `docs/agent/loop-goal.md`, including that the spec row's return type is
      amended off `array<string>` to whatever `mixed` lets the checker state, and that saying so
      here settles what `Core\Request::query` owes at M8. `examples/collect.mwl:37` calls it.

## Backlog

- `Tag::Bytes` in `mwl-runtime` — the third group in a row has routed around it; it unblocks
  `Random::bytes`, `Core\Encoding`, `Core\Bytes`, `Hash::*` and ADR 0009 § 3's `as` rows
  (`crates/mwl-stdlib/src/random.rs` gap 1, `mwl-ir` gap 20).
- `Core\Csv::parse`/`format` — spec § 12's third table, one dependency pick,
  `examples/collect.mwl:41`.
- `Core\ObjectSet`/`ObjectMap` — spec § 7, additionally blocked on `new Core\X<T>()` parsing.
- `==` over two `Core`-owned instances is object identity, so two equal `Uuid`s are not `==`
  (`crates/mwl-stdlib/src/uuid.rs` gap 2). A `compareTo` row on `Core\Uuid` would answer it the way
  `Core\Time\Instant` does; spec § 11 does not write one yet.
- `Core\Time` still owes `Date`/`TimeOfDay`/`Core\Month` (`crates/mwl-stdlib/src/time.rs` gap 1).
- `docs/spec/02-php-migration.md` is 31% classified, one pass per PHP domain
  (`python tools/check-migration.py`).

`orient.py` printed everything this session needed. One thing it could not: `Core\Str`'s member list
and `mwl-codegen`'s missing `BinOp` row for `Ty::Str` both had to be grepped while writing a `.mwlt`
case — a `[context] modules` selector for `mwl-stdlib/src/str.rs` would have covered the first.
