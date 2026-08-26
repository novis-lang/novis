# Handoff

## State

**Conformance is the only frontier left, at 506 of 600; the differential gate is met at 151 of
the 150 it requires.** Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **506 passed, 0 failed** and `mwl test tests/` is **657 passed, 0
failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all (playbook,
twice), and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is newer than
it (playbook, *Running things*).

**`python tools/gaps.py --errors` holds nothing a case can take**, and has since the previous
session: 58 sites, 57 `fatal` and unreachable by any handler, the one `thrown` left (`csv.rs:512`)
unreachable from source. Every depth slice from here is a judgement call against conventions.md's
four case shapes.

**Six members stopped aborting this session and none of them is asserted yet.**
`Core\Str::repeat`, `::padStart`, `::padEnd`, `Core\Bytes::fill`, `::repeat` and `::join` all
reserved infallibly after `mwl_runtime::affordable` had let a count through, so any size below
`isize::MAX` the machine could not serve killed the process — exit 127, nothing catchable. The
string half goes through the new `MwlStr::try_build` (`crates/mwl-runtime/src/string.rs:317`), the
bytes half through `try_reserve`; the plan's `Open now` owns the shape, the two sentences and which
member reaches which. **Measured after the change**: all six throw and are caught, exit 0.

**`Core\Arr::fill` is the last one, and it is a runtime seam rather than a call-site fix** —
`MwlArray` has no fallible growth at all. That is item 1 below.

**`orient.py`'s `[context] modules` manifest is still wrong.** It names `registry.rs`, `json.rs`,
`arr.rs` and `regex.rs`; this session needed `str.rs`, `bytes.rs` and `crates/mwl-runtime/src/{string,abi}.rs`.
`json.rs` and `regex.rs` can come out; `arr.rs` and `crates/mwl-runtime/src/array.rs` are what the
next item needs.

## Next group

Two slices, and the second is what makes the first six changes visible to the gate. The file set is
`crates/mwl-runtime/src/array.rs`, `crates/mwl-stdlib/src/arr.rs` and `tests/conformance/core/`.
Take them in this order; the second stands alone if the first proves larger than it looks.

- [ ] **`Core\Arr::fill` draws fallibly** — `crates/mwl-stdlib/src/arr.rs:1317` (`append_copies`'s
      `affordable` call) and `arr.rs:1332` (`append_borrowed`, the infallible push). Unlike the
      string and bytes halves there is no fallible entry point to reach for: `MwlArray`'s growth in
      `crates/mwl-runtime/src/array.rs` aborts, so the slice adds one there — `MwlStr::try_build`
      (`crates/mwl-runtime/src/string.rs:317`) is the worked shape for splitting an aborting
      allocation into a fallible half and an `unwrap_or_else(handle_alloc_error)` wrapper.
      `Core\Arr::fill(1000000000000, 0)` aborts today; it must throw and be caught.
- [ ] **One conformance case for the agreement** (conventions.md's fourth shape) —
      `tests/conformance/core/` as its own new file, because the gate counts files. Every
      count-shaped allocator refuses the same way: one step inside its bound it answers, one step
      outside it throws in `RuntimeError` naming the member. The seven reachable members are
      `Core\Str::repeat`/`padStart`/`padEnd`, `Core\Bytes::repeat`/`fill`, `Core\Random::bytes`/`token`,
      the last two already asserted by `random-every-count-shaped-refusal-agrees.mwlt` — read that
      case for the shape rather than inventing one. `Core\Bytes::join`'s refusal is not reachable;
      the playbook's new bullet says why and how the obvious probe misreports.

## Backlog

- `mwl_stdlib::json` gap 2: `decodeAs<T>`'s decoder reads a scalar-fielded class only (ADR 0071).
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row (plan, `Open now`).
- A class carrying `#[Json]` and declaring no field refuses with the wrong reason (plan, `Open now`).
- `Core\Regex\Match::group`'s bad-key refusal is `RuntimeError` where the call-site-argument rule
  argues `LogicError` (plan, `Open now`).
- `Core\Str::wrap`'s multibyte half is the last unwritten `--ORACLE-DIVERGES--` file (plan, `Open now`).
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
