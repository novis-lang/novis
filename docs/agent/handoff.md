# Handoff

## State

**Goal 4, M8.** The differential suite's `min_passing = 250` gate
(`docs/agent/loop-goal.toml:2630`) — the driver's one failing acceptance check for the last two
sessions — **is met: `python tools/gaps.py` reports the suite holds 250**, up from 246, with this
session's four new `tests/differential/core/hash-*` cases. Nothing is blocked, and no other
acceptance check has been seen to fail; the driver stops at the first failure, so whether a check
*after* this one has ever run is only answerable from the next iteration's ledger line. If one
fails, it outranks the group below.

`python tools/gaps.py`'s **differential gap list is still empty** — every member with a PHP twin has
an oracle case — so remaining differential work is depth, and the four cases this session added are
that: `equals` against `hash_equals` over one-octet and length differences; `hmac` across all six
block widths in the strong subset, with RFC 2104's zero-extension collision asserted on both sides
of the boundary; `of` across every padding and length-field boundary; and `Stream` given an empty
`update` before, between and after every real one, for all fourteen algorithms PHP can compute. All
four agree with PHP exactly — no `--ORACLE-DIVERGES--` finding in the class.

**A `Core\Digest` case is a first-class value in a `.nvst` case**: `array<Core\Digest> $algos = […]`,
`foreach ($algos as Core\Digest $algo)` and `Core\Hash::of($s, $algo)` all compile, so a roster sweep
no longer needs the one-branch-per-algorithm shape the older hash cases use.

## Next group

**Conformance depth, not differential** — the gate above is closed. `python tools/gaps.py` ranks
`Core\IO\Metadata` thinnest of all 67 classes (depth 1.0, floor 1: `isDir` 1, `isFile` 1,
`modifiedAt` 1). All three slices share one module — `crates/nvs-stdlib/src/io.rs` — and one file
set under `tests/conformance/core/io-*.nvst`; read a landed `io-` case first for how a case creates
and cleans up a file it can then stat.

- [ ] **`Core\IO\Metadata`'s four accessors agree with `Core\IO`'s three free members** (~1 case).
      The *agreement* shape: one `stat` handed a regular file and a directory, `size`, `modifiedAt`,
      `isFile` and `isDir` read off it, and each compared to the free member answering the same
      question about the same path — a class that grew its own answer fails here while every line
      still reads plausibly. `crates/nvs-stdlib/src/io.rs:1806`,
      `crates/nvs-stdlib/src/io.rs:3053`, `crates/nvs-stdlib/src/io.rs:3096`,
      `crates/nvs-stdlib/src/io.rs:3115`.
- [ ] **`Core\IO\Metadata::size` and `::modifiedAt` at their edges** (~1 case). An empty file is
      `size` 0 rather than absent; a rewritten file's `size` follows the rewrite; a directory has a
      `size` at all. `modifiedAt` is an `Instant`, so what a case can pin without a clock is that
      re-statting an untouched file answers the same one.
      `crates/nvs-stdlib/src/io.rs:3053`, `crates/nvs-stdlib/src/io.rs:3061`,
      `crates/nvs-stdlib/src/io.rs:2987`.
- [ ] **`Core\IO::isFile` and `::isDir` over a path that is neither** (~1 case). The *edges* shape:
      a name that does not exist, and a path whose parent component is a regular file — both answer
      `false` rather than throwing, which is the claim one existing case each cannot make.
      `crates/nvs-stdlib/src/io.rs:152`, `crates/nvs-stdlib/src/io.rs:161`,
      `crates/nvs-stdlib/src/io.rs:3096`, `crates/nvs-stdlib/src/io.rs:3115`.

## Backlog

- `Core\Http\Response::status` and `::text`, 3 cases each — `gaps.py`'s second-thinnest class.
- `Core\Mail::send` at 3 cases, its floor — `docs/adr/0082-the-first-party-framework.md` § 2.
- `Core\Env::mode` 3 and `::all` 4, and `Core\Cldr::pluralCategory` 4 — `gaps.py`, thinnest members.
- Five unasserted `thrown` paths a case could catch: `crates/nvs-stdlib/src/csv.rs:610`,
  `crates/nvs-stdlib/src/env.rs:379`, `crates/nvs-stdlib/src/command.rs:390` — `gaps.py --errors`.
- `Core\Hash\Stream` is now 5 cases per member; the class's remaining depth is `update` over `bytes`
  rather than `string`, if the row admits it — `crates/nvs-stdlib/src/hash.rs:928`.
