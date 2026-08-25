# Handoff

## State

**Spec § 12's first table now exists as a class, percent-encoding half only**:
`crates/mwl-stdlib/src/uri.rs` registers `Uri::encodeComponent`, `decodeComponent`,
`encodeFormValue` and `decodeFormValue` over no dependency at all. That module's own docs own the
two PHP-exact byte sets (they differ on a space *and* on `~`, deliberately), why a malformed escape
decodes to itself, why a decoded non-UTF-8 octet throws, and why `isValid` is **not** here.

**`Uri::isValid` was deliberately left out of its own slice.** It answers exactly the question
`parse` throws on, and `ground-rules.md`'s "an external specification is a dependency rather than a
hand-written parser" makes RFC 3986 a dependency — so hand-writing a validator now and binding a
crate for `parse` next session is the drift `crate::uuid`'s single `read` exists to avoid. It moved
into the group below, beside the pick that decides it.

`python tools/verify.py` green; `mwl test tests/conformance/` is 369 cases. The two new ones are
`tests/conformance/core/uri-encodes-a-component-and-a-form-value-differently.mwlt` and
`uri-decodes-both-spellings-of-a-space.mwlt`. `tools/leak-check.sh` under WSL is clean over
`.agent-tmp/uri-refcounts.mwl`. No new dependency, so `cargo deny` and the attribution file are
untouched.

**`Tag::Bytes` is still the one blocker on this fixture**, unchanged: `Random::bytes`,
`Core\Encoding`, `Core\Bytes` and `Hash::of`/`hmac`/`equals` cannot construct a value, and it is
also why both decoders here answer `string` rather than the honest `bytes` (`uri.rs` gap 2).

## Next group — the rest of `Core\Uri`, spec § 12's first table

**Shared file set:** `crates/mwl-stdlib/src/uri.rs` (`CLASS` at `uri.rs:122`, `instance: &[]` at
`uri.rs:154`, `address` at `uri.rs:161`, `text_of` at `uri.rs:286`, `produced` at `uri.rs:303`,
`decode` at `uri.rs:244`), `Cargo.toml:133` (the `uuid` line is the model for a new
`[workspace.dependencies]` entry) plus `crates/mwl-stdlib/Cargo.toml`, and
`tests/conformance/core/`. `lib.rs:200`/`lib.rs:247` and `registry.rs:605` are already wired, so
adding members here touches neither. `uuid.rs` is the nearest model for an instance:
`CLASS` at `uuid.rs:115`, `built` at `uuid.rs:181`, the receiver read at `uuid.rs:248`.

- [ ] **Pick the RFC 3986 crate, then land `Uri::parse` + `Uri::isValid` and the `Uri` instance.**
      Spec § 12's first two rows (`docs/spec/01-core-library.md:754-755`), the pick against
      ADR 0051 § 4, recorded in `uri.rs`'s own module doc. Both members share one `read`, per
      `uri.rs` gap 1. The instance needs slots — a `string` per component is the shape the
      `Tag::Bytes` blocker leaves available. A new dependency owes three things (AGENTS.md): the
      `[workspace.dependencies]` line saying why that crate, `cargo deny check`, and
      `python tools/gen-attribution.py`. Candidates worth one probe each: `fluent-uri` (RFC 3986,
      holds a relative reference, has reference resolution) against `url` (WHATWG, normalizes, and
      cannot hold the relative reference `$uri->resolve` takes).
- [ ] **`$uri->with` and `$uri->resolve`.** Spec § 12 rows 7-8 (`:760-761`). `with` is ADR 0063 R2's
      options bag over the six components — `CoreTy::Options` at `path.rs:76` is the model; `resolve`
      is RFC 3986 § 5, which is the chosen crate's job, not a hand-written merge.
- [ ] **`Uri::parseQuery` and `buildQuery`.** Spec § 12 rows 5-6 (`:758-759`) and the prose at
      `:767`; PHP's bracket convention in full is a **pre-authorized** standing decision, so the
      return type is `array<mixed>` and the spec row already says so. Both members consume
      `uri.rs:244`'s `decode`/`encode` rather than re-implementing them.

## Backlog

- `Core\Validate`, `Core\Csv`, `Core\Out` — the rest of spec § 12's tables (`01-core-library.md:775+`).
- `Tag::Bytes` in `mwl_runtime` unblocks `Random::bytes`, `Core\Encoding`, `Core\Bytes`, `Hash::*`
  and turns both `Uri` decoders' return type honest — `crates/mwl-stdlib/src/random.rs` gap 1.
- `examples/collect.mwl` is the fixture this whole run is aimed at — plan `Open now`, the § 7/8/9/11/12
  paragraph.
- `ObjectSet`/`ObjectMap` need `new Core\X<T>()` to parse — same plan paragraph.
- `Core\Uuid` has no `bytes` round trip and `v7` no intra-millisecond counter — `uuid.rs` gaps 1 and 3.
- ADR 0090 § 3's non-scalar row means `==` over two `Core`-owned instances is object identity;
  every such class works around it — `uuid.rs` gap 2.
