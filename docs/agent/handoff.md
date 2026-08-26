# Handoff

## State

**Conformance is the only frontier left, at 509 of 600; the differential gate is met at 151 of
the 150 it requires.** Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean) and
executes both `.mwlt` trees itself, so after a green `verify.py` there is nothing else to run
(playbook, *Running things*) — in particular no release rebuild.

**The array representation is pinned at the library layer now.** ADR 0007 § 5's "a statement about
keys, not about storage" is asserted by
`tests/conformance/core/arr-readers-agree-across-both-array-shapes.mwlt`: 53 `Core\Arr` members, 16
over the empty pair, 16 over a middle-hole subject and `foreach`, each asked of a packed list and of
the same content forced into the hash form, counted rather than read off a line. The forcing keys
are `"08"` and `"x"` because `integer_key` refuses both, so neither moves the append counter; the
counter itself, and both sides of every bound it has, are
`tests/conformance/array/unset-does-not-move-the-append-counter.mwlt`, every row of which was
checked against PHP 8.5.9 and matches.

**A crash was found on that path and is not fixed**: an append after a key of `i64::MAX` names a
live key and dies on a `debug_assert`, silently overwriting in release. The plan's `Open now` states
it with its anchors; it is the next group's first slice.

**`orient.py`'s `[context] modules` manifest is still wrong**, third session running: it names
`registry.rs`, `json.rs`, `arr.rs` and `regex.rs`, and `crates/mwl-runtime/src/{array,string}.rs`
should be in it. Separately, the pack truncates the rest-of-group items mid-sentence, so this
session spent a `peek.py` re-reading this file for item 2 — printing each remaining bullet whole
would be cheaper than the call it costs.

## Next group

Three slices over one crash, and the file set is `crates/mwl-runtime/src/array.rs`,
`crates/mwl-codegen/src/emit.rs` and `tests/conformance/array/`. Take them in order; slice 2 needs
slice 1.

- [ ] **`$a[]` refuses an occupied next element instead of crashing** — `Table::append`
      (`crates/mwl-runtime/src/array.rs:383`) reads a counter `Table::note_index`
      (`array.rs:267`) saturated at `i64::MAX`, so its `debug_assert` at `array.rs:395` fires and a
      release build overwrites a live entry. PHP 8.5 throws `Error: Cannot add element to the array
      as the next element is already occupied`, and `Fault::thrown` (`crates/mwl-runtime/src/abi.rs:92`)
      is the catchable shape to match it. The obstacle is the channel: `mwl_array_append`
      (`array.rs:1341`) is an `extern "C"` returning `*mut ArrayHeader` with nowhere to put a
      `Fault`, reached through `RuntimeSig::ArrayAppend` (`crates/mwl-codegen/src/emit.rs:714`).
      Decide the signature there and record it in `mwl-runtime`'s module doc.
- [ ] **Pin the bound on both sides** (conventions.md's *a bound asserted on both sides*) — a case
      beside `unset-does-not-move-the-append-counter.mwlt` asserting that a key of
      `9223372036854775806` still appends at `9223372036854775807` (verified) and that the next
      append is refused and catchable. It cannot be written until slice 1 lands.
- [ ] **Audit the other `RuntimeSig::ArrayAppend` users** — `mwl_array_unset` is emitted through
      that same signature at `crates/mwl-codegen/src/emit.rs:727`; say in the doc whether that is
      deliberate reuse or a name that has drifted.

## Backlog

- `python tools/gaps.py --errors` still holds nothing a case can take: 58 sites, 57 `fatal` and
  unreachable by any handler, the one `thrown` left (`csv.rs:512`) unreachable from source. Plan,
  `Open now`.
- `docs/agent/loop-goal.toml`'s `[context] modules` needs `crates/mwl-runtime/src/{array,string}.rs`
  and can drop `json.rs`/`regex.rs`. No session owns that file.
- `Core\Json::decodeAs<T>` reads a scalar-fielded class only — no enum, `decimal`, `Instant`, array
  or nested-class field. `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row, so
  `Core\Str::format` is not yet the sink that ADR makes it. `mwl_stdlib::hash`'s module doc.
- `do`/`while` is the one M4 control-flow statement that does not lower. `mwl-ir` gap 1.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
