# Handoff

## State

**Conformance is the only frontier left, at 506 of 600; the differential gate is met at 151 of
the 150 it requires.** Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean),
`mwl test tests/conformance` is **506 passed, 0 failed** and `mwl test tests/` is **657 passed, 0
failed** — run those as well as `verify.py`, which executes no `.mwlt` case at all (playbook,
twice), and **rebuild `target/release/mwl.exe` first** if anything under `crates/` is newer than
it (playbook, *Running things*).

**`python tools/gaps.py --errors` holds nothing a case can take.** 58 sites, 57 of them `fatal`
and unreachable by any handler, and the one `thrown` left (`csv.rs:512`) is unreachable from
source — the plan's `Open now` says why. Every depth slice from here is a judgement call against
conventions.md's four case shapes, not a tool's output.

**One behaviour changed this session, and the same defect is still live in six other members.**
`Core\Random::token` reserved its draw through `vec![0; n]`/`String::with_capacity`, which *abort
the process* when the allocator refuses; `mwl_runtime::affordable` only rejects a size past
`isize::MAX`, so any count below that the machine cannot serve killed the process — exit 127,
nothing catchable. It reserves through `try_reserve_exact` now. `Core\Str::repeat`, the padding
pair, `Core\Bytes::fill`/`repeat`/`join` and `Core\Arr::fill` still abort, measured, and that is
the next group; the plan's `Open now` carries the anchors.

**`Core\Random` is closed** — the previous handoff's third slice (`int`'s inclusive ends) is
already asserted by `random-every-draw-is-swept-for-its-invariants.mwlt`, so it is dropped rather
than carried.

**`orient.py`'s `[context] modules` manifest is still wrong.** It names `registry.rs`, `json.rs`,
`arr.rs` and `regex.rs`; this session needed `random.rs` and `crates/mwl-runtime/src/abi.rs`, and
the group below needs `str.rs`, `bytes.rs` and `arr.rs`. `json.rs` and `regex.rs` can come out.

## Next group

One seam and its call sites: `mwl_runtime::affordable`
(`crates/mwl-runtime/src/abi.rs:161`) refuses only what cannot be allocated *at all*, so every
member that trusts it and then allocates infallibly aborts the process on a count between the two.
The worked fix is `mwl_core_random_bytes` at `crates/mwl-stdlib/src/random.rs:315` — reserve
fallibly, then `resize` — and the file set is `crates/mwl-stdlib/src/{str,bytes,arr}.rs` plus
`tests/conformance/core/`. Take them in this order; the first is the one measured to abort.

- [ ] **`Core\Str::repeat` and the padding pair draw fallibly** — `crates/mwl-stdlib/src/str.rs:2099`
      (`built(len, …)` is the infallible one) and `str.rs:2060`, whose run is measured for
      `padStart`/`padEnd` alike. `Core\Str::repeat("x", 1000000000000)` prints *memory allocation
      of 1000000000024 bytes failed* and exits 127 today.
- [ ] **`Core\Bytes::fill`, `::repeat` and `::join`** — `crates/mwl-stdlib/src/bytes.rs:622`
      (`vec![octet; length]`), `:634` (`subject.repeat(times)`) and `:695`. All three go through
      that file's own `affordable` wrapper at `bytes.rs:356`, which is where their member name is
      already formatted.
- [ ] **`Core\Arr::fill`** — `crates/mwl-stdlib/src/arr.rs:1317`. An entry is a tagged value rather
      than an octet, so what it reserves is `MwlArray`'s own capacity; check whether that path has a
      fallible spelling before writing the guard, and say so in the plan if it does not.
- [ ] **One conformance case for the agreement** — every count-shaped allocator refuses at the same
      seam, in `RuntimeError`, naming its own member, counted over the whole set rather than row by
      row. Model:
      `tests/conformance/core/random-an-oversized-draw-names-the-check-that-refused-it.mwlt`.

## Backlog

- The differential gap is 8 members and the gate is already met, so it is off the frontier;
  `Core\Path`'s three are the largest block — `path.rs:447`, `:478`, `:664` — plan's `Open now`.
- `Core\Math::gcd`/`lcm` have no callable twin on either leg (neither `php` has `gmp`) — plan's
  `Open now` says what an oracle case would have to do instead.
- `Core\Json::decodeAs` refuses a fieldless attributed class with a sentence that is wrong about
  why — plan's `Open now`.
- `Core\Regex\Match::group`'s out-of-set key throws `RuntimeError` where the call-site-argument rule
  argues for `LogicError` — plan's `Open now`.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir` gap 1.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
