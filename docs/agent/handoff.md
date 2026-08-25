# Handoff

## State

**`Core\Encoding`'s base64 and base32 members are built — spec § 7's codec half is done except the
`Charset` trio.** `crates/mwl-stdlib/src/encoding.rs` now registers `toBase64`/`fromBase64`,
`toBase64Url`/`fromBase64Url` and `toBase32`/`fromBase32` beside the hex pair. That module's own doc
owns every rule they differ on — which alphabet, which padding, and the one place a decoder is lenient
(base32 folds case and optional padding, because it is a single pair with nothing to disambiguate
against, and neither variance can change the octets). It also owns the two dependency picks under ADR
0051 § 4: `base64` (already in the lock file under `wasmtime-internal-cache`, so it adds no crate) and
`data-encoding` (new, MIT, no dependencies of its own — picked because it states padding, trailing
bits and symbol translation as separate checked rules rather than one alphabet constant).
`cargo deny check` is green and `THIRD-PARTY-LICENSES.txt` is regenerated.

**`examples/collect.mwl:26` now reports `Core\Encoding::encodeText`, not `toBase64`.** That single
member is all the gate fixture still needs from § 7, and it is the next group's first slice.
Conformance is **379** of 600; differential is 86 of 150 and has not moved.

**`D:` filled to 24 KB free mid-session and the first `verify.py` failed as a wall of `LNK1108`.**
`cargo clean` freed 34 GB; the rebuild is under four minutes. This is the playbook trap — check
`Get-PSDrive D` before reading a linker error as a code failure.

## Next group — § 7's last trio, then `Core\Bytes`

**Shared file set:** `crates/mwl-stdlib/src/encoding.rs` (`:120` `CLASS` rows, `:187` `address`,
`:215` `bytes_of`, `:225` `text_of`, `:259` `shown`, `:275`/`:301` the two `why_not_*` message
renderers to copy, `:495` the unit tests that pin each engine's config),
`crates/mwl-stdlib/src/registry.rs` (`CLASSES` `:614`, `CONSTRUCTORS` `:647`, `ENUMS` `:694`,
`CoreEnum` `:665`, `CoreTy::EnumCase` `:240`), `crates/mwl-stdlib/src/lib.rs` (`:191` the `mod` list,
`:265` the `address` chain), `Cargo.toml`'s `[workspace.dependencies]`, and `tests/conformance/core/`.
Spec rows: `docs/spec/01-core-library.md` § 7 (`:590`), whose `Charset` paragraph and `Core\Bytes`
paragraph are at `:617` and `:608`.

- [ ] **`Core\Charset` and the text trio** — `encodeText(string, Charset): bytes`,
      `decodeText(bytes, Charset): string`, `isValidText(bytes, Charset): bool`, plus the enum
      (`01-core-library.md:617`). The roster is the **WHATWG Encoding Standard's index**, not a
      curated list, which is why the spec names `encoding_rs` — that crate is already in `Cargo.lock`
      (`:703`) and in the local registry cache, so it resolves offline. `Charset` is an ordinary
      `CoreEnum` in `ENUMS`, not a `CoreTy::EnumCase` union: every case is legal in every position
      here. R4 and ADR 0009 § 3 mean a conversion that cannot be exact **throws** — there is no
      `//IGNORE` or `//TRANSLIT`, so `decodeText` refuses a malformed sequence rather than emitting
      U+FFFD, which is *not* `encoding_rs`'s default `decode` and needs `decode_without_bom_handling
      _and_without_replacement`. This closes `collect.mwl:26`.
- [ ] **`Core\Bytes`, the `Core\Str` mirror** — `length`, `at`, `slice`, `indexOf`, `compare`,
      `contains`, `startsWith`, `endsWith`, `join`, `fill`, `repeat` (`01-core-library.md:608`). A new
      `mod bytes;` — **`mod`, not `pub mod`**, so its `CLASS`/`NAME` are `pub(crate)`. Every offset is
      a byte offset (ADR 0009 § 1: there is no other unit), which is the whole reason these are not
      `Core\Str` rows with a wider parameter. `str.rs` is the shape to copy member for member; ADR
      0063 R6 fixes each spelling.
- [ ] **`Core\Bytes::pack`/`unpack`** — `pack(string $format, mixed ...$values)` and
      `unpack(bytes, string): array<mixed>`. The format string is an ADR 0057 intrinsic **and** an
      ADR 0088 sink on both members, which is the part that is not a straight port; take it only after
      the mirror members land, and check whether ADR 0088's registry-wide qualifier item has to move
      first.

## Backlog

- `Core\Uri::parse`/`isValid` and the `Uri` instance — needs an RFC 3986 dependency picked under ADR
  0051 § 4 (`docs/implementation-plan.md`, `Open now`).
- `Core\Csv` — the other § 12 member still owing a dependency pick (same field).
- `every_part_one_spec_member_is_registered` does not exist; it is the loop's own definition of done
  (`docs/agent/loop-goal.md` § acceptance).
- Differential corpus is 86 of 150 and has not moved for several sessions (`Open now`).
- ADR 0088's registry-wide qualifier classification — no `Core` member row carries one yet.
- `Core\Hash::stream` and `Core\Random::bytes` — § 11's remaining rows (`hash.rs` gap 1).
