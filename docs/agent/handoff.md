# Handoff

## State

**Conformance is the only frontier left, at 510 of 600; the differential gate is met at 151 of the
150 it requires.** Verify is green (**1597** cargo tests, 74 suites, clippy and fmt clean) and
executes both `.mwlt` trees itself, so after a green `verify.py` there is nothing else to run
(playbook, *Running things*) — in particular no release rebuild.

**The `$a[]` crash is closed, and all three slices of the previous group landed.** `$a[] = v` onto an
array whose append counter has saturated at `i64::MAX` now throws PHP 8.5's `Cannot add element to
the array as the next element is already occupied` as a catchable `LogicError` instead of firing a
`debug_assert` and silently overwriting a live entry in release. The decision — why
`mwl_array_append` alone among the array primitives carries ADR 0002's `(ctx, array, value, out) ->
status` shape, and why the occupancy test runs *before* the copy-on-write separation so a refusal
leaves the caller's pointer live and owned — is `crates/mwl-runtime/src/array.rs`'s module doc
§ *the append is the one array write with a fault channel*. Slice 3's audit was forced into slice 1
rather than deferred: `mwl_array_unset` was emitted through `RuntimeSig::ArrayAppend` and now has
its own `RuntimeSig::ArrayUnset`; the playbook's *Writing MWL itself* has the trap.

**The new refcount edge is valgrind-clean.** `tools/leak-check.sh` over a fixture exercising the
refusal three ways — a fresh string temporary, an aliasing read of a local, and an uncaught
propagate out of a callee frame holding a shared (refcount-2) array — reports 0 failures.

**`orient.py`'s `[context] modules` manifest is still wrong, fourth session running.** It names
`registry.rs`, `json.rs`, `arr.rs` and `regex.rs`; this session worked entirely in
`crates/mwl-runtime/src/array.rs`, `crates/mwl-codegen/src/{emit,lib}.rs` and
`crates/mwl-ir/src/{ir.rs,lower/}`, none of which the pack printed a map line for. The pack also
still truncates the rest-of-group bullets mid-sentence, costing one `peek.py` back into this file.

## Next group

Four cases closing three of the eight members `python tools/gaps.py --differential` still lists,
plus the conformance case the same file set is already open for. The file set is
`crates/mwl-stdlib/src/path.rs`, `tests/differential/` and `tests/conformance/core/`; the first
three are one shape each and share their reading.

- [ ] **`Core\Path::basename` against PHP's `basename`** (`crates/mwl-stdlib/src/path.rs:447`) — a
      `tests/differential/` oracle case, so PHP computes the expectation and nothing is frozen by
      hand. Sweep the boundaries the twin is written around: a trailing separator, a bare name, a
      root, an empty subject, a suffix argument that does and does not match.
- [ ] **`Core\Path::dirname` against PHP's `dirname`** (`path.rs:478`) — same shape, same file, and
      the two twins disagree with each other at the root, which is the row worth having.
- [ ] **`Core\Path::normalize` against PHP's `realpath`** (`path.rs:664`) — the twin touches the
      filesystem and MWL's does not, so pin the rows where they agree and say in the case comment
      which rows are deliberately absent rather than asserting a divergence PHP cannot answer.
- [ ] **One `tests/conformance/core/` case: the separator invariant, counted rather than read off a
      line** (conventions.md's *invariance over a sweep*). Every `Core\Path` member that builds a
      path emits `Path::SEPARATOR`, which differs between the native and WSL legs (loop-goal.md
      § *Standing decisions*), so assert that a table of subjects round-trips through
      `Core\Str::replace($p, Core\Path::SEPARATOR, "/")` identically on both — a member that grew
      its own separator handling fails here while still looking right on its own line.

## Backlog

- Five members with a PHP twin and no oracle case remain after the group above — `Core\Arr::flattenDeep`, `Core\Math::gcd`/`lcm`, `Core\Json::decode`/`isValid` (`python tools/gaps.py --differential`).
- `Core\Csv::format`'s `thrown` at `crates/mwl-stdlib/src/csv.rs:512` is the one catchable site `gaps.py --errors` lists among 58; the other 57 are `fatal` and unreachable by any case.
- `MwlArray::append`'s 62 producers still call the panicking wrapper, which is sound only because each builds its array from index 0; the first `Core` member to append onto a *caller's* array wants `try_append` and a `Fault` (`crates/mwl-runtime/src/array.rs`).
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` gap 1).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row (`mwl_stdlib::hash`'s module doc).
