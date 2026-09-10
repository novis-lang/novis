# Handoff

## State

**Goal `unowned-sweep`, stage 2.** ADR 0147's mechanism is landed end to end: `Const::NeverWritten`
in the registry, `ConstArg::NeverWritten` in `nvs-types`, `InstKind::ConstUnset` in `nvs-ir` and the
codegen arm that writes `Tag::Unset` over a zero payload. `rule:core-api/omission-is-not-a-written-null`,
`rule:core-api/a-nullable-field-omits-as-the-never-written-marker`,
`rule:core-api/a-written-null-removes`, `rule:core-api/the-marker-never-reaches-a-program` and
`rule:core-api/the-bag-abi-is-unchanged` are all `shipped`.

`Core\Uri::with` is the first member to spend it — `port`, `query` and `fragment` are `?T` and a
written `null` removes; the other three refuse one. `Core\Queue::push`'s `args` took the marker as
its omission fill because `mixed` admits a written `null`; both states still mean *no payload*.

`rule:core-classes/uri-removable-components` stays `designed`: its second level, the
`queryParameter`/`withQueryParameter` pair, is `crates/nvs-stdlib/src/uri.rs`'s known gap 1 now.
Nothing is blocked.

The website rule mirror was stale before this session — `node website/scripts/sync-rules.mjs`
regenerated 30-odd pages that had nothing to do with this work, and it landed as its own commit.

## Next group

**Stage 2: the options bag, continued** — one file set: `crates/nvs-stdlib/src/queue.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-types/src/core_lib.rs`.

- [ ] **`Core\Queue`'s `$args` refuses a `secret`** — `crates/nvs-stdlib/src/queue.rs:64` is gap 2
      and `crates/nvs-stdlib/src/queue.rs:981` the row. `rule:security/secret-qualifier` and
      `rule:security/secret-sinks-refuse`: a durable row read back by another process is a sink by
      every test either applies. `CoreTy::Mixed` carries no qualifier —
      `crates/nvs-stdlib/src/registry.rs:1031` is `classification`, whose leaf list is where a
      spelling would go — so this is a registry spelling before it is a line in the helper.
- [ ] **`Core\Queue`'s `limits` and `grants` are declared** — `crates/nvs-stdlib/src/queue.rs:55` is
      gap 1: each is an *option* whose value is itself a `{…}`, and
      `rule:core-api/shape-parameter` makes a shape only ever a whole parameter, so a
      `CoreOption`'s type is never a bag and `crates/nvs-stdlib/src/registry.rs:771`'s `CoreField`
      is never a `CoreTy::Shape`. `Core\Db::open` no longer waits on it — its settings literal *is*
      a whole parameter — so what is left is whether an option may carry one at all.

## Backlog

- `Core\Uri`'s `queryParameter`/`withQueryParameter` pair — `crates/nvs-stdlib/src/uri.rs` gap 1,
  `rule:core-classes/uri-removable-components`' second level; stage 2 work, unclaimed by any item.
- `array<T>` widens to accept a covariant read — goal `unowned-sweep`'s own standing decision.
- The panic hook `rule:errors/helper-abi` has wanted since M5 — presentation only, never containment.
- `nvs meta --json` spells `Const::NeverWritten` as `(omitted)`; no consumer pins that string yet.
