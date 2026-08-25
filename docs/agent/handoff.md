# Handoff

## State

**`Core\Hash` and `Core\Digest` are built — spec § 11's one-shot half.**
`crates/mwl-stdlib/src/hash.rs` registers `of(bytes|string, Digest): bytes`,
`hmac(bytes|string, bytes, StrongDigest): bytes` and `equals(bytes, bytes): bool`. That module's own
doc owns the dependency picks (RustCrypto `sha2`/`sha1`/`md-5`/`hmac`, plus `crc32fast` and `subtle`,
under ADR 0051 § 4), why `Crc32` answers four big-endian octets, and why `equals` is constant-time.
`stream(Digest): Hash\Stream` is unbuilt and is that file's gap 1; the `secret` qualifier on `hmac`'s
key is gap 2 and belongs to ADR 0088's registry-wide item, not to this class.

**`registry::CoreTy::EnumCase` is new, and it is what states a closed subset of an enum.** § 11's
`StrongDigest` is `CoreTy::Union` over three `CoreTy::EnumCase` rows (`hash.rs`, `STRONG`), not a
second enum — so one `Core\Digest::Sha256` value satisfies both `of` and `hmac`, and
`Hash::hmac($m, $k, Digest::Md5)` is `E0401` naming the three cases the position takes. That variant's
own doc comment in `registry.rs:240` owns the reasoning; `mwl_types::core_lib`'s `lower` interns it
through `TypeInterner::enum_case`, exactly as a source-written case type. It works because
`mwl_types::expr::literals::placed_literal` already looks inside a union.

**`examples/collect.mwl`'s frontier moved off § 11 and onto § 7.** Its first report is now
`Core\Encoding::toBase64` at `collect.mwl:26`. Conformance is 377 of 600; differential is 86 of 150 and
has not moved; `every_part_one_spec_member_is_registered` still does not exist.

**Two loop sessions held this tree at once, and the cause is known.** The previous iteration's driver
died to the `UnicodeEncodeError` that `tools/loop.py`'s uncommitted fix addresses — but it died
*without killing its child*, so that orphan was still working § 11 when the restarted driver launched a
second session onto the same handoff. Both built `Core\Hash` independently and converged; `hash.rs` and
one conformance case were replaced mid-session, the duplicate refusal case was dropped, and the commit
below therefore contains both authors' work. The `loop.py` fix stops the crash, **not** the orphan:
until the driver reaps its child on its own death, a crash still leaves one running. Check for a stray
session before trusting `git status` to describe your own work.

## Next group — the two `collect.mwl` lines still reporting, both § 7

**Shared file set:** `crates/mwl-stdlib/src/encoding.rs` (`:53` `CLASS`, `:78` `address`, `:98`
`bytes_of`, `:108` `text_of`, `:138` `SHOWN_CHARS`/`shown` for a bounded throw message, `:163`
`toHex` as the shape to copy), `crates/mwl-stdlib/src/registry.rs` (`CLASSES` `:614`, `ENUMS` `:694`,
`CoreEnum` `:665`, `CoreTy::EnumCase` `:240`), `Cargo.toml`'s `[workspace.dependencies]`, and
`tests/conformance/core/`. Spec rows: `docs/spec/01-core-library.md` § 7 (`:590`).

- [ ] **`Core\Encoding`'s base64 family** — `toBase64`/`fromBase64` and `toBase64Url`/`fromBase64Url`
      (spec § 7's table, `01-core-library.md:600`). Four rows, one dependency picked under ADR 0051
      § 4 — the alphabet/padding/URL-safe detail is exactly what hex's module comment says is *not*
      hex's answer. Decoding throws rather than substituting (ADR 0009 § 3), and `shown()` bounds what
      the message quotes.
- [ ] **`Core\Encoding::toBase32`/`fromBase32`** — the same shape once base64 lands, and the row TOTP
      needs (ADR 0060). Same file, same dependency question; likely the same crate.
- [ ] **`Core\Encoding::encodeText`/`decodeText`/`isValidText` and the `Core\Charset` enum** — spec
      § 7 names the WHATWG Encoding Standard's index as the roster and `encoding_rs` as what
      implements it, so the enum is a `registry::ENUMS` row whose cases come from that document.
      `Core\Hash`'s `DIGEST` is the worked example of an enum declared beside the member that takes
      it; `hash.rs`'s `digest_kind` is the Rust-side reader to copy.

## Backlog

- `Core\Hash::stream` needs a `Core` instance holding *mutable* state — a `CoreClass::slots` question
  no other instance has posed (`hash.rs` gap 1).
- `Core\Random::bytes` is now unblocked — `Tag::Bytes` exists (`mwl-runtime`'s module doc);
  `random.rs`'s own gap 1 still says it is blocked and is stale.
- `Core\Bytes` in full — spec § 7's second half, `Core\Str`'s member names over octets.
- `Core\Uri::parse`/`isValid` still owe an RFC 3986 dependency; `Csv` and `Validate` owe theirs
  (`docs/implementation-plan.md`, *Open now*).
- ADR 0088's registry-wide qualifier classification: no `CoreTy` carries `secret`/`tainted` yet, so
  `hmac`'s key and `Str::format`'s template are both unclassified.
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).

## Gaps in `orient.py`'s pack this session

Two `[context]` fields in `docs/agent/loop-goal.toml` were missing a selector:

- `modules` — `crates/mwl-types/src/expr/members.rs` and `expr/literals.rs`. Whether an enum-case
  *expression* can place against a union of case types is the question every `CoreTy::EnumCase` row
  depends on, and it is answered only there.
- `adrs` — `0047-literal-and-enum-case-types.md` § 3. The ground-rules one-liner names the rule but
  not what the narrowed type is, which is what the registry row has to state.
