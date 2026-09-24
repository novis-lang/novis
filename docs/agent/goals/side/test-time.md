---
milestone: post-parity
---

# Side goal — a check runs again only when something it reads has changed

When this goal is green, every check the loop, `verify.py` and `dossier.py` run is keyed on what it
really reads, and nothing else. An edit inside a `#[cfg(test)] mod tests` block re-runs that crate's
tests and clippy, and nothing that runs the `nvs` binary. An edit to a reference card re-runs what
prints cards, and nothing that only runs programs. The fuzz target, ThreadSanitizer and the database
matrix re-run when their own crates change. `dossier.py` runs every floor group in one process, its
proofs are remembered across sweeps, and a session blesses an example without waiting for a
thin-LTO release link. No check is dropped, and none runs less often than its inputs change.

A table of synthetic edits, `tools/data/impact-probes.toml`, is the proof. It names, for each edit,
what must run again **and** what must not, and `python tools/impact.py --probe` holds every tool to
it. The must-run half is what stops a narrower key from hiding a regression.

## Why a side goal

It changes the tools the chain run imports — `tools/loop.py`, `verify.py`, `verify_keys.py`,
`impact.py`, `dossier.py` — and the memo keys every sweep reads. On a branch of its own the chain
keeps walking the dossier goals on the old keys, and the one full re-key happens once, when this
lands. Nothing on the chain waits for it.

## What is on disk today, measured

Measured on 2026-09-24 from `.loop/check-times.ndjson` (3.7 days, 201 sweeps) and the 62 session
transcripts under `.loop/logs/`:

- **Goal-end sweeps cost 23.6 of 28 check-hours.** 53 gate-open sweeps, median 29 minutes, about
  930 checks each, and usually `1 remembered`. The dossier goals' commits change a `mod tests` block
  or a card const in `crates/nvs-stdlib/src/*.rs`, and that re-keys nearly every check.
- **`dossier.py` is 40% of a session's wall clock**, and `verify.py` is 16%. A `--bless` takes 1 s,
  or 120-330 s when it first links a release `nvs`.

What causes it, by file:

- `crates/nvs-cli/src/script.rs:2505` `units_held_stay_bounded_after_ten_thousand_edits` makes
  10,000 compiles in a debug build (`EDITS`, `:2513`). It takes 315 s, and it is the tail of every
  `verify.py` run (`.agent-tmp/verify-test-times.json`). It asserts `held() <= 2` after every
  edit, so an unbounded table fails by the third edit. The floor names it by the prefix
  `units_held_stay_bounded_after_` (commit 32a3f19f1), so the name is free to change.
- `tools/loop.py:2190` `plain_crate_test` accepts only `test -p X` and `test -p X --test T`. The
  floor's `test -p nvs-cli --bin nvs <filter>` check therefore runs its own `cargo` and pays the
  300 s a second time, instead of taking `verify.py`'s record for the same binary.
- `tools/verify_keys.py:34` already has a `shipped` tier, which is the code tier without the body of
  an inline `#[cfg(test)] mod`. Only `conformance`, `differential`, `reference` and `extension` use
  it (`:413-417`). A file some `.rs` embeds is raw at every tier (`:353-356`), even when the
  `include_str!` sits in a test module. Seven stdlib files are embedded by their own policy tests
  (`cache.rs`, `cli.rs`, `crypto.rs`, `jwe.rs`, `http.rs`, `http/transport.rs`, `ratelimit.rs`).
- `tools/impact.py:313` keys a test binary's dependencies at the `code` tier, but a dependency is
  compiled without `cfg(test)`. An edit to stdlib's test module therefore re-runs 125 of 180
  binaries (628 of 717 CPU-seconds). `common()` (`:294-301`) puts every embedded file in every key.
