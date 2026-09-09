---
milestone: post-parity
---
# Loop goal 9 — `rule:core-classes/temporary-dir-sweep`'s temporary-directory sweep

Land `rule:core-classes/temporary-dir-sweep`:
every directory `Core\IO::temporaryDir` hands out dies with its script, swept by the runtime without ever
throwing; a dead process's leftovers are reclaimed at `nvs serve` boot and under `nvs tmp clean`, keyed on
owner liveness and never on age. The ADR is **written and accepted** — this goal implements it and does
not reopen it. It sits after the parity program because the post-response sweep and the boot sweep both
live in `nvs-server`, which goal `server` creates.

This goal is small by chain standards: one design, five crates touched at one seam each. Its floor is the
whole parity program, which is most of what its acceptance list weighs.

## Two things every session must hold

**The sweep never throws and never runs user code.** A path already gone is the goal state; a refused
deletion is one log line, retried by whichever sweep comes next (0131 § 3). The sweep is native teardown
placed *after* the last user code — after [0127](../../decisions/0127.md)'s
`onExit` queue on a CLI ending, after [0072](../../decisions/0072.md) § 6's
`afterResponse` work on a request — and off the request path. `Core\IO::remove`/`removeDir` keep
throwing; only the automatic sweeps are silent-but-logged (0131 § 6).

**The orphan sweep's predicate is owner liveness, never age**, and it only ever walks the owned root
(0131 §§ 2, 4). Every failure mode must fall toward under-deleting: a recycled pid leaks an entry until a
later sweep, and no test or fix may introduce a path that deletes a live owner's directory. `tmp clean`
has no force flag.

## Stage 0 — the catch-up

Nothing. `rule:core-classes/temporary-dir-sweep` landed with its spec § 14 amendment in the same change; no fixture predates the rule.

## Stage 1 — the floor

Goal `server`'s whole acceptance list — the entire parity program, six goals deep, never traded.

## Stage 2 — the keystone: the owned root and the end-of-script sweep

1. **The owned root.** `capability::temp_dir` creates under `[io] temp_root` (a Boot key in
   `nvs-config`), else a private `novis` subdirectory of the platform temporary directory — no longer
   bare `std::env::temp_dir()`. Entry naming (`nvs-<pid>-<nonce>`) and the `fs.write` check are
   unchanged.
2. **The tracked list and the sweep.** The runtime records each path `temporaryDir` answers, per script;
   at every ending the process survives, native teardown deletes what still stands, after the `onExit`
   queue, throwing nothing, logging refusals.
3. **The registry card** rewritten in the same slice per
   [0117](../../decisions/0117.md): the "removing
   it is the program's own job" paragraph is replaced by 0131's contract, and `docs/novis.md`
   regenerated.

## Stage 3 — the server: post-response and boot

1. **A request's temporary dirs are swept after its `afterResponse` work**, off the request path; an
   aborted request's dirs are swept by the surviving worker
   ([0106](../../decisions/0106.md)).
2. **`nvs serve` boot runs the orphan sweep** over the owned root, before traffic: every `nvs-<pid>-*`
   entry whose pid is dead is removed; live owners are skipped.

## Stage 4 — `nvs tmp clean`

The same orphan sweep as a subcommand: prints each path it removes, `--dry-run` prints and deletes
nothing, exits 0 when there is nothing to do. No force flag, per the standing decision above.

## Stage 5 — `keep_temporary`, the fixture, the suites

1. **`[debug] keep_temporary`** (reloadable, [0078](../../decisions/0078.md)):
   the end-of-script sweep logs each path it would have deleted and deletes none. No in-language setter
   exists or is added.
2. **`examples/tempdir.nvs`** — a script that creates a temporary dir, writes and reads a file inside
   it, and prints what it read; a named integration test then asserts the directory is gone from the
   owned root after the run.
3. The conformance and differential suites, and the leak sweep, still green.

## Standing decisions

- **`rule:core-classes/temporary-dir-sweep` is the design and is not reopened.** Its sections are the items above; a conflict between
  this file and the ADR is a bug in this file. No new ADR numbers in this goal.
- **Pid-liveness has a per-platform edge** (recycled pids, access-denied on `OpenProcess`). Any
  ambiguity resolves toward *skip* — under-delete, never over-delete — decided-and-recorded in the
  sweep module's doc comment, never `BLOCKED`.
- **Windows deletion refusals are expected, not failures.** A held handle (indexer, scanner) makes the
  sweep log and move on; tests that need a refusal simulate one by holding the handle themselves.
- **`[io]` is a new config section**; its shape follows
  [0064](../../decisions/0064.md) and its key classification
  [0078](../../decisions/0078.md) — `temp_root` Boot, `keep_temporary`
  reloadable, both recorded in the registry's reloadability field.
