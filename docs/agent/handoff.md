# Handoff

## State

**§ 12's `Core\Uri` is whole** — the grammar half landed beside the percent-encoding half that was
already there: `Uri::parse`, `Uri::isValid`, the eight readers on the `Uri` instance
(`scheme`/`userInfo`/`host`/`port`/`path`/`query`/`fragment`/`toString`), `$uri->with` and
`$uri->resolve`. `crates/mwl-stdlib/src/uri.rs`'s module doc owns the whole design; the short of
it is that RFC 3986 is a grammar with an external specification so ground-rules.md forced a
dependency, and the pick was **`fluent-uri` over `url`** because WHATWG's reading rewrites its
input and cannot hold a relative reference. `parse` reports and never normalizes; `toString` is the
text that was parsed, byte for byte. § 12's table gained two rows for the readers, the way
`parseQuery`'s row was amended before.

**Every § 12 dependency is now picked**, so `Core\Uri`, `Core\Csv` and `Core\Validate` are all
built and whole and `examples/collect.mwl`'s frontier is still `Core\Out::capture` at
`collect.mwl:47` — which lands with M4S's sink work (ADR 0088 §§ 3, 5), not before. `collect.mwl:43`'s
`?array<T>` panic is still queued behind it, unreported because resolution errors print before
lowering runs.

Conformance is **400** of 600; differential is 86 of 150 and has not moved. The ratchet is at
**27 keys**, and **20 of them are `Core\Arr`** — which is what the next group is.

`Core\Uri` opened one hole, recorded as `uri.rs` gap 1: `$uri->with` replaces a component and
cannot **remove** one, because an omitted option and a written `null` would both arrive as
`Tag::Null`. The fix is an options bag that can tell them apart, not a `""`-means-remove rule.

## Next group — `Core\Arr`, in three slices off one file

**Shared file set for all three:** `crates/mwl-stdlib/src/arr.rs` (`:23` `CLASS`, `:90` the
`contains` row to copy a set-member's shape from, `:254` `unique`, `:402` `address`),
`crates/mwl-stdlib/src/registry.rs:733` (`ENUMS`, where a `SetOn` line goes),
`crates/mwl-stdlib/tests/spec-members-outstanding.txt`'s § 2 block, and `tests/conformance/core/`.
`crates/mwl-runtime/src/identity.rs:137` (`value_identical`) and `:255` (`value_hash`) are already
built and are what the set half needs. `uri.rs` is the freshest model for an options bag whose
options must be told apart from `null`, and `arr.rs`'s own `ORDER` enum is the model for `SetOn`.

- [ ] **ADR 0069's combination members** — `overlay`, `overlayDeep`, `underlay`, `appendAll`, spec
      rows at `docs/spec/01-core-library.md:283-286`, rule at
      [ADR 0069](../adr/0069-array-combination-is-key-type-independent.md). The refusal half is
      already built (`E0467` for `array + array`), so this is the replacement half. **First
      question to settle:** the spec writes `array<T|U>` and `registry`'s own
      `a_union_is_only_ever_a_parameter` says a union cannot be a return type — pick the spelling
      the registry can state (`array<mixed>` is the safe one) and record it in `arr.rs`'s module
      doc, per loop-goal.md § *Standing decisions*. All four take a `CoreTy::Variadic` tail, which
      is one ABI argument whatever the call writes.
- [ ] **The set half** — `diff` and `intersect`, spec rows at `:287-288`. Needs `SetOn { Values,
      Keys, Both }` added to `registry::ENUMS` beside the member (`registry.rs:724`'s doc says why
      it is deliberately absent today) and an `{on?: SetOn, by?: callable, comparator?: callable}`
      bag. `mwl-stdlib`'s own gap 3 (`lib.rs:165`) says everything else these need already exists.
- [ ] **The structure members** — `append`, `prepend`, `slice`, spec rows at `:239`, `:242-243`.
      `append`/`prepend` are variadic; `slice` takes `?int $length = null` and a
      `{preserveKeys?: bool}` bag, both shapes the registry already states
      (`Core\Str::slice` is the `?int` model at `str.rs:119`).

## Backlog

- `Core\Out::capture` — the last § 12 key; blocked on M4S's sink work (ADR 0088 §§ 3, 5).
- The `?array<T>` index hole — `mwl-ir` panics at `crates/mwl-ir/src/lower/expr.rs:2952`; playbook
  § *Writing a test case* has the three spellings that work around it.
- § 4's `Date`, `TimeOfDay` and `Core\Month` — `mwl_stdlib::time` gap 1.
- § 5's `compile`/`replaceWith`, which need `Pattern` — `mwl_stdlib::regex` gap 1.
- § 10's `{previous: $e}` options shape and `$e->location` — ADR 0071 § 5 needs them.
- § 11's `Random::bytes` and `Hash::stream` — the `bytes` tag they waited on exists now.

## Gap in `orient.py`'s pack

`loop-goal.toml`'s `[context] rules` did not include ground-rules.md's **"anything with an external
specification is a dependency, and if no crate exists the feature is not built"** (README.md
§ *Decisions taken at project start*). That rule decided this whole session's first question, and
it was only found because `uri.rs`'s own module doc happened to quote it. Every remaining `Core`
slice that might bind a crate needs it printed — add its selector.
