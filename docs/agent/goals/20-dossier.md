# Loop goal 20 — queue the dossier

**One session, one command, and the chain is a hundred goals longer.**
[ADR 0134](../../adr/0134-every-shipped-feature-owes-four-proofs.md) settled that every shipped feature
owes four proofs — a test from Novis *and* from Rust, three examples, one measured figure, one file
written to break it — and `tools/dossier.py` derives that roster from `nvs meta --json` and the reference
chapters rather than a list anybody maintains. On 2026-09-04 the sweep says **795 features, one of them
complete**. This goal is where the loop stops adding surface and starts closing what is behind it.

It writes no proof itself. Its whole job is to emit the goals that do, onto the end of the chain the
driver is already walking, and to read enough of what it emitted to be sure three hundred sessions are
about to do the right thing.

## The target

    python tools/dossier.py --emit-goals --append-chain docs/agent/goals/chain.toml

One goal per group of features sharing an implementing file set, each with its own `[context]` manifest,
each gated by `dossier.py --verify --group <G>`. As of the emission that motivated this goal that is
**93 goals over 794 owed features**, appended as goals 21 onward. The number is not frozen here: whatever
the roster owes on the day this runs is what gets written, and a group that owes nothing is left out.

## The item list

Everything below is one file set — `tools/dossier.py` and the goals directory — so this is one session.

- [ ] **A release binary first.** The roster is `nvs meta --json`; `cargo build --release -p nvs-cli` is
      what makes the emission describe the language as it now stands rather than as it stood.
- [ ] **Emit.** The command above. It prints how many goals it wrote and the range it appended as.
- [ ] **Read what you emitted, and this is the real work of the session.** Open three or four of the
      generated `.toml`s — one `Core` class, one `lang:` chapter, one `tools:` chapter — and check the
      three things a generator gets wrong: the `[context]` manifest names modules that exist, the
      `--group` in the check spells the group the way `dossier.py` spells it, and the batch is a file set
      rather than an alphabetical run. A manifest that matches no module is a warning `orient.py` prints
      every session of that goal, so it is cheap to fix now and expensive to leave.
- [ ] **Fix the generator, never the generated file.** Anything wrong in a `.toml` is wrong in
      `goal_toml()`; edit that, re-emit, and let the files be overwritten. A hand-edit is lost at the next
      emission and the header of every generated file says so.
- [ ] **Commit the generated tree in one commit**, separate from any fix to `dossier.py`.

## Standing decisions

**The perf proof stays on.** `--no-perf` drops 685 of the 3,035 owed proofs and is the obvious way to
make this smaller, and it is not taken: a figure per feature is what lets a later change be re-measured
against us rather than against PHP, which is [ADR 0026](../../adr/0026-performance-measurement-methodology.md)'s
whole point. The user decided this on 2026-09-04 knowing the size.

**`--per-goal` stays at 18** unless what you read in step 3 says otherwise. It is a batch size, not a
slice budget: a session takes what fits under the 200k ceiling and the next one continues, exactly as
everywhere else. Changing it changes every file name, so decide once, in this session, and say why in
the commit.

**No ADR slot.** ADR 0134 is the decision and it is already written. The one thing this goal *did* decide
is recorded in `docs/agent/commands.md` § the dossier: the emitter may now be fired by a session, because
`--append-chain` and `Chain.refresh()` are what turn "then somebody restarts the driver" into "the run
continues". Nothing else here is new design.

**Do not start the proofs.** The first `Core` class is goal 21's, with goal 21's manifest and goal 21's
floor. A session that writes a few examples here spends the session's fixed cost twice for them.

**The floor grows, and that is accepted.** `goal-switch.py` folds each goal's checks into the next, so by
the end of the run every session carries one `dossier --verify --group` check per goal behind it. They
are short walks — the gate executes nothing, and `--run` caches a green verdict against the bytes that
produced it in `.loop/dossier-green.json` — but the acceptance test does get slower over three hundred
sessions, and the tail of the chain is where to look if a session's wall clock drifts.
