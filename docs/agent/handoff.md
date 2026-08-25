# Handoff

## State

**§ 12's `Core\Validate` is whole** — all six members (`isEmail`, `isDomain`, `isIp`, `isMac`,
`isAscii`, `isPrintable`) in a new `crates/mwl-stdlib/src/validate.rs`, pinned by one conformance
case, `tests/conformance/core/validate-members.mwlt`. That module's own doc owns the whole design:
why it binds **no dependency** (a validator draws a *line*, and where the line falls is a policy the
caller must be able to read — the opposite of `csv.rs`'s parser argument), exactly where each line
falls, and every divergence from `filter_var`/`ctype_*` it takes deliberately. Spec § 12 states
these as prose rather than table rows, so nothing was struck from the ratchet.

**The registry can state a literal union now.** `registry::CoreTy::IntLiteral(i64)` is new, and an
option's type may be a `CoreTy::Union` of them defaulting to `Const::Null` — the guard test is
`a_union_option_is_a_closed_set_of_literals` (`registry.rs`), and `a_literal_type_only_appears_inside_a_union`
is its placement rule. `Core\Validate::isIp($s, {version: 5})` is now `E0401: expected 4|6, found
int` end to end; `mwl-types`' `core_lib` test `a_literal_union_option_interns_to_its_two_literals`
pins the interned shape.

**Conformance is 398** of 600; differential is 86 of 150 and has not moved. The ratchet is still at
**31 keys**. `examples/collect.mwl`'s frontier has moved to **`Core\Out::capture` at
`collect.mwl:47`** — which is not a cheap next slice: spec § 12 line 818 makes its return the
**carrier of the sink in force** (ADR 0088 §§ 3, 5), so it lands with M4S's sink work, not before.
`collect.mwl:43`'s `?array<T>` panic is still waiting behind it, unreported because resolution
errors print before lowering runs.

## Next group — `Core\Uri`'s remaining half, then the `?array<T>` hole

**Shared file set for `[1]`/`[2]`:** `crates/mwl-stdlib/src/uri.rs` (`:179` `NAME`, `:183` `CLASS`,
`:229` `instance:`, `:230` `slots:`, `:236` `address`), `crates/mwl-stdlib/src/registry.rs:669`
(`crate::uri::CLASS` is already in `CLASSES`), `crates/mwl-stdlib/tests/spec-members-outstanding.txt`
(strike `§12 Uri::parse`, `§12 Uri::isValid`, then `§12 $uri->with`, `§12 $uri->resolve`), and
`tests/conformance/core/`. `csv.rs` and `validate.rs` are the two freshest models — the first for a
dependency pick, the second for arguing none.

- [ ] **`Core\Uri::parse` and `Uri::isValid`** — spec rows at `docs/spec/01-core-library.md:767-768`,
      prose at `:776`. This is **the last § 12 dependency still to pick**: RFC 3986, and the
      WHATWG-URL crates are a different specification rather than a stricter one, so state which
      question the member answers before choosing. `parse` returns a `CoreTy::Instance` and so needs
      `CoreClass::slots` filled for the first time in this module — `objmap.rs` is the shape.
      `Uri::parse` is an ADR 0057 intrinsic; `isValid` is neutral and must not launder (§ 12's
      `Core\Http::allowUrl` at § 16 is the SSRF question, not this one).
- [ ] **`$uri->with({scheme?, host?, port?, path?, query?, fragment?})` and `$uri->resolve`** — spec
      rows at `docs/spec/01-core-library.md:773-774`, same file, reached through `instance:` at
      `uri.rs:229`. An instance member's receiver is argument slot 0 and is not in `params`, so
      `with`'s `args:` is 1 + the bag's six.
- [ ] **The `?array<T>` index hole** (different file set — `crates/mwl-ir/src/lower/expr.rs:2952`):
      a narrowed nullable array loses its element type, so `$head["name"]` panics at
      `collect.mwl:43` even inside a `!= null` guard. The playbook's *Writing a test case* bullet has
      the three spellings that do lower.

## Backlog

- § 2 owes 19 of the ratchet's 31 keys — `Arr::diff`/`intersect` and ADR 0069's combination members
  ([docs/adr/0069](../adr/0069-array-combination-is-key-type-independent.md)).
- `Core\Out::capture` waits on ADR 0088's sink carrier — `docs/spec/01-core-library.md:818`.
- § 9 owes `Core\Heap` and the `Iterable` its three rows declare (`mwl_stdlib::objmap` module doc).
- § 5 owes `compile`/`replaceWith`, which need `Pattern` (`mwl_stdlib::regex` gap 1).
- § 10 owes the constructor's `{previous: $e}` shape and `$e->location` ([ADR 0071](../adr/0071-derived-codecs.md) § 5).
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` module doc).

## Notes for the driver

`orient.py`'s pack was complete for this goal — nothing outside it was needed. Both slices landed in
**one commit** rather than one each: they share every file (`validate.rs`, `registry.rs`, `lib.rs`,
one conformance case), so a per-slice staging would have had nothing to separate.
