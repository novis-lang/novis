# ADR 0134 — Every shipped feature owes four proofs, and the roster of features is derived rather than kept

- **Status:** Accepted
- **Date:** 2026-09-02
- **Scope:** what a feature owes before it counts as finished — a test from both sides, three
  real-world examples, one measured performance figure, one attack — where each lives, and how a
  loop is generated that produces them. It does **not** decide what any individual test asserts
  ([docs/agent/conventions.md](../agent/conventions.md) owns the shape of a case), how the loop is
  driven ([docs/agent/coordinator.md](../agent/coordinator.md)), how Novis is measured *against
  other engines* ([benches/userland/README.md](../../benches/userland/README.md)), how the
  compiler's own historical performance is tracked ([0026](0026-performance-measurement-methodology.md)),
  or what the website renders ([website/README.md](../../website/README.md)).
- **Depends on:** 0079, 0117
- **Validated by:** `tools/dossier.py`, `tests/hostile/README.md`, `benches/members/README.md`,
  `docs/examples/README.md`

> **In short:** a feature is finished when four artefacts exist for it, not when it works. **(1)** Its
> behaviour is pinned from Novis *and* from Rust; **(2)** three small self-contained examples show a
> reader what it is for, in `docs/examples/`, mirrored to the website; **(3)** one program in
> `benches/members/` gives it a measured cost, appended to `docs/perf/members.ndjson` with a machine
> fingerprint — Novis against itself, never against PHP; **(4)** one program in `tests/hostile/` is
> written to break it, and passes when the runtime is still standing. **The roster of features is
> derived from `nvs meta --json` and the reference chapters, never kept as a list**, so a feature
> that ships is owed these on the next sweep and one that is deleted stops being owed with nobody
> editing anything. `python tools/dossier.py` audits, gates, runs and measures all of it, and
> `--emit-goals` writes the unattended loop that produces what is missing. What makes this
> affordable to re-run is that the gate executes nothing: a figure is re-measured only when the
> commit that last touched its implementation changes, and a green run is remembered against the
> bytes that produced it.

## Context

- The repository already proves a great deal — 1,678 conformance cases, a differential suite against
  PHP, guard tests with named thresholds, a valgrind sweep, a cross-engine benchmark suite — but
  **nothing relates any of it to a feature**. `tools/gaps.py` can say a class is thin and
  `tools/holes.py` can say a shape is refused; neither can answer "is `Core\Str::length` finished",
  because *finished* was never written down.
- The four things a feature was informally expected to have — tests, examples, a performance figure,
  an adversarial case — were each owned by a different tree with a different convention, and three of
  the four had no tree at all. Examples existed for 8 of 532 members. No member had a recorded cost.
  No member had an attack.
- A list of features would rot on contact. The registry grows every session of an unattended run; a
  hand-kept roster is wrong within a day, and a roster that is wrong silently under-reports, which is
  the failure mode that looks like success.

## Decision

### 1. Four proofs, and a feature is not finished until it has all four

| Proof | Where | What it is |
|---|---|---|
| **tests** | `tests/conformance/`, `tests/differential/`, `crates/**/src/**.rs` | the behaviour pinned from Novis *and* from Rust. A `Core` member owes at least one of each |
| **examples** | `docs/examples/<path>/` | three small, self-contained, plainly-commented programs a reader learns from |
| **perf** | `benches/members/<path>.nvs` + `docs/perf/members.ndjson` | one measured figure, so a change can be re-measured |
| **hostile** | `tests/hostile/<path>/` | one program written to break it |

Each tree's own README owns what a file in it *is*; `tools/dossier.py --help` owns how each is run.
Neither restates the other, and this ADR restates neither.

**Not every kind of feature owes all four.** An enum is not attacked and a directive is not
benchmarked. What each kind owes is data — `POLICY` in `tools/dossier.py`, overridable per feature in
`tools/data/dossier-policy.toml` — because a policy in prose is a policy nothing can check. A single
feature excused from a single proof is a `[skip]` entry carrying **the reason as its value**, so
"this cannot be measured" and "nobody wrote one" never look the same in the audit.