- `tools/loop.py:2024-2028` `COMMAND_READS`:
  - It keys the fuzz target on every package `fuzz/Cargo.toml` names, stdlib included. But
    `fuzz/fuzz_targets/prefix.rs:24-26` uses only `nvs-syntax` and `nvs-diagnostics`.
  - It keys ThreadSanitizer on `tools/`, `docs/`, every crate's tests and `OTHER`.
  - It keys the database matrix on `tools/` and every crate's tests.
  - `EDITOR_READS` keys `vsix packages` on raw `crates/`.
- `tools/loop.py:3276` falls back to the whole-tree key when a Python gate spawns anything but `nvs`
  or `git ls-files`. Every `dossier.py --verify` spawns `cmd /c ver`: `platform.system()` inside
  `fingerprint()` (`tools/dossier.py:1526`). So none of the ~90 floor dossier checks is ever
  remembered. Every observed gate's key also holds the whole tree's path set (`PATHS`), so one new
  file re-runs 92 gates. `.loop/check-reads.json` is 73 MB, and it is loaded on every `Goal()`.
- `tools/dossier.py`:
  - `:3501` `collect(entries)` reads the whole roster, whatever `--group` or `--only` scoped. Each
    of the ~90 floor checks is its own process, and they run one after another.
  - `--id` does not narrow `--run` or `--record-perf` (`:3485-3500`). About 50 session commands
    ran the whole roster by mistake, at ~160 s each.
  - `:542-556` `current_binary` rebuilds `cargo build --release -p nvs-cli` (thin LTO,
    `codegen-units = 1`, `Cargo.toml:1029-1033`) whenever any file under `crates/` is newer than the
    binary. That includes a test-only edit.
  - `:501-515` `newest_build_input` walks only `crates/` and four manifests. It misses the files the
    binary embeds from outside `crates/`, which `tools/loop.py:1864-1868` lists: `docs/reference/`,
    `docs/spec/02-php-migration.md`, `LICENSE`, `THIRD-PARTY-LICENSES.txt` and
    `tools/data/php-builtins.txt`. **An edit to a reference chapter leaves proofs judged on a stale
    binary.** That is a correctness bug, and this goal fixes it.
  - `:1425` `binary_key` is the binary's size and mtime, so any relink re-runs all 2,289 remembered
    programs.
- `tools/dossier.py:1593` `impl_hash` hashes the implementing file's text. One comment edit in
  `io.rs` therefore marks every `Core\IO*` member's perf stale. That is what
  `rule:testing/member-perf-ledger` says today, and this goal changes it (Stage 8).
- Reference cards are `const NAME: ClassDoc | MethodDoc | EnumDoc | ParamDoc | ErrorDoc | ... = ...;`
  items in `crates/nvs-stdlib/src/*.rs` (the types are at `registry.rs:1041-2904`). The text is read
  only by `crates/nvs-cli/src/meta.rs`, `agent.rs` and `doc.rs`, by `nvs-lsp`, and by the
  registry's own tests. No running program reads it.
- `verify.py` runs rustdoc over 18 crates to find 5 doctests (`nvs-diagnostics` `diagnostic.rs:5`
  and `lib.rs:7`, `nvs-runtime` `abi.rs:636` and `fmt.rs:25`, `nvs-syntax` `lib.rs:46`). That costs
  32 s. Its pool runs 16 binaries at once, with no `RUST_TEST_THREADS`. Three of 17 sessions went
  red from load alone (from the verify logs, not re-checked): `nvs-host` `watchdog.rs:926`,
  `nvs-stdlib` `http/transport.rs:4117` and `socket.rs:1241`, and `nvs-lsp` `latency.rs:212`.

## Stage 0 — the catch-up

This goal makes the sentences below wrong. The session that changes the behaviour rewrites each one
whole, and not before:

- `tools/verify_keys.py`, the module doc: the tiers, and "embedded is raw everywhere".
- `tools/impact.py`, the module doc § *What a test binary reads*: "each at the code tier".
- `tools/loop.py`:
  - the comments over the partitions, `COMMAND_READS`, `BUILDS` and `EDITOR_READS`;
  - `plain_crate_test`'s doc;
  - the `observed_inputs` doc.
