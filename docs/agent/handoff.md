# Handoff

## State

**Spec § 7's `Core\Bytes` is ten members of twelve.** `crates/mwl-stdlib/src/bytes.rs` registers
`length`, `at`, `slice`, `indexOf`, `compare`, `contains`, `startsWith`, `endsWith`, `fill` and
`repeat`. That module's own doc owns the four signature calls the spec's prose does not state —
`at` answers a `uint`, `indexOf` has no `caseInsensitive`, `compare` answers an ordering `int`, and
`fill` is length-first — and `docs/spec/01-core-library.md` § 7 is amended to point at it.

**`join` is blocked, not unreached**, and it is the same hole twice: `registry::Const` has no
`bytes` variant to state `bytes $separator = ""` with, and ADR 0009 § 3's `string as bytes` does not
lower either — both want one constant buffer materialized at a call site. `pack`/`unpack` are
unwritten for the ordinary reason. The plan's `Open now` § 7 clause says the same.

Conformance is **382** of 600 (two new `bytes-*` cases); differential is 86 of 150 and has not
moved. `examples/collect.mwl`'s frontier is unchanged — `Core\Uri::parseQuery` at `collect.mwl:36`,
then `Core\Csv::parse`, `Core\Validate::isEmail`, `Core\Out::capture`.

## Next group — ADR 0009 § 3's conversion rows, and the `Core\Bytes::join` they unblock

**Shared file set:** `crates/mwl-ir/src/lower/expr.rs` (`:795`, the panic arm that names the missing
rows), `crates/mwl-ir/src/lower/call.rs` (`:405`, `ConstArg::Str` → `InstKind::ConstStr`),
`crates/mwl-types/src/defaults.rs` (`:65` `enum ConstArg`), `crates/mwl-types/src/core_lib.rs`
(`:197`, the `Const` → `ConstArg` map), `crates/mwl-types/src/lower.rs` (`:285`, the type a default
gets), `crates/mwl-stdlib/src/registry.rs` (`:332` `enum Const`, `CLASSES` `:614`),
`crates/mwl-stdlib/src/bytes.rs`, and `tests/conformance/core/`.

- [ ] **`string as bytes` and `bytes as string` lower** (`0009-string-and-bytes.md` § 3). Total and
      free one way — the same `MwlStr` allocation retagged, no copy — and checked the other, throwing
      on a buffer that is not well-formed UTF-8, never replacing. `mwl_runtime::Tag` already has its
      `Bytes` row over that allocation, so this is a lowering arm and a codegen row, not a runtime
      change. Today it is a `panic!` with a backtrace rather than a diagnostic, which is what makes
      it worth doing first: it is the shape every future `bytes` test case reaches for.
- [ ] **`Const::Bytes` and `Core\Bytes::join`** (`01-core-library.md` § 7, `:608`). One variant each
      in `Const` and `ConstArg`, the map between them, the `InstKind` a call site materializes, then
      `join(array<bytes> $parts, bytes $separator = ""): bytes` — mirror
      `crates/mwl-stdlib/src/str.rs:588`'s `mwl_core_str_join` body, which is where the array walk
      and its `unsafe` justification already are.
- [ ] **`Core\Bytes::pack`/`unpack`** (`01-core-library.md` § 7, `:608`). `pack(string $format,
      mixed ...$values): bytes` and `unpack(bytes $b, string $format): array<mixed>`. The format
      string is an ADR 0057 intrinsic **and a sink on both members** — one of R11's four grammars —
      so this is also the first `Core\Bytes` row that ADR 0088's qualifier classification touches.
      `registry::CoreTy::Variadic` can already state the tail.

## Backlog

- `Core\Uri::parseQuery` with PHP's full bracket convention — the gate's own frontier at
  `examples/collect.mwl:36`; `docs/agent/loop-goal.md` § *Standing decisions* settles its shape.
- `Core\Csv`, `Core\Validate::isEmail`, `Core\Out::capture` — § 12, the rest of that fixture.
- `every_part_one_spec_member_is_registered` does not exist yet — the loop's own definition of done
  (`docs/implementation-plan.md`, `Open now`).
- § 1's eleven text-shaping rows, § 2's `Arr::diff`/`intersect` and ADR 0069's combination members.
- ADR 0088's registry-wide qualifier classification, which `Core\Str::format` already needs.
- Differential is 86 of 150 and has not moved in two runs (`docs/agent/loop-goal.md` § *Stage 4*).

## Gaps in `orient.py`'s `[context]` manifest

Each cost a fetch this session or the last; naming the field is the whole fix.

- **`modules` is missing `granularity.rs`** — `Core\Str`'s unit lives there, and every `Core\Bytes`
  member is defined by *not* using it, so a session mirroring `str.rs` reads it to find that out.
- **`modules` is missing `hash.rs`** (still true) — `Core\Digest` is the only worked example of a
  `Core`-owned enum, so any slice adding one pays to rediscover it.
- The next group works in `mwl-ir` and `mwl-types`; neither `src/lower/*` nor `defaults.rs` is in
  `modules`, and `adrs` should gain `0007` § 2 (the conversion rows) beside its § 3.