### 2. The roster is derived, never kept

Four live sources, and no list anywhere:

- **`nvs meta --json`** — every registered class, member, exception, enum, interface and `nvs.toml`
  directive. It is the registry's own answer ([0117](0117-an-implemented-core-member-documents-itself-in-the-registry.md)),
  so a member that is not implemented is not owed proofs and a member that lands is owed them at
  once.
- **`docs/reference/lang/*.md`** and **`docs/reference/tools/*.md`** — every `#` heading is one
  language or tool feature. Those chapters are already the hand-written half of `docs/novis.md` and
  already state every rule the shipped compiler has, which makes their headings a roster that
  maintains itself.

A source that stops naming a feature stops owing proofs for it, and one that starts naming a new
feature owes them on the next sweep. Nobody edits a list, so no list is ever stale.

### 3. Attribution is by path for three proofs and by a marker for the fourth

An example, an attack and a bench are attributed **by where they sit**: all four trees share one
relative path per feature, so `docs/examples/core/Str/length/`,
`tests/hostile/core/Str/length/` and `benches/members/core/Str/length.nvs` need no registration.

A test cannot work that way — a case lives where its suite wants it, and one case often pins several
features — so a test is attributed by a comment naming what it covers:

    // covers: Core\Str::length, lang:expressions/precedence-and-associativity

in the `--FILE--` block of a `.nvst` case, or above a Rust `#[test]`. For a `Core` member the scan
additionally credits a case that plainly calls it (`Core\Str::length(`), which is what lets the 1,678
cases written before this ADR count without being rewritten. The written `::` spelling is the only
inference: crediting `->length(` to every class a case happens to name is unsound rather than merely
loose, and no tightening fixes it without a type checker.

### 4. The perf figure is Novis against Novis, fingerprinted, and re-measured only when the implementation moves

There is no PHP column in `docs/perf/members.md` and there will not be one; that comparison is
[benches/userland](../../benches/userland/README.md)'s and stays there. This ledger answers one
question: *we changed something — what did it cost?*

- **`ns/op`** is wall clock, with the empty program's start-up floor subtracted.
- **`units`** is that figure divided by a fixed calibration program measured in the same sweep.
- **Every record carries a machine fingerprint**, and the report **refuses to print a delta across
  two of them**. Wall clock is not comparable across machines — that is [0026](0026-performance-measurement-methodology.md)'s
  whole finding — and `units` divides out the clock speed well enough to travel to about a tenth,
  which is enough to see a 3× regression and not enough to claim 5%.
- **Nothing here gates a build.** A regression is a row with a `Δ` on it. The guards that fail a
  build are `benches/abi-probe/tests/perf_guards.rs` and stay exactly as they are.

**A figure is re-measured only when the commit that last touched its implementing file changes.**
That currency rule is what makes a roster of hundreds affordable: change `crates/nvs-stdlib/src/str.rs`
and every `Core\Str` figure goes stale at once; change anything else and nothing is re-measured.

Callgrind instruction counts — 0026's cross-machine headline — are deliberately **not** what this
ledger records. They are Linux-only and about fifty times slow, so a sweep of every feature is an
hours-long run on one platform, and this ledger has to be cheap enough that it is actually taken.
The two measurements answer different questions and 0026 § 1 already keeps two mechanisms apart for
exactly this reason.

### 5. A hostile case is judged by a contract, not by frozen output

Its assertion is that nothing came apart. A program that throws, one a limit stops, one that runs out
of memory and says so, and one that simply works are all passes; a panic, an abort, a hang past its
timeout, a crash-shaped exit status and a definite leak under valgrind are not.

**The one failure that would otherwise look like a pass is a compile diagnostic**, and it is checked
for by name: an attack that does not compile was never delivered, and a typo would otherwise survive
every sweep for the rest of this repository's life — [loop-authoring.md](../agent/loop-authoring.md)
§ 3's "a green suite is not a run guard", in a new place. Where the refusal *is* the assertion — a
sink handed a tainted value, a capability used without being granted — the case says
`// hostile: expect-refusal`, and compiling cleanly is then what fails it.