- `tools/dossier.py`, the docs of `RELEASE_STAMP`, `BUILD_INPUTS`, `current_binary`, `binary_key`,
  `impl_hash` and `fingerprint`, and the module doc's section on figure currency.
- `docs/rules/testing/member-perf-ledger.md`, the paragraph that starts "A figure is re-measured
  only when". `python tools/rules.py --render` regenerates the chapter.
- `docs/agent/commands.md`:
  - § *Verifying*;
  - § *What a feature still owes, and the loop that pays it*: which binary a proof runs on, and
    what `--id` scopes;
  - § *Disk*: the proof profile's target directory.
- `crates/nvs-cli/src/script.rs`, the bounded-units test's comment.

## Stage 1 — the floor

Main's carried floor. A side run is always checked against it (`tools/side.py`), and it is never
traded.

## Stage 2 — the bounded-units test, and the driver takes verify's record for every test shape

One file set: `crates/nvs-cli/src/script.rs`, `tools/loop.py`.

- **The test runs 200 edits** and is renamed `units_held_stay_bounded_after_two_hundred_edits`.
  The assertion after every edit stays.
- **`plain_crate_test` also recognises `test -p X --bin B [filter]` and `test -p X --lib
  [filter]`.** The driver answers such a check from `verify.py`'s per-binary record, as it already
  does for the plain forms. A filter narrows only which test names it reads out of that record.
- **`python tools/loop.py --why "<check name>"`** prints how the check's key is made. It prints one
  line `source: <verify record | binary key | observed | package key | partitions | everything>`,
  then the key's inputs, grouped by partition or package. This is the tool every later stage
  proves itself with.

## Stage 3 — the probe table (the contract, written before any key moves)

One file set: `tools/data/impact-probes.toml` and `tools/impact.py`.

- **`python tools/impact.py --probe`** loads the table. For each probe, it applies the edit to an
  in-memory copy of the tree, as `--explain` already moves a file (`_Moved`). It then computes,
  over that copy, the key of every:
  - `verify.py` step (`STEP_READS`);
  - test binary;
  - check in the installed `docs/agent/loop-goal.toml`, through the same `inputs_for` the driver uses;
  - dossier program, and the dossier "release binary is current" answer.
- It prints one line per probe, then `impact probes: every probe holds` or the probes that do not
  hold, and it exits non-zero on any miss.
- **A probe** names an edit (a file, and a synthetic change of one kind), a `rerun` list and a
  `keep` list. Each list entry is a check name, a check-name prefix (`valgrind `, `dossier: `), a
  verify step, or a test-binary name. **Both lists are required.** A probe with an empty `rerun`
  list is refused, because a key that re-runs nothing would pass every `keep`.
- **The probes**, at least these ten:

  | Probe | Edit | Must re-run | Must not re-run |
  |---|---|---|---|
  | `test-module` | a new `#[test] fn` inside `jwt.rs`'s inline `mod tests` | stdlib's lib tests, clippy | other test binaries, doc-tests, `valgrind `, native examples, the `.nvst` suites, fuzz, TSan, the database matrix, `dossier: `, the proof and release builds, vsix |
  | `self-included-test-module` | the same, in `jwe.rs`, which a policy test embeds | the policy test's binary, clippy | everything `test-module` keeps |
  | `card` | one word of a `MethodDoc` `short` in `json.rs` | stdlib's lib tests, the `nvs meta`, `nvs agent` and `nvs doc` checks, `nvs-lsp`'s binaries | `valgrind `, native examples, the `.nvst` suites, the examples and attacks that `dossier: ` runs, fuzz, TSan, the database matrix |
  | `stdlib-code` | a function body in `json.rs` | `valgrind `, native examples, the suites, `dossier: `, the database matrix, cost margins | fuzz, TSan |
  | `syntax-code` | a function body in `nvs-syntax` | fuzz, `valgrind `, the suites | none required |
  | `host-code` | a function body in `nvs-host` | TSan | fuzz |
  | `embedded-chapter` | a line of a `docs/reference/` chapter that `nvs-cli` embeds | the checks that print it, and dossier's "binary is current" answer turns to no | fuzz, TSan |
  | `driver-tool` | a line of `tools/loop.py` | the gates that read it | `valgrind `, the suites, TSan, fuzz, the database matrix, `dossier: ` |
  | `new-example` | a new `.nvs` under one member's `docs/examples/` folder | that member's dossier group | every other `dossier: ` group, the observed gates that did not read that folder |
  | `editor` | a line under `editors/vscode/src/` | vsix | `valgrind `, the suites; a `crates/` edit does not re-run vsix |

  Before Stage 4, most `keep` cells fail. That is the red frontier.

