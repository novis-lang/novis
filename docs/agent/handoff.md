# Handoff

## State

**The driver's failing acceptance check is closed.**
`tests/conformance/core/a-command-table-answers-its-own-help.nvst` is written and passes. It does not
call `Core\Command::help` on purpose: that member, `::run` and `::completions` are goal 4's
(`docs/agent/goals/1-core-depth.md` item 6 — "the table only"), and no `Core\Command` row exists in
`nvs_stdlib::registry`. What it asserts instead is the claim § 6's promise rests on — *nobody writes
usage text* — by generating the help out of the declarations themselves through ADR 0046 §§ 4-5's
retrieval: four rows over three methods, every one described, no two named alike, and one `{about:
string}` shape asked of a `#[Command]` and an `#[Option]` alike. It also took `Core\Attributes::all` off
`BELOW_THE_FLOOR` in `crates/nvs-stdlib/tests/conformance_coverage.rs:264` — one line of item 10, paid
for by a case written for something else.

**Item 12 has started: `Core\Str` is off the `UNCLASSIFIED` roster.** All 37 rows carry ADR 0088 § 2's
classification, and the rule they were classified by is written down once, in `Qual`'s own doc comment
(`crates/nvs-stdlib/src/registry.rs:74`), for the classes still owing one. It lands three ways: neutral
wherever the answer carries no byte of an argument, contagious everywhere else — including a needle
that never appears in the answer — and sink on `format`'s template, which is one of ADR 0063 R11's four
grammars. **The cross-check that made this cheap**: `docs/spec/01-core-library.md`'s Q column already
renders every one of these marks, and it agreed with the independent judgement member for member. Read
it first for the next class.

**The classification is declaration-only today.** `nvs_types::core_lib.rs:286` lowers `CoreTy::Text(_)`
to a plain interned `string`, so nothing a program can do changed and the playbook bullet about no
`Core` member accepting a `tainted` argument is still true — including for `Core\Str`. Enforcement is a
separate item and this session did not touch it.

**Untouched:** `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 are stale (the route table
is built now), a `bytes` array key ICEs in `nvs-ir`, and `catch (Core\Error $e)` panics — the last two
have playbook bullets under *Writing a test case*.

**Orientation gaps.** `[context] adrs` printed no section for either ADR this session needed: 0086 § 6
(the command table's own spelling) and 0088 §§ 1-2 (§ 1's sink predicate, whose R11 corollary is what
makes `Core\Str::format` a sink). Both were sliced by hand for one call each. Still owed from before:
`0085 §§ 1-4` and `0071 §§ 2, 7`.

## Next group

**Item 12 continued, the qualifier classification — ADR 0088 § 2. Shared file set:**
`crates/nvs-stdlib/src/registry.rs` (the `UNCLASSIFIED` roster at `:1526`, which now opens at
`Core\Arr`) plus one `crates/nvs-stdlib/src/<class>.rs` per slice. `Qual`'s doc at `registry.rs:74` is
the rule; the spec's Q column is the second opinion, and a disagreement between the two is the finding.

- [ ] **`Core\Arr` and `Core\Attributes`, the four rows the roster opens with.** `hasKey` and `column`
      take a `string` key beside an `array<T>`; `get`/`all` take the optional member name, which names a
      declaration rather than flowing anywhere. Anchors: `crates/nvs-stdlib/src/arr.rs:176`, `:368`,
      `crates/nvs-stdlib/src/attributes.rs:47`, `:54`, `crates/nvs-stdlib/src/registry.rs:1526`.
- [ ] **`Core\Math::fromBase`/`format` and `Core\Json::decode`/`decodeAs`/`isValid`.** ADR 0088 § 1's
      table names `Core\Json::decode`'s input outright — data, tainted-friendly, result tainted — so
      that row is decided and the other four follow `Qual`'s rule. Anchors:
      `crates/nvs-stdlib/src/math.rs`, `crates/nvs-stdlib/src/json.rs`.
- [ ] **`Core\Encoding`'s five rows**, same file set plus `crates/nvs-stdlib/src/encoding.rs`. Base64
      and text transcoding are contagious in both directions; the question worth a minute is whether
      `isValidText` is neutral, which it is by the answer-carries-no-byte half of the rule.

## Backlog

- `Core\Command::run`/`::help`/`::completions` — goal 4, not this goal (`1-core-depth.md` item 6).
- The rest of `UNCLASSIFIED` after the three slices above — `registry.rs:1526` is the worklist.
- Enforcing the classification rather than declaring it: `nvs_types::core_lib.rs:286` erases it.
- `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 no longer describe the tree.
- Item 10's per-class conformance floor — `python tools/gaps.py` is the worklist.
- A `bytes` array key ICEs in `nvs-ir`; `catch (Core\Error $e)` panics — both have playbook bullets.
