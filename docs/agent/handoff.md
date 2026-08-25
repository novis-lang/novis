# Handoff

## State

**§ 12's query pair is whole.** `Core\Uri::parseQuery` and `Core\Uri::buildQuery` are registered,
implemented and pinned by a conformance case each; both live in `crates/mwl-stdlib/src/uri.rs`
beside the four percent-encoding members they are built out of, and that module's own doc owns the
bracket convention, the two places it diverges from `parse_str` (no key is ever rewritten, and a
malformed name stays one literal key) and the one place `buildQuery` differs from `as string`
(`false` writes `0`). The spec table already said `array<mixed>` and already named
`Core\Request::query` at M8 — no spec edit was owed after all.

**Both members walk with an explicit stack, not recursion**, because the depth is caller text; each
descent *borrows* the child array the parent already owns rather than retaining it, so nothing is
copied. That is a new refcount edge and it is valgrind-clean
(`tools/leak-check.sh .agent-tmp/uri-query-leak.mwl`, 0 failures).

**The ratchet in `crates/mwl-stdlib/tests/spec-members-outstanding.txt` is at 33 keys** for §§ 2-12;
when it is empty, Part I is registered whole. Conformance is **396** of 600; differential is 86 of
150 and has not moved.

**`examples/collect.mwl`'s frontier has moved to `Core\Csv::parse` at `collect.mwl:39`**, then
`Core\Validate::isEmail` (`:46`) and `Core\Out::capture` (`:47`). That last line also needs a fixture
fix of its own: `fn () => { echo "inner"; }` is a block-bodied closure with no declared return type
(`E0450`), so it must become `fn (): void => { … }` when `Out::capture` lands.

## Next group — `Core\Csv`, both halves

**Shared file set for `[1]`/`[2]`:** a new `crates/mwl-stdlib/src/csv.rs`,
`crates/mwl-stdlib/src/lib.rs` (`:210` the `mod` list, `:244` `symbols()`, `:277` the `address`
chain), `crates/mwl-stdlib/src/registry.rs:625` `CLASSES`,
`crates/mwl-stdlib/tests/spec-members-outstanding.txt`, `tests/conformance/core/`, and
`examples/collect.mwl:39`. Take both — one module, one dependency pick, and the round trip is the
case that pins either honestly. `[3]` is a different file; leave it.

- [ ] **`Core\Csv::parse`** (§ 12; spec row
      `parse(string $text, {separator?, quote?, escape?, header?: bool}): array<array<string>>`).
      Pick the crate under [ADR 0051](../adr/0051-standard-library-tiers.md) § 4's two questions —
      CSV *is* attacker-reachable, so a C dependency needs question 2's record and almost certainly
      fails it; record the pick and its reasoning in the module's own doc comment, per the goal's
      standing decisions. The three things a new dependency owes are in `playbook.md` § *Adding a
      `Core` member*. `{header: true}` consumes the first row as column names and keys every returned
      row by them, and is not itself returned — the return type is unchanged because every MWL array
      key is a `string` already.
- [ ] **`Core\Csv::format`** (§ 12) — the inverse, same file, same crate;
      `{header: [...]}` writes those names as the first row.
- [ ] **`Core\Validate`'s six members** (§ 12) — `isEmail`, `isIp(…, {version?: 4|6})`, `isMac`,
      `isDomain`, `isAscii`, `isPrintable`, all `(subject, …): bool`. A different file
      (`crates/mwl-stdlib/src/validate.rs`), so a different group. **No `Validate` member launders
      anything** (ADR 0024), and `isUrl`/`oneOf`/`isIpV4`/`isIpV6` are deliberately absent.

## Backlog

- `Uri::parse`/`isValid` and the `Uri` instance need the RFC 3986 crate pick — `uri.rs` gap 1.
- `uri.rs` gap 2 is now a *spec* question, not a runtime one: `Tag::Bytes` exists, so the two
  decoders could answer `bytes`, but § 12's table writes `string`.
- `Core\Out::capture` needs ADR 0088's sink carrier, not a `string` — `docs/spec/01-core-library.md`
  § 12.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s module doc.
- ADR 0007 § 2's `array<T> as array<U>` row does not lower at all — `mwl-ir` gap; it is what stops a
  case indexing into an `array<mixed>` (`playbook.md` § *Writing a test case*).
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