## Stage 4 — one key for what the binary is built from (the keystone)

One file set: `tools/verify_keys.py`, `tools/impact.py`, `tools/loop.py`, `tools/dossier.py`.

- **One function** in `verify_keys` (or `impact`) returns the key of what a binary is built from:
  - its cargo closure, with each dependency at the `shipped` tier;
  - its own package raw, only for the test binary built from that package;
  - the toolchain and the manifests;
  - the files embedded **at an include site outside a test module**. An `include_str!` inside a
    test module is an input of that test binary alone.

  `verify.py`, `impact.py`, `loop.py` and `dossier.py` all call it, and no second copy of the
  rule exists.
- **`impact.py:313`** keys dependencies at `shipped`. `common()` holds only the embedded files of
  the binary's own closure.
- **`COMMAND_READS` rows name what each command builds and opens:**
  - fuzz: the closure of the crates the target's source `use`s, plus `fuzz/**`. A target that
    reaches a crate some other way is keyed on the whole manifest, the wide direction.
  - TSan: the `-p` packages of `tools/tsan.sh` (raw, since their tests run), their closure at
    `shipped`, `tools/tsan.sh`, and `examples/`.
  - the database matrix: its packages the same way, plus `tests/db/` and `tools/db-matrix.py`.
  - abi-probe: drops `crate-tests`. `EDITOR_READS` becomes `editors/`.

  `docs`, `tools` and `OTHER` stay only where a grep shows the command opens them. The row's
  comment names the grep.
- **`dossier.py` decides "the binary is current" by this key, never by mtime.** The stamp holds the
  key, and cargo is asked only when the key moved. `binary_key` is this key too, so a relink that
  changed nothing re-runs nothing. This fixes the stale-binary bug above.
- **The one full re-key** that follows is accepted. No memo or ledger is migrated for it.

## Stage 5 — a card edit re-runs only what prints cards

One file set: `tools/verify_keys.py` and the checks' key choices in `tools/loop.py` and `tools/dossier.py`.

- **A tier below `shipped`** blanks the initializer of every `const` whose type is one of the
  registry's card types (`ClassDoc`, `MethodDoc`, `ParamDoc`, `ShapeKeyDoc`, `ErrorDoc`, `EnumDoc`,
  `CaseDoc`, and any `*Doc` added later, found by type name). It also blanks a slice of them. A card
  written inline anywhere else is left in the tier, the safe direction.
- **These checks key on it:** the checks that only run programs — `valgrind `, native examples,
  fixtures, the `.nvst` suites, the examples and attacks that dossier runs, fuzz, TSan and the
  database matrix. Everything else keeps `shipped`. That includes every check that runs `nvs meta`,
  `nvs agent`, `nvs doc` or `nvs lsp`, cost margins, and every test binary.
- **The cards stay where they are.** `rule:core-api/reference-card` is unchanged.

## Stage 6 — dossier runs as one pass

One file set: `tools/dossier.py`, and the dossier half of `tools/loop.py`.

- **`--verify` takes several `--group` values**, builds the roster and `collect` once, and runs the
  programs of every named group in one pool. It prints each group's verdict lines exactly as a
  single-group run prints them today.
- **The driver runs all the dossier checks a sweep owes as one `dossier.py` call.** It gives each
  check the verdict lines of its own group and remembers each check on its own key. The check names
  and `want` lines on the floor do not change.
