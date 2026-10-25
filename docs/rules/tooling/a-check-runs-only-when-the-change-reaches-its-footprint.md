The acceptance sweep, `bun nv verify`, `bun nv affected` and `bun nv proofs` start only the atoms a
change reaches, read from what each atom's green runs were observed to use. An atom is the smallest
thing that runs on its own: one `.nvst` case, one proof program, one Rust test binary, one `bun nv`
command, one tools test file, one other plan check. Every run of one records its footprint in
`.cache/select.sqlite`: the Rust items that ran, from coverage, and the `Core` classes, cards, files,
directories and paths it looked up, from the log `nvs` and the test binaries write. A configuration file
is recorded by part: its global tables, and the paths an `[[app]]` block that applies to the entry file
could be keyed on. An edit of the repository's own `nvs.toml` moves only the parts it changed, and the
test that resolves that file whole runs on every edit of it. A selection turns
the change since the recorded tree into keys and looks the atoms up under them. **A run counts the
moment each atom's run is recorded**, not when the whole run ends: the atom is stamped with the tree the
run's selection was read at, the next selection reads it against the change since that tree, and a
verdict judged and not yet recorded is kept the same way, so a run that is killed loses only the atoms
still running. The run that moves the recorded tree clears the stamps it covers. An atom that is new,
last red, owed or diverged runs too, and so does every atom when a global file changes. Nothing else
starts. A bench is judged on the release build, so `bun nv affected --run` judges only the benches the
change itself reaches, and `bun nv benches --owed` pays the owed ones before a release, where the
`proofs` job is red on a stale perf record.
`tools/nv/select/` is the one engine all four ask, `select.ts` there holds the rule and
`keys.ts` the keys, and `bun nv select --explain <atom>` says why one atom was chosen or not.

The footprint is observed rather than predicted because a prediction at crate granularity cannot tell
a function body a program runs from one it does not: an edit to one body re-ran thousands of cases,
and every narrowing of it was a claim someone had to prove and remember. An atom's footprint is the
union of its runs since its definition last changed, pruned of items that no longer exist, so a path
a run takes only sometimes is kept once it has been seen. Footprints are kept per platform, because
what an atom does differs between Windows and Linux, and `bun nv select --seed` records every
atom from nothing for a new platform or a lost store.

What coverage cannot see is closed over the reference graph `tools/nv-scan --items` reads: a const, a
static, a type or a table moves every item whose name Rust can resolve to it, which in another package
takes an import or a qualified path and in shipped code no dev-dependency, a `macro_rules!` every item
that invokes it,
a class table's changed row that row's class, and a card only what prints it. A row, a class or a `use`
whose only change is its link to a card or its import of a card type moves only what prints that card.
A doc-comment edit moves nothing. **Anything that cannot be attributed widens**: an unparseable file, an
unmapped function or an unreadable record selects more, never less.

The pipeline's one debug build is `covws` (`target/covws`, built by `tools/nv/lib/covws.ts` through
`tools/covwrap`), which instruments the workspace's own crates and nothing else. Every debug `nvs` and
test binary the sweep, verify, `bun nv try` and `bun nv reference` run is that build, handed to checks
as `NVS_BIN`; `target/debug` is what a person builds by hand. A recorded run starts with an empty compile
cache. A proof program is judged on the uninstrumented `target/proof` build and recorded in a second run
on `covws`, which is never a verdict; when the two runs end differently, the program always runs until
they agree.

The heavy checks, which build the release profile, fuzz, TSan or the database matrix, record no
coverage of their own. A release test, a bench and the database
matrix are keyed on a **twin**: the same work run again on `covws`, whose verdict counts for nothing and
whose coverage is the check's keys. The twin of a release test runs its tests with `--include-ignored`,
a bench runs once on the covws `nvs`, and the database matrix runs its SQLite leg. Each also holds a key
for what its twin cannot reach: `profile:optimized` for release code, and every file of `nvs-db` for the
server drivers. Code only Linux compiles moves `platform:elsewhere`, which the database matrix, fuzz
and TSan hold. Fuzz and TSan build inside WSL and have no twin: they are keyed on every
item and directory of the crates they build, the one predicted key left. A twin that cannot run or be
read records `*`, which every change reaches until a twin is read again. A check with `memoize = false`
is never answered from the store, and
`bun nv select --full` runs every atom that is not heavy.

