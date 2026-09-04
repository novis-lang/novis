# Loop goal 50 — queue the dossier

**One session, one command, and the chain is a hundred goals longer.**
[ADR 0134](../../adr/0134-every-shipped-feature-owes-four-proofs.md) settled that every shipped feature
owes four proofs — a test from Novis *and* from Rust, three examples, one measured figure, one file
written to break it — and `tools/dossier.py` derives that roster from `nvs meta --json` and the reference
chapters rather than a list anybody maintains. On 2026-09-04 the sweep says **795 features, one of them
complete**. This goal is where the loop stops adding surface and starts closing what is behind it.

It writes no proof itself. Its whole job is to emit the goals that do, onto the end of the chain the
driver is already walking, and then — holding the only view of those goals anyone will ever have all at
once — to **spend the rest of the session making the loop good at the shape it is about to repeat 93
times.** Emission is stage 2 and takes minutes; stage 3 is the goal.

## The target

    python tools/dossier.py --emit-goals --append-chain docs/agent/goals/chain.toml

One goal per group of features sharing an implementing file set, each with its own `[context]` manifest,
each gated by `dossier.py --verify --group <G>`. As of the emission that motivated this goal that is
**93 goals over 794 owed features**, appended as goals 51 onward — the emitter numbers what it appends
from the highest number already in the chain, so hand-written entries landing in front of this one never
move them. The number is not frozen here: whatever
the roster owes on the day this runs is what gets written, and a group that owes nothing is left out.

## Stage 2 — the item list

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

## Stage 3 — prepare the loop for 93 of the same thing

This is an **optimization pass, run at the one moment it has leverage**: after the emission, before the
first generated goal. [optimization-prompt.md](../optimization-prompt.md) is the standing version of this
pass and its rules hold here — *make a change when you can name the evidence that it removes nothing,
otherwise write it down* — with one difference that is the whole reason this stage exists. The standing
pass is **backward**-looking: it undoes drift the last twenty-five sessions caused. This one is
**forward**-looking, and its evidence is not a measurement of what happened but the 93 files you just
wrote. Read menu item 8 there before you start; it is the item this stage put on the menu.

**Three findings are already on the table**, measured on 2026-09-04 against the emission that motivated
this goal. Confirm each against your own tree — the numbers move — and then decide. They are a floor for
this stage, not its ceiling.

- [ ] **14 of the 93 goals name a `[context] modules` entry `orient.py` cannot map.** Every `lang:` and
      `tools:` goal points `modules` at its reference chapter, and `orient.py` maps
      `brief.crate_modules()` and `brief.editor_modules()` only, so the entry prints nothing and warns
      once in every session of that goal. `python tools/dossier.py --check-goals` is the finding, and the
      stage 3 check below is that command exiting 0. **The fix is a judgment call and it is yours**:
      point those goals at the crates that implement the chapter, or teach `orient.py` to map a reference
      chapter, or drop the entry and accept that a `lang:` goal has no map. Say which and why in the
      commit. What is *not* open is where the fix goes — `goal_toml()`, then re-emit.
- [ ] **The repetitive part is file creation, not thinking.** A feature owes ~6 files at paths derived
      from its id (`docs/examples/core/Str/at/`, `tests/hostile/core/Str/at/`,
      `benches/members/core/Str/at.nvs`), and 794 features is ~4,600 files. `--bless` already writes an
      example's `.out`. Decide whether a `--scaffold '<id>'` that creates the directories and the stub
      files earns its place, using `python tools/loop-stats.py --attribute` on the sessions you have:
      **if the Write/Edit share of a session is not where the cost is, do not build it** — say so in the
      report and move on. This is the one item most likely to be worth building and most likely to be
      built on a guess.
- [ ] **The acceptance sweep grows, the pack does not.** Each generated goal adds one
      `dossier.py --verify --group` check to the floor, so the last goal's sweep runs ~330 checks where
      today's runs 239. That cost is **wall clock, not context** — the checks are read by `loop.py` and
      never enter a session's pack — and both caches already exist (`.loop/dossier-green.json`, and the
      sweep asking last session's failing check first). Measure one sweep before deciding it is a
      problem; a slow sweep that is correct is not drift.

**Then look for what these three did not name.** You have the 93 goals, `loop-stats.py`,
`loop-stats.py --attribute`, `orient.py --audit --goal <a generated goal>`, and the emitter that wrote
all of them. Anything you change in `dossier.py` costs one commit and lands in 93 files; anything you
change in one generated file is discarded by the next emission.

- [ ] **Write the report to `.loop/optimization/report.md`**, in
      [optimization-prompt.md](../optimization-prompt.md) § *The report*'s shape. Its *Proposals* section
      is the valuable half: everything you saw and could not safely act on, for the sessions that come
      after and for the user's next by-hand pass.

## Standing decisions

**The perf proof stays on.** `--no-perf` drops 685 of the 3,035 owed proofs and is the obvious way to
make this smaller, and it is not taken: a figure per feature is what lets a later change be re-measured
against us rather than against PHP, which is [ADR 0026](../../adr/0026-performance-measurement-methodology.md)'s
whole point. The user decided this on 2026-09-04 knowing the size.

**`--per-goal` stays at 18** unless what you read in stage 3 says otherwise. It is a batch size, not a
slice budget: a session takes what fits under the 200k ceiling and the next one continues, exactly as
everywhere else. Changing it changes every file name, so decide once, in this session, and say why in
the commit.

**No ADR slot.** ADR 0134 is the decision and it is already written. The one thing this goal *did* decide
is recorded in `docs/agent/commands.md` § the dossier: the emitter may now be fired by a session, because
`--append-chain` and `Chain.refresh()` are what turn "then somebody restarts the driver" into "the run
continues". Nothing else here is new design.

**Do not start the proofs.** The first `Core` class is goal 51's, with goal 51's manifest and goal 51's
floor. A session that writes a few examples here spends the session's fixed cost twice for them. Stage 3
is the exception that proves it: writing *one* example and *one* attack to feel the shape is a
measurement, and it belongs in the report rather than in a commit under `docs/examples/`.

**The optimization cadence is not yours to raise.** `loop-supervisor.py`'s `--optimize-every` is 25 and
this stage does not move it, for [optimization-prompt.md](../optimization-prompt.md) menu item 7's reason:
down is a measurement and up is a bet, and nothing you hold prices a phase that has not run yet. What
this stage *can* observe is that the automatic pass now has a valid action on a generated goal at all —
menu item 8 — which it did not before, and that is the change that mattered. If the first fifty generated
sessions say the cadence is wrong, that is a *proposal* in the report, with the numbers, for the user.

**The floor grows, and that is accepted.** `goal-switch.py` folds each goal's checks into the next, so by
the end of the run every session carries one `dossier --verify --group` check per goal behind it. They
are short walks — the gate executes nothing, and `--run` caches a green verdict against the bytes that
produced it in `.loop/dossier-green.json` — but the acceptance test does get slower over three hundred
sessions, and the tail of the chain is where to look if a session's wall clock drifts.
