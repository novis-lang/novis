The acceptance sweep, `bun nv verify`, `bun nv affected` and `bun nv proofs` start only the atoms a
change reaches, read from what each atom's green runs were observed to use. An atom is the smallest
thing that runs on its own: one `.nvst` case, one proof program, one Rust test binary, one `bun nv`
command, one tools test file, one other plan check. Every run of one records its footprint in
`.cache/select.sqlite`: the Rust items that ran, from coverage, and the `Core` classes, cards, files,
directories and paths it looked up, from the log `nvs` and the test binaries write. A selection turns
the change since the recorded tree into keys and looks the atoms up under them. An atom that is new,
last red, owed or diverged runs too, and so does every atom when a global file changes. Nothing else
starts. `tools/nv/select/` is the one engine all four ask, `select.ts` there holds the rule and
`keys.ts` the keys, and `bun nv select --explain <atom>` says why one atom was chosen or not.

What coverage cannot see is closed over the reference graph `tools/nv-scan --items` reads: a const, a
static, a type or a table moves every item whose name Rust can resolve to it, which in another package
takes an import or a qualified path and in shipped code no dev-dependency, a `macro_rules!` every item
that invokes it,
a class table's changed row that row's class, and a card only what prints it. A doc-comment edit moves
nothing. **Anything that cannot be attributed widens**: an unparseable file, an unmapped function or an
unreadable record selects more, never less.

The pipeline's one debug build is `covws` (`target/covws`, built by `tools/nv/lib/covws.ts` through
`tools/covwrap`), which instruments the workspace's own crates and nothing else. Every debug `nvs` and
test binary the sweep, verify, `bun nv try` and `bun nv reference` run is that build, handed to checks
as `NVS_BIN`; `target/debug` is what a person builds by hand. A recorded run starts with an empty compile
cache. A proof program is judged on the uninstrumented `target/proof` build and recorded in a second run
on `covws`, which is never a verdict; when the two runs end differently, the program always runs until
they agree.

The heavy checks, which build the release profile, fuzz, TSan, the database matrix or run a Linux leg,
wait for the floor gate, and record no coverage of their own. A release test, a bench and the database
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

**Every third goal end runs every heavy check and both legs**, whatever changed. That is the safety net
for the twins' keys. The goal-end sweeps between those run what their goal reached.

**The full run is the safety net.** `bun nv select --full` runs and records every atom that is not
heavy, whatever changed, and the loop runs it when the floor gate opens by its count. An atom red there
that the selection since the store's tree did not pick is a selection miss: it is printed, kept in the
store until a full run finds the atom green again, and fails the run. `bun nv select --mutate tools/data/select-mutations.json` is the proof that the keys lose
nothing: it applies one batch of breaking edits at a time, runs every atom that is not heavy against a
copy of the store named by `NV_SELECT_STORE`, requires every red atom to have been selected, and
reverts the edits.
