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
change itself reaches, and the loop judges every selected bench, owed, red and new ones included,
whenever its floor gate is open. `tools/nv/select/` is the one engine all four ask, `select.ts` there holds the rule and
`keys.ts` the keys, and `bun nv select --explain <atom>` says why one atom was chosen or not.

The footprint is observed rather than predicted because a prediction at crate granularity cannot tell
a function body a program runs from one it does not: an edit to one body re-ran thousands of cases,
and every narrowing of it was a claim someone had to prove and remember. An atom's footprint is the
union of its runs since its definition last changed, pruned of items that no longer exist, so a path
a run takes only sometimes is kept once it has been seen. Footprints are kept per platform, because
what an atom does differs between Windows and a Linux leg, and `bun nv select --seed` records every
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

The heavy checks, which build the release profile, fuzz, TSan, the database matrix or run a Linux leg,
record no coverage of their own. A release test, a bench and the database
matrix are keyed on a **twin**: the same work run again on `covws`, whose verdict counts for nothing and
whose coverage is the check's keys. The twin of a release test runs its tests with `--include-ignored`,
a bench runs once on the covws `nvs`, and the database matrix runs its SQLite leg. Each also holds a key
for what its twin cannot reach: `profile:optimized` for release code, and every file of `nvs-db` for the
server drivers. A Linux leg runs every fixture and suite of the plan again, so it is selected whenever
one of those atoms is. Code only Linux compiles moves `platform:elsewhere`, which the legs, the database
matrix, fuzz and TSan hold. Fuzz and TSan build inside WSL and have no twin: they are keyed on every
item and directory of the crates they build, the one predicted key left. A twin that cannot run or be
read records `*`, which every change reaches until a twin is read again. A check with `memoize = false`
is never answered from the store, and
`bun nv loop --goal-only --full` runs every atom of every check.

A heavy check keyed on every file of every crate it builds would be reached by almost any Rust change,
so a goal that names them would pay for all of them after nearly every session; the twin is what keeps a
change that reaches none of its code from running it. A twin is paid only when its heavy check runs.

**Every open floor gate runs both Linux legs over every fixture and suite of the goal**, whatever
changed (`tools/nv/cmd/sweep.ts`, through `bun nv legs --full`'s `legsOver`). A leg records no footprint
of its own and is selected through its fixtures' atoms, so this run is the safety net for what that
selection cannot see. The gate runs no other heavy check. CI is their safety net: its nightly deep lane
runs the release profile, fuzz, TSan and the database matrix whatever changed, and a push runs each one
its diff reaches (`rule:testing/ci-lanes`).

**The full run is the safety net.** `bun nv select --full` runs and records every atom that is not
heavy, whatever changed, and the loop runs it when the floor gate opens by its count. An atom red there
that the selection since the store's tree did not pick is a selection miss: it is printed, kept in the
store until a full run finds the atom green again, and fails the run. `bun nv select --mutate tools/data/select-mutations.json` is the proof that the keys lose
nothing: it applies one batch of breaking edits at a time, runs every atom that is not heavy against a
copy of the store named by `NV_SELECT_STORE`, requires every red atom to have been selected, and
reverts the edits.

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
- **Keying a Linux leg on a copy of its fixtures' footprints.** The copy goes stale between leg runs
  when a fixture's own footprint grows. Selecting the leg through the fixtures' own atoms reads the
  live footprints.
- **Running a Linux leg over only the fixtures the change reached.** A fixture reached while the floor
  gate is shut would need a Linux owed state of its own. The leg's cost is fixed, so what is cut is how
  often it runs.
- **Running the Linux legs at the floor gate only when the store says the change reached them.** A
  leg's selection is read through its fixtures' atoms, so a regression that selection misses would
  never be run on Linux. The gate opens one session in `FLOOR_GATE_EVERY`, so both legs cost a fixed
  run at that interval.