Freezing the output instead was rejected: every one of these programs is written to produce output
nobody can predict, and a suite whose expectations must be maintained is a suite that gets weakened
until it passes.

### 6. One slice is one feature, not one proof

The unattended loop takes a feature and writes **all four proofs together**. The expensive thing a
session buys is understanding what the feature does at its edges, and the test, the examples, the
bench and the attack all spend that same understanding; split across four sessions it is bought four
times, against a fixed cost of 35 of a session's 62 calls
([AGENTS.md](../../AGENTS.md) § *Session workflow*).

`python tools/dossier.py --emit-goals` writes the chain: one goal per group of features sharing an
implementing file set, each with a `[context]` manifest naming that file set, each gated by commands
that exit non-zero. Regenerating it is how it stays current — a group that owes nothing is left out,
so a second emission writes the chain that is *left*.

### 7. The examples are the website's copy, and they live in the repository

`docs/examples/` is authoritative and `website/examples/` is a mirror rebuilt by
`npm run sync:examples`. The site's own rule — tool-owned files are regenerated, human-owned files are
never overwritten — is unchanged; this simply makes the example tree one of the tool-owned ones. The
reason it lives in the repository is that the same sweep that tests a feature writes its examples, and
a sweep cannot write into a tree it is not allowed to touch.

## Consequences

- **Something can now be finished.** "Is this done" has a command and an exit code, per feature and
  per group, where before it had an opinion.
- **A cost is spent up front, per feature, forever.** Three examples and an attack are real work, and
  there are hundreds of features. That is the trade this ADR makes deliberately: it is priority 2
  (correctness) and priority 1 (a hostile case is a security question) bought with time, and the loop
  is what spends it.
- **The gate stays cheap to re-run**, which is the property that decides whether it is run at all: it
  executes nothing, a green `--run` is remembered against the bytes that produced it, and a figure is
  re-measured only where the implementation moved. A second sweep over a finished tree is a walk of
  the directories.
- **The audit will read as mostly-red for a long time**, and that is the honest number rather than a
  reason to weaken the policy. `--no-perf` exists for the case where one proof is deliberately not
  being pursued yet; it changes what is owed and says so in the audit, and it does not delete
  anything.
- **Two new trees and one moved one.** `tests/hostile/` and `benches/members/` are new;
  `website/examples/core/` became `docs/examples/core/` and is mirrored back.

## Alternatives rejected

- **A hand-kept checklist of features.** Rots within a day of an unattended run, and under-reports
  silently. The registry already knows what ships.
- **One proof per session, four sessions per feature.** Buys the same understanding four times; § 6.
- **Coverage instrumentation instead of a Rust-side test requirement.** `llvm-cov` over this
  workspace answers "which lines ran", not "which feature is proven", and a line-coverage percentage
  is exactly the kind of threshold [loop-authoring.md](../agent/loop-authoring.md) § 3 warns is not a
  milestone.
- **Frozen output for hostile cases.** § 5.
- **Callgrind as the per-feature figure.** § 4.
- **Examples in the website repository, written by a human afterwards.** They were, for 8 members of
  532, over the life of the project to date. The sweep that understands the feature is the one that
  should write them.

## Verification

- `python tools/dossier.py` prints the audit; `--gate` exits non-zero naming what is owed;
  `--run examples` and `--run hostile` execute what is on disk and exit non-zero on the first
  failure; `--record-perf` appends to the ledger and `--perf-report` renders it.
- `Core\Str::length` is the worked example and is complete in all four: a Rust `#[test]` carrying its
  marker plus 123 conformance cases that call it, five examples under
  `docs/examples/core/Str/length/`, `benches/members/core/Str/length.nvs` with a recorded figure, and
  `tests/hostile/core/Str/length/01-unbounded-and-degenerate-input.nvs`.
- `python tools/dossier.py --emit-goals` writes a chain `python tools/loop.py --list` reads, so the
  generated goals are validated against the driver's own schema rather than by inspection.
