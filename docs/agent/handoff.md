# Handoff

## State

**Spec § 7's `Core\Encoding` is whole.** `crates/mwl-stdlib/src/encoding.rs` now registers
`encodeText`/`decodeText`/`isValidText` over `Core\Charset` beside the hex, base64 and base32 pairs.
That module's own doc owns the two design calls a reader will otherwise read as bugs: **`Ascii` and
`Latin1` are cases of their own** rather than the standard's aliases of `windows-1252`, and
**`replacement` is absent**. `docs/spec/01-core-library.md` § 7's `Charset` paragraph is amended to
match. Five cases (`Utf8`, `Utf16Le`, `Utf16Be`, `Ascii`, `Latin1`) convert in that module; the other
36 delegate to `encoding_rs`, whose `[workspace.dependencies]` comment states the ADR 0051 § 4 answer.
`cargo deny check` is green and `THIRD-PARTY-LICENSES.txt` is regenerated.

**`examples/collect.mwl`'s frontier has moved off § 7** — it now reports `Core\Uri::parseQuery` at
`collect.mwl:36`, then `Core\Csv::parse`, `Core\Validate::isEmail` and `Core\Out::capture`. Those are
§ 12, not the next group: `Core\Bytes` is the rest of § 7 and is the cheaper file set.
Conformance is **380** of 600; differential is 86 of 150 and has not moved.

**One session, one slice.** Slice 2 needs `crates/mwl-stdlib/src/str.rs` loaded to mirror, which is a
fresh ~400-line read on top of this session's; the ceiling rule said stop.

**`orient.py`'s `[context] modules` is missing `hash.rs`.** `Core\Digest` is the only worked example
of a `Core`-owned enum — the `CoreEnum` roster, the Rust mirror it dispatches on, and the test that
pins the two together — so any slice adding an enum pays to rediscover it. Add the selector.

## Next group — `Core\Bytes`, the rest of § 7

**Shared file set:** a new `crates/mwl-stdlib/src/bytes.rs` (a `mod`, not a `pub mod` — see the
playbook), `crates/mwl-stdlib/src/str.rs` (`:49` `CLASS`, `:53` `length`, `:60` `at`, `:95` `slice`,
`:102` `indexOf`, `:141` `join`, `:202` `repeat`, `:374` `address` — the mirror to copy member for
member), `crates/mwl-stdlib/src/registry.rs` (`CLASSES` `:614`), `crates/mwl-stdlib/src/lib.rs`
(`:191` the `mod` list, `:263` the `address` chain), and `tests/conformance/core/`.
Spec rows: `docs/spec/01-core-library.md` § 7 (`:590`), whose `Core\Bytes` paragraph is at `:608`.

- [ ] **`Core\Bytes`, the indexing half** — `length`, `at`, `slice`, `indexOf`, `compare`
      (`01-core-library.md:608`). R6 pairs each with `Core\Str`'s member of the same name, and the
      difference is the unit: `Core\Str` indexes by grapheme cluster and this indexes by **byte
      offset**, which ADR 0009 § 1 says is the only unit `bytes` has to be ambiguous about. So the
      bodies are simpler than `str.rs`'s, not a copy of them — no `unicode-segmentation`, and `at`
      answers a `uint` rather than a one-character `string`.
- [ ] **`Core\Bytes`, the predicates and the builders** — `contains`, `startsWith`, `endsWith`,
      `join(array<bytes> $parts, bytes $separator = "")`, `fill`, `repeat`. The three predicates are
      what magic-byte sniffing needs; `join` rather than a `concat` of its own is what keeps R6's
      pairing with `Core\Str`. Same file set, so this is the cheap second slice.
- [ ] **`Core\Bytes::pack`/`unpack`** — `pack(string $format, mixed ...$values)` and
      `unpack(bytes $b, string $format): array<mixed>`. The format string is an **ADR 0057 intrinsic**
      (§ 1's closed list) *and* an ADR 0088 **sink** — one of R11's four grammars, exactly as
      `Core\Str::format`'s template is. `registry::CoreTy::Variadic` is one ABI argument whatever the
      call writes, so `pack` is `args: [2]` (playbook: *A registry row's arity and its helper's
      `args: [N]`*).

## Backlog

- `mwl-stdlib`'s member rows carry no ADR 0088 qualifier classification, and nothing refuses an
  unclassified `string`/`bytes` parameter — `docs/implementation-plan.md` § *Open now*.
- § 12's `Core\Uri::parse` still owes an RFC 3986 dependency, and `Core\Csv` one under ADR 0051 § 4 —
  both are `examples/collect.mwl`'s remaining reports.
- `every_part_one_spec_member_is_registered`, the loop's own definition of done, does not exist yet —
  `docs/agent/loop-goal.md` § *Acceptance*.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s module doc.
- Differential is 86 of 150 and has not moved in several runs — `docs/implementation-plan.md`.
- `docs/spec/02-php-migration.md` is 31% classified; `mbstring`'s rows are now answerable, since
  `Core\Encoding`'s trio is what replaces `mb_convert_encoding` and `mb_check_encoding`.