- **No spawn but `nvs`.** `fingerprint()` is computed only where perf is recorded, and
  `impl_hash` is cached per path. `loop.py --why "dossier: lang:types"` prints `source: observed`.
- **`--id` narrows** `--run`, `--verify` and `--record-perf` to that one feature, as `--only` does.

## Stage 7 — proofs run on a fast build during a goal, on the release build at its end

One file set: `Cargo.toml`, `tools/dossier.py`, `tools/loop.py`, `tools/disk.py`.

- **`[profile.proof]`** inherits `release` and sets `lto = false`, `incremental = true` and
  `codegen-units = 256`. Overflow checks stay on and debug assertions stay off, as in release.
  Exactly one command builds it, named once in `dossier.py` beside `RELEASE_BUILD`.
- **`--bless`, `--run`, and `--verify` in a session and in a scoped sweep** run on the proof binary.
- **The release binary runs:**
  - `--record-perf`;
  - the cost-margin guards;
  - every gate-open sweep's dossier checks, so the floor gate and goal end are judged on release.
- **`tools/disk.py`** counts `target/proof/`. Its size is stated in the profile's comment.

## Stage 8 — the smaller cuts

One file set per bullet.

- **The driver takes verify's `conformance` and `differential` verdicts** when their keys match.
- **`PATHS` enters a key only for a gate that ran `git ls-files`.** `.loop/check-reads.json` is
  stored in a form small enough to read in well under a second.
- **`doctest = false`** is set in every crate with no doctest. A `tools/lints.py` check fails when
  one of those crates gains a runnable doctest.
- **Load does not fail a test.**
  - The four tests named above wait on a condition, never on a wall-clock bound.
  - `verify.py`'s pool sets `RUST_TEST_THREADS` so that pool width times threads is about the
    machine's cores.
- **Perf currency ignores comments, layout and cards.**
  - `impl_hash` hashes the implementing file at the Stage 5 tier.
  - The same commit rewrites every `impl_hash` in `docs/perf/members.ndjson` whose old-form hash
    matches the file on disk, so no figure is re-measured for the change of form.
  - `rule:testing/member-perf-ledger`'s currency paragraph is rewritten whole, with a decision record
    in the shape `docs/agent/conventions.md` gives.

## Standing decisions

These are the user's calls, made on 2026-09-24. No session re-decides one.

- **Every narrowed key is proved by `impact.py --probe` and cheap probes. It is never proved by a
  sweep.** Starting `loop.py --settle`, `--goal-only --full` or a full `verify.py` to show that a
  cut works is the mistake the user stopped on 2026-09-23. The side run's own sweeps are the only
  sweeps.
- **The bounded-units test runs 200 edits.** The floor names it by prefix. That was done on main
  before this goal started, so this goal never edits the chain's files.
- **One key for what a binary is built from**, used by all four tools (Stage 4).
- **The cards stay in the registry** (`rule:core-api/reference-card`). A key tier skips them, and
  no card moves to a data file.
- **The proof profile** is the user's choice. Its cost is one more target directory on disk. Its
  risk is that a difference that depends on optimisation shows up only at a gate-open sweep, which
  runs on release.
- **Dossier becomes one process.** Every floor check keeps its name and `want` lines.
- **Perf currency ignores comments.** That reverses the rule's "a comment edit stales its file's
  figures". It is recorded in a decision record, and the ledger is migrated in the same commit.
- **Nothing is dropped and no gate frequency changes.** `FLOOR_GATE_EVERY`, `AUDIT_EVERY`, the
  heavy legs and the fuzz campaign all stay. A narrower key is the only lever.
- **The tradeoffs, stated once.**
  - Security and correctness: none lost. Every key is at least as wide as what the build reads, the
    probe table's `rerun` cells hold it there, and one bug that could hide a regression (the stale
    dossier binary) is fixed.
  - Latency: none on any request path; this is tooling.
  - Memory: none at run time.
  - Disk: one proof target directory.
  - Simplicity: one key function in place of five tables, and one dossier process in place of
    ninety.