A heavy check keyed on every file of every crate it builds would be reached by almost any Rust change,
so a goal that names them would pay for all of them after nearly every session; the twin is what keeps a
change that reaches none of its code from running it. A twin is paid only when its heavy check runs.

**The leak check is CI's nightly job, not the loop's.** `bun nv valgrind` runs every `examples/`
program under memcheck in the deep lane (`rule:testing/the-deep-lane`), and CI's Linux jobs run every
test on Linux at each push. A run in the loop would be keyed on a goal's fixtures, and a goal that names
none would run nothing: the nightly job names its own programs. The debt run runs no heavy check.
Looper's sweep runs the heavy checks the live goal names, and a push to CI runs each one its diff
reaches (`rule:testing/ci-lanes`).

**The floor debt is paid once per goal, and nightly CI is the safety net.** An atom a change reached
that a run did not start is owed, and it stays selected until a run of it is green. `bun nv debt` is
the floor check `floor-debt` in `docs/goals/README.md`, with `reach: true`, so looper judges it last
and only once every stage is done and every other check is green. It runs the owed atoms, oldest
first: proof programs, cases, test binaries, tools tests and plan checks, and the red proof programs,
which `nv verify` never runs again. It pays all of them, and is red while any stays red. It leaves the benches to a release, because a goal's owed
benches alone took over an hour to pay. No session pays debt,
and nothing pays it between two sessions, so a red in an owed atom can stay unseen for up to one goal.
Nothing in the loop runs every atom whatever changed, so a selection miss shows up in CI's nightly deep
lane, which runs every job whatever changed. The hook after each session, `bun nv sweep`, reads the
newest nightly run back and writes each failed job beside looper's own reds in `.loop/last-sweep.md`.

`bun nv select --full` runs and records every atom that is not heavy, whatever changed, when a person
runs it. An atom red there that the selection since the store's tree did not pick is a selection miss:
it is printed, kept in the store until a full run finds the atom green again, and fails the run.
`bun nv select --mutate tools/data/select-mutations.json` is the proof that the keys lose nothing: it
applies one batch of breaking edits at a time, runs every atom that is not heavy against a copy of the
store named by `NV_SELECT_STORE`, requires every red atom to have been selected, and reverts the edits.

- **Memory:** nothing at run time; this is the development machine's disk, never a request's. The
  store and the coverage-map cache under `.cache/select/covmap/` are per machine and git-ignored, and
  grow with the number of atoms and builds, never with the number of runs. Each run's raw profile is
  deleted as it is read. The `covws` build takes the place of the pipeline's `target/debug`, at about
  the same size, and a selected proof program runs twice.

**Declined:**
- **Refining the static keys with finer tiers, more partitions and more exception lists.** Each
  refinement moved the over-selection somewhere else rather than removing it.
- **A static call graph read with `syn`.** Dynamic dispatch, the registry's lookup by name, JIT code
  calling Rust helpers through `no_mangle` symbols and closures passed across crates all hide the
  edges that matter, so the graph is unsound exactly where the runtime is interesting.
- **Judging proof programs on an instrumented optimized build.** Instrumentation slows an attack
  severalfold and some far more, which changes what an attack's time limit means. Recording on a
  second debug run costs no build and leaves the verdict on the uninstrumented one. A perf guard's
  margins would change meaning the same way, so no release test is recorded on an instrumented build.
- **A WSL leg and a valgrind sweep over the live goal's fixtures, paid as floor debt.** The goals'
  checks name no fixture, so both legs ran nothing while the debt run reported them as waiting. CI's
  Linux jobs run every test on Linux, and the nightly `valgrind` job names its own programs.
- **A full run in the loop every few sessions, whatever changed.** That run re-recorded
  every atom and judged every bench, and it held the loop for far longer than the debt the sessions
  between left. Paying the debt a fixed budget at a time costs each session the same bounded time. The
  local store is never re-recorded whole, so a selection miss is caught by nightly CI rather than here.
