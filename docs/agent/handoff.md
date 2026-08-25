# Handoff

## State

**`bytes` is a live runtime representation now.** `mwl_runtime::Tag::Bytes` (`value.rs:83`) is a tag of
its own over the *existing* `MwlStr` allocation: one heap shape, two tags. `mwl-runtime`'s module doc
§ *`bytes` is a tag, not a second heap shape* owns that decision, what it spends, and the two readers
that state a rule of their own (`value_to_string` refuses a `bytes`; `value_truthy` drops `string`'s
`"0"` case). `Value::bytes`/`as_bytes`/`buffer_ptr` are `value.rs:235`/`:360`/`:396`;
`mwl_codegen::ty::tag_of` splits `Ty::Str` from `Ty::Bytes`, and `release`/`retain` keep one arm for the
pair. Verified end to end: a `bytes` local round-trips through codegen and valgrind reports no leak.

**`Core\Encoding` exists, with § 7's hex pair only.** `crates/mwl-stdlib/src/encoding.rs` —
`toHex(bytes): string` and `fromHex(string): bytes`, no dependency (its module doc says why hex answers
ADR 0051 § 4 differently from base64/base32). One conformance case,
`tests/conformance/core/encoding-round-trips-hex-and-refuses-anything-else.mwlt`.

**`python tools/loop.py --goal-only` still stops at `collect.mwl`.** The first report has moved off § 7
and onto § 11: `Core\Hash::of`/`Core\Digest` at `collect.mwl:25`. Conformance is 375 of 600,
differential 86 of 150, `every_part_one_spec_member_is_registered` still does not exist.

**`orient.py` did not print three things this session**, all `[context]` gaps in `loop-goal.toml`:
`modules` names only `mwl-runtime/src/identity.rs`, so `value.rs`, `release.rs`, `helpers.rs`,
`string.rs` and `mwl-codegen/src/ty.rs` had to be found by hand; `adrs` is missing
`0009-string-and-bytes.md` §§ 1 and 3, which is the rule every § 7 slice sits inside.

## Next group — the two `collect.mwl` lines still reporting

**Shared file set:** `crates/mwl-stdlib/src/encoding.rs` (`:53` `CLASS`, `:78` `address`, `:98`
`bytes_of`, `:154` the `toHex` helper as the shape to copy), a new `crates/mwl-stdlib/src/hash.rs`,
`crates/mwl-stdlib/src/registry.rs` (`CLASSES` at `:597`, `ENUMS` at `:669`, `CoreEnum` at `:640`,
`CoreTy::Bytes` at `:109`), `crates/mwl-stdlib/src/lib.rs` (`mod` list `:191`, `address_of` `:264`),
`Cargo.toml`'s `[workspace.dependencies]`, and `tests/conformance/core/`. Spec rows:
`docs/spec/01-core-library.md` § 11 (`:705`) and § 7 (`:590`).

- [ ] **`Core\Hash::of`/`hmac`/`equals` and the `Core\Digest` enum** — § 11's remaining half, and what
      `collect.mwl:25` writes. `of(bytes|string, Digest): bytes`, so it is the first member to *return*
      a `bytes` from real work. `Digest` is a `registry::ENUMS` row (`:669`); a `CoreTy::Enum` parameter
      and its Rust-side reader are `math.rs:436` and `math.rs:1231`. `equals` must be constant-time —
      say so in the module doc. Dependency picked under ADR 0051 § 4 (`sha2`/`hmac` or a suite crate),
      with the three things a new dependency owes (playbook § *Adding a `Core` member*).
- [ ] **`Core\Encoding`'s base64 family** — `toBase64`/`fromBase64`/`toBase64Url`/`fromBase64Url`,
      half of what `collect.mwl:26` writes. Unlike hex this one names a dependency; `encoding.rs`'s
      module doc already says why and expects the next author to record the pick there.
- [ ] **`Core\Encoding::encodeText`/`decodeText`/`isValidText` and `Core\Charset`** — the other half of
      `collect.mwl:26`. § 7 fixes the roster as the WHATWG index, which is `encoding_rs`'s, so the enum
      is that crate's list rather than one this repo curates.

## Backlog

- `Core\Encoding`'s `toBase32`/`fromBase32` — needed by TOTP, ADR 0060; off `collect.mwl`'s path.
- `Core\Bytes` × 14 (§ 7's second half, `:606`) — `pack`/`unpack` are ADR 0088 sinks.
- `Ty::Bytes` has no `truthy_convert` row (`mwl-ir` `lower/expr.rs:894` panics); the *tagged* path is
  decided and built, so this is wiring, not a decision.
- `Core\Heap<T>` and § 9's `Iterable` — `docs/spec/01-core-library.md:660-662`; `Heap` also needs
  ADR 0013's `Comparable` reachable from a `Core` class.
- **Constructor property promotion does not create a property**: `public readonly string $name` in a
  `constructor` parameter compiles, and `$obj->name` is then `E0405`. Owner: `mwl-types`' own gaps.
- `every_part_one_spec_member_is_registered` — the loop's definition of done, `loop-goal.md` Stage 4.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
