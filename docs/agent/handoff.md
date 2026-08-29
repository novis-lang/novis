# Handoff

## State

**The failing acceptance check is closed.**
`tests/conformance/reject/a-derive-field-with-no-codec-is-refused-at-its-declaration.nvst` is
written and passes. ADR 0071 § 2's reachable-set refusal is pinned with both sides of the bound
in one refusing file: `Row::$cache` and `Row::$h` are both `Handle`, a class with no codec, and
§ 2's own `#[Json\Field(skip: true)]` takes the first off the contract — so it is examined and
accepted before `$h` is reached, and `E0756` names exactly one property. `Handle` is declared
*after* the class that names it, which is what pins the reachability question being asked over
the whole program rather than at the property walk.

**Item 11's four biggest remaining modules are finished.** Sixteen sites left `OWED_A_CASE`,
each probed with `nvs check` on a scratch file before the comment was written:

- `arr.rs` (7): `chunk`'s `size` and `fill`'s `count` (`CoreTy::Uint`, probed with `mixed` and
  with the literal `-1`, which is `E0401: expected 'uint', found 'int'` rather than a guard),
  `fillKeys`' key array, `sort`'s `order` and `preserveKeys`, `sortByKey`'s `order` (the
  `CoreTy::Enum` shape `Core\Str::normalize` states in full), and `average`'s `usize`→`u64`
  counter.
- `uri.rs` (7): `written`'s five `with` string options, `with`'s `port`, `resolve`'s `string`
  parameter, `buildQuery`'s array, plus three that are **post-conditions rather than
  boundaries** and say so — `scalar_text`'s `value_to_string` answer (every `Ok` arm of that
  helper builds a `Value::str`, carrier arm included), `resolve`'s null `text` slot
  (`Core\Uri` declares no `constructor`, so every instance came from `built`, whose slot 0 is
  unconditional), and `resolve`'s re-parse of `fluent-uri`'s own output.
- `hash.rs` (2): `hmac`'s `CoreTy::Union(STRONG)` parameter, and `Stream::finish`'s chunk tag —
  `update` is the slot's only writer and its parameter is `bytes|string`.
- `bytes.rs` (3): `pack`'s format and its `CoreTy::Variadic` value list (the judgement
  `Core\Path::join` states in full), and `unpack`'s format.

**Sixteen lines are left in `OWED_A_CASE`**, across nine files, and it is still the worklist.

**Untouched:** item 12's classification (`UNCLASSIFIED`, `crates/nvs-stdlib/src/registry.rs`).

**Found, not fixed:** a `bytes` array key ICEs in `nvs-ir` rather than being diagnosed, and
`catch (Core\Error $e)` panics — both have playbook bullets under *Writing a test case*.

**Orientation gap, fifteenth session running:** `[context] adrs` still does not carry
`0085 §§ 1-4`, and now also owes **`0071 §§ 2, 7`** — this session wrote a case pinning § 2 and
had to reconstruct the rule from `nvs_types::derive`'s module doc and
`crates/nvs-types/tests/derive.rs`. Nothing in the pack still names the stage-5 `[[check]]`
blocks' `cases`/`tests` lists, so an acceptance failure naming a `.nvst` is triaged by
`grep`ping `loop-goal.toml`.

## Next group

**Item 11's tail, then item 12's next class. Shared file set:**
`crates/nvs-stdlib/tests/conformance_coverage.rs:603` — `OWED_A_CASE` is the worklist and a
slice is done when its lines are gone — plus one `crates/nvs-stdlib/src/<class>.rs` per slice.

- [ ] **`router.rs`'s three stems and `format.rs`'s one, goal § item 11.** Two of the four carry
      the same `value_to_string` post-condition this session judged at
      `crates/nvs-stdlib/src/uri.rs`'s `scalar_text`, so they are a citation rather than a fresh
      judgement — read that comment first. Sites: `router.rs:229`, `:282`, `:285`,
      `format.rs:475`.
- [ ] **`json.rs`'s three stems and `regex.rs`'s one, goal § item 11.** All four are internal
      contracts rather than argument guards, so expect the `uri.rs` post-condition shape and not
      the `E0401` one. Sites: `json.rs:250` (`maxDepth` fits a `u32`), `:714`, `:823`,
      `regex.rs:1042`.
- [ ] **`validate.rs`'s two and `math.rs`'s one, goal § item 11.** `Core\Validate::isIp`'s
      `version` option and `Core\Math::round`'s `mode` are both the `CoreTy::Enum` shape
      `arr.rs`'s `sort` now states — probe each with a `mixed` binding and with the bare `int`
      literal. Sites: `validate.rs:275`, `:280`, `math.rs:1302`.

## Backlog

- `debug.rs:147` and `:187`, and `path.rs:606` — the last three of `OWED_A_CASE` after the group
  above, plus `test.rs:663` and `:692`. `crates/nvs-stdlib/tests/conformance_coverage.rs`.
- Item 12's `Core\Validate` (6) and `Core\Uuid` (2) qualifier rows, all `Qual::Neutral` —
  `docs/agent/loop-goal.md` § item 12.
- `catch (Core\Error $e)` panics in `nvs-ir` instead of diagnosing — `docs/agent/playbook.md`.
- A `bytes` array key ICEs in `nvs-ir` rather than being refused — `docs/agent/playbook.md`.
- `[context] adrs` owes `0071 §§ 2, 7` and `0085 §§ 1-4`; no `[context]` field names the stage-5
  `[[check]]` case lists — `docs/agent/loop-goal.toml`.
