# Handoff

## State

**Stage 0's last item is landed, and § E of `docs/perf/userland-gap.md` is struck whole.** A
`Core\Str` member now writes its result **into the allocation it is answered from**:
`mwl_runtime::MwlStr::build` (`string.rs:@build`) hands the producer a `StrWriter`
(`string.rs:@StrWriter`) over that allocation, so nothing is built in a `String` and copied in
afterwards. `repeat`, `padStart`/`padEnd` and `reverse` know their length exactly and allocate once;
`replace` starts the writer at its subject's length and the writer `realloc`s on `String`'s own
doubling. **`join` is deliberately unchanged** — its length costs a walk of the subject's array
slots and all three ways round that measured worse; `str.rs`'s new module § *A result is written
once* holds that finding and the two rejected "measure it first" designs so they are not retried.

**Measured, full 9-rep sweep:** suite median **0.69× → 0.80×**. `05-string-replace` 0.91× →
**1.05×** (work 80.1 ms → 70.8 ms), `07-string-normalize` 0.50× → **0.93×** (81.8 → 44.5). Verify is
green (**1584** tests, 74 suites, clippy and fmt clean); no refcount edge changed, so no valgrind run
was owed.

**The acceptance check for item 22 is still red, and it is one test short.**
`loop-goal.toml:456` (`mwl-stdlib (one write per result)`) names two tests.
`a_str_member_allocates_its_result_once` exists now — it counts allocations behind a debug-only
counting allocator in `crates/mwl-stdlib/tests/allocation_policy.rs`. The other,
`no_member_revalidates_a_string_argument`, **does not exist and cannot pass yet**: nine sibling
argument readers still run `std::str::from_utf8` over a `string` the tag already guarantees. That is
the next group's first slice, and it closes the check.

**`orient.py` still does not print `docs/perf/userland-gap.md`.** `[context]` in `loop-goal.toml`
has no field selecting a perf doc. Stage 0 is done, so the manifest now needs re-pointing wholesale
rather than one more selector.

## Next group

All of `crates/mwl-stdlib/src/`, one mechanical change repeated across nine files, plus the guard
the acceptance check names. Slice 1 unblocks the Stage 0 check; 2 and 3 carry the same two patterns
into the modules `str.rs` left behind.

- [ ] **The sibling `text()` helpers stop re-validating.** Each reads `as_str_bytes` and then
      `std::str::from_utf8` — an O(n) pass ADR 0009's tag already discharges. Replace the pair with
      `Value::as_text()`, keeping each module's own wrong-tag message: `csv.rs:240`,
      `encoding.rs:446`, `json.rs:1009`, `path.rs:380`, `regex.rs:575`, `time.rs:1505`,
      `uri.rs:680`, `uuid.rs:206`, `validate.rs:250`. `str.rs:620` is the shape to copy. Then add
      **`no_member_revalidates_a_string_argument`** beside
      `a_str_member_allocates_its_result_once` in `crates/mwl-stdlib/tests/allocation_policy.rs`
      (a source scan over those readers is the cheapest true form), which is what turns
      `loop-goal.toml:456` green.
- [ ] **`produced` writes once in the sibling modules.** The same helper as `str.rs`'s, still
      allocating twice: `bytes.rs:425`, `path.rs:434`, `regex.rs:678`, and `uri.rs`'s own. Use
      `MwlStr::build` where a length is exact or a capacity is a fair guess, and leave a member
      alone where the length costs a walk — `str.rs`'s § *A result is written once* is the rule and
      the playbook's *Performance* section says why measuring first is not the answer.
- [ ] **Re-point `[context]` in `docs/agent/loop-goal.toml`.** Stage 0 is finished, so its
      manifest — perf modules, ADR 0009's §§, the string playbook bullets — selects the wrong pack
      for whatever the loop takes next. `python tools/orient.py --audit` prices it.

## Backlog

- `Core\Out::capture` is the **last** key on `crates/mwl-stdlib/tests/spec-members-outstanding.txt`,
  and `Core\Out` has no module at all yet; it needs M4S's sink work first (plan, *Open now*).
- `examples/collect.mwl` is Stage 3's one failing fixture, blocked on that member at `collect.mwl:47`.
- `Core\Json::decodeAs<T>` — `mwl_stdlib::json` gap 2, unblocked now that a call site may write a
  type argument.
- Stage 4's counts are their own work: conformance 436 of 600, differential 90 of 150.
- Item 19 still owes the append-only `docs/perf/history.ndjson` entry item 15 asked for.
- `do`/`while` does not lower (`mwl-ir` gap 1's remainder); ADR 0088's registry-wide qualifier
  classification lands with M4S.
