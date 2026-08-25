# Handoff

## State

**§ 12's `Core\Csv` is whole.** `parse` and `format` are registered, implemented and pinned by one
round-trip conformance case; both live in `crates/mwl-stdlib/src/csv.rs`, whose own module doc owns
the dependency pick, the dialect defaults, the ragged-record rule and the writer's one quoting rule.
The reader binds **`csv-core`** — pure Rust, so ADR 0051 § 4's second question does not arise — and
the writer binds nothing; that asymmetry and its reasoning are the module doc's first section. The
dependency's three obligations are done: the `[workspace.dependencies]` comment, `cargo deny check`
(green) and `python tools/gen-attribution.py`.

**`spec_registry_coverage.rs` resolved a Member cell too loosely and now does not.** Its § 12 heading
names four classes, two of which write a `parse` row, so registering `Core\Csv::parse` silently made
`§12 Uri::parse`'s ratchet line "stale". A cell that writes its own qualifier is now resolved against
that class alone (`scoped`, `tests/spec_registry_coverage.rs:113`); a bare name keeps the old
section-wide reading. `Uri::parse` is back on the list, where it belongs.

**The ratchet is at 31 keys** for §§ 2-12. Conformance is **397** of 600; differential is 86 of 150 and
has not moved. Valgrind is clean over the new nested-array edges
(`tools/leak-check.sh .agent-tmp/csv-leak.mwl`, 0 failures).

**`examples/collect.mwl`'s frontier is now `Core\Validate::isEmail` at `collect.mwl:46`**, then
`Core\Out::capture` (`:47`). Two things are known to be waiting behind them, neither reported yet
because resolution errors are printed before lowering runs:

- `collect.mwl:43`'s `$head["name"]` **panics `mwl-ir`** — a `?array<string>` from `Core\Arr::first`
  cannot be indexed even inside a `!= null` guard. New playbook bullet under *Writing a test case*
  has the anchor and the three spellings that do lower.
- `collect.mwl:47`'s `fn () => { echo "inner"; }` is a block-bodied closure with no declared return
  type (`E0450`) and must become `fn (): void => { … }` when `Out::capture` lands.

## Next group — `Core\Validate`, then the `?array<T>` hole under the fixture

**Shared file set for `[1]`/`[2]`:** a new `crates/mwl-stdlib/src/validate.rs`,
`crates/mwl-stdlib/src/lib.rs` (`:192` the `mod` list, `:268` the `address` chain, and `symbols()`
just above it), `crates/mwl-stdlib/src/registry.rs:646` `CLASSES`, `tests/conformance/core/`, and
`examples/collect.mwl:46`. `csv.rs` is the freshest model for a new domain module: `mod` not
`pub mod`, `pub(crate) const NAME`/`CLASS`, one `address` arm per symbol.

- [ ] **`Core\Validate`'s format half** — `isEmail`, `isDomain`, `isIp(string $s, {version?: 4|6})`,
      `isMac`. Spec § 12 states all six as **prose**, at
      `docs/spec/01-core-library.md:793`, not as a table row — so **none of them is on
      `tests/spec-members-outstanding.txt`** and `spec_registry_coverage.rs` will not ask for them;
      the gate that does is `collect.mwl:46` and `conformance_coverage.rs`. `{version: 4|6}` is
      ADR 0047's literal-union option type (`CoreTy::Union` of two `IntLiteral`s — check what
      `registry::CoreTy` can actually state before writing the row). Pick the dependency, or argue
      none, in the module doc as `csv.rs` does: `isIp` is `std::net::IpAddr::from_str`, and `isEmail`
      is the one that needs a decision rather than a regex nobody can review.
- [ ] **`Core\Validate`'s byte half** — `isAscii`, `isPrintable`, same file, same case.
      `docs/spec/01-core-library.md:193` ties both to ADR 0009 and says they do **not** launder:
      no `Validate` member returns anything but `bool`.
- [ ] **The `?array<T>` index hole** (different file set — `crates/mwl-ir/src/lower/expr.rs:2952` and
      whatever records the narrowed type in `mwl-types`). It blocks `collect.mwl:43` and therefore
      the gate, whichever § 12 member lands next. Take it only as a slice of its own.

## Backlog

- `Core\Out::capture` needs ADR 0088's sink carrier, not a `string` — spec § 12, `01-core-library.md:817`.
- `Uri::parse`/`isValid` and the `Uri` instance owe an RFC 3986 dependency — the last § 12 pick.
- § 2 owes `Arr::diff`/`intersect` and ADR 0069's combination members — 19 of the ratchet's 31 keys.
- Differential is 86 of 150 and has not moved in several runs — `docs/agent/loop-goal.md` § *Stage 4*.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- `orient.py`'s `[context] modules` prints no map line for `mwl-stdlib/src/uri.rs` or `path.rs`, which
  are the two modules a new `Core` domain is copied from; add them to the goal's manifest.
