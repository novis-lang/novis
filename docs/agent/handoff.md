# Handoff

## State

**The failing acceptance check is closed.** `nvs-stdlib (the coverage gates)` names five tests and all
five now exist; `every_error_path_is_asserted_or_declared_unreachable`
(`crates/nvs-stdlib/tests/conformance_coverage.rs:692`) was the last one.

**The gate is a source scan with two escapes.** Every `Fault::` site in `crates/nvs-stdlib/src` whose
message opens with fourteen or more literal characters must either have that stem appear in a case
under `tests/conformance/` or `tests/differential/`, or carry the phrase `unreachable from source` in a
comment within `DECLARATION_WINDOW` (8) lines above it (`conformance_coverage.rs:373`). `OWED_A_CASE`
(`:604`) is the ratchet it landed over — 57 entries keyed by `(file, stem)`, failing both on a site
that is not listed and on a listed one that has since become asserted or declared, so it only shrinks.
What it deliberately cannot see is in the test's own doc comment.

**Ten `Core\Arr` guards are declared rather than listed.** `count`, `isEmpty`, `hasKey`, `isList`,
`keys`, `values`, `reverse`, `flip`, `withoutFirst`, `withoutLast` all take `array<T>` at parameter 0,
so `E0401: expected array<mixed>, found mixed` refuses the call before the runtime answers — probed
with `nvs run`, not assumed. `nvs_core_arr_count` (`crates/nvs-stdlib/src/arr.rs:829`) states the
judgement and the other nine cite it; that is the shape every remaining `expected {:?}, got tag {}`
row takes.

**Untouched:** item 12's classification (`UNCLASSIFIED`, `crates/nvs-stdlib/src/registry.rs:1509`,
still 118) and the six members in `BELOW_THE_FLOOR` (`conformance_coverage.rs:264`).

**Orientation gap, twelfth session running:** `[context] adrs` still does not carry `0085 §§ 1-4`.
Still true from last session and paid for a second time: nothing in the pack says which of the
coverage gates' five named tests exist, so the failing check has to be triaged by `grep`ping
`loop-goal.toml`'s stage-5 `[[check]]` block, which is the only home of that list and which no
`[context]` field prints.

## Next group

**Item 11's remaining families, then item 12's next class. Shared file set:**
`crates/nvs-stdlib/tests/conformance_coverage.rs:604` — `OWED_A_CASE` is the worklist, and a slice is
done when its lines are gone — plus one `crates/nvs-stdlib/src/<class>.rs` per slice.

- [ ] **The argument type-guards outside `arr.rs`, goal § item 11.** Same judgement as the ten already
      declared, one module at a time and checked against that module's own `CLASS` row before writing
      it: `bytes.rs:707`, `json.rs:237` and `:533`, `path.rs:592`, `debug.rs:177`, `str.rs:882` and
      `:1305`, `test.rs:315`, `:367`, `:432`. A guard behind a `mixed` or `Instance` parameter is
      **not** in this family — check the row, then probe with `nvs run` as `arr.rs` did.
- [ ] **The two `Fault::thrown` boundaries item 11 names outright**, which owe a case rather than a
      comment: `crates/nvs-stdlib/src/csv.rs:512` (a row holding a value that is not a `string`) and
      `crates/nvs-stdlib/src/router.rs:351` (`urlAbsolute` with no origin configured). A `.nvst` case
      catches and echoes the message, which is what removes the line. `Core\Router::urlAbsolute` is
      also in `BELOW_THE_FLOOR`, so that one case closes a line of item 10 as well.
- [ ] **`Core\Validate` (6) and `Core\Uuid` (2), all `Qual::Neutral`, goal § item 12.** The rows are
      `crates/nvs-stdlib/src/validate.rs:159` and `crates/nvs-stdlib/src/uuid.rs:117`; delete the
      eight lines they own from `UNCLASSIFIED` (`registry.rs:1509`), which the gate at `:1652`
      requires in the same commit.

## Backlog

- `Core\Str`'s 37 rows — item 12's largest single block, the spec's Q column at
  `docs/spec/01-core-library.md` § 1.
- Wire `Qual` into `nvs_types::expr::args` so a classification refuses a `tainted` argument — ADR 0088
  § 2; only the data has landed.
- The six members still under the conformance floor — `BELOW_THE_FLOOR`, `conformance_coverage.rs:264`.
- The 104 `Fault::` sites whose message opens on its own format hole are outside both `gaps.py` and the
  gate; a few literal words before the first hole brings one under it and makes a better message.
- `[context] adrs` lacks `0085 §§ 1-4`, and no `[context]` field prints an acceptance check's `tests`
  list — `docs/agent/loop-goal.toml`.
