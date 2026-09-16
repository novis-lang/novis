---
milestone: dossier
---
# Loop goal 65 — queue the dossier

**One session, one command, and the chain is a hundred goals longer.**
`rule:testing/four-proofs` settled that every shipped feature
owes four proofs — a test from Novis *and* from Rust, three examples, one measured figure, one file
written to break it — and `tools/dossier.py` derives that roster from `nvs meta --json` and the reference
chapters rather than a list anybody maintains. The sweep says **1,069 features, four of them complete**.
This goal is where the loop stops adding surface and starts closing what is behind it.

It writes no proof itself. Its whole job is to emit the goals that do, onto the end of the chain the
driver is already walking, and then — holding the only view of those goals anyone will ever have all at
once — to **spend the rest of the session making the loop good at the shape it is about to repeat 92
times.** Emission is stage 2 and takes minutes; stage 3 is the goal.

## Why here

The dossier writes no proof: it fires `dossier.py --emit-goals` at this file, so
`rule:testing/four-proofs`'s roster — one goal per group of features owing four proofs — lands on
the end of the chain the driver is already walking, and `Chain.refresh()` in loop.py walks into it
without a restart. Everything after goal `dossier` is generated; nothing after it is hand-written, and it
is last because a proof is only worth writing over a feature that has stopped moving. `dossier.py`'s
emitter numbers what it appends from the highest number already on the chain (`chain_numbers`, read
by `emit_goals`), so the generated goals start directly after this entry however many hand-written
ones land in front of it — and one that lands in front renumbers this entry, which is `chain.py`'s
job and no author's.

## The target

    python tools/dossier.py --emit-goals

One goal per group of features sharing an implementing file set, each with its own `[context]` manifest,
each gated by `dossier.py --verify --group <G>`. As of the emission that motivated this goal that is
**92 goals over 1,065 owed features**, appended directly after this goal — the emitter numbers what it
appends from the highest number already on the chain, and a hand-written goal landing in front of this
one renumbers them all, which is `chain.py`'s job. The number is not frozen here: whatever
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

## Stage 3 — prepare the loop for 92 of the same thing

This is an **optimization pass, run at the one moment it has leverage**: after the emission, before the
first generated goal. [optimization-prompt.md](../optimization-prompt.md) is the standing version of this
pass and its rules hold here — *make a change when you can name the evidence that it removes nothing,
otherwise write it down* — with one difference that is the whole reason this stage exists. The standing
pass is **backward**-looking: it undoes drift the last twenty-five sessions caused. This one is
**forward**-looking, and its evidence is not a measurement of what happened but the 92 files you just
wrote. Read menu item 8 there before you start; it is the item this stage put on the menu.

**Four findings are already on the table**, measured against the emission
that motivated this goal. Confirm each against your own tree — the numbers move — and then decide. They are a floor for
this stage, not its ceiling.

- [ ] **Every goal's `[context] modules` resolves, and the map prints it — confirm that on your own
      emission.** A `lang:` or `tools:` goal names its reference chapter, a `types:` or
      `config:directives` goal the table its rows come from, a `Core` goal the files its members'
      registry literals sit in; `orient.py` resolves any pattern against `git ls-files`, and
      `python tools/dossier.py --check-goals` tests the same thing, so the stage 3 check below is that
      command exiting 0. One member is still unplaced — a standalone `CoreMethod` const the registry
      reader attributes to the class above it — and its goal carries its neighbours' anchors. If a
      generated goal's map still misses the file its sessions open, the fix is an anchor in `roster()`
      or `table_anchors()`, then re-emit — never a hand-edit.
- [ ] **The repetitive part is file creation, not thinking.** A feature owes ~6 files at paths derived
      from its id (`docs/examples/core/Str/at/`, `tests/hostile/core/Str/at/`,
      `benches/members/core/Str/at.nvs`), and 830 features is ~6,700 files. `--bless` already writes an
      example's `.out`. **Measured, and the answer came back no**: `loop-stats.py
      --attribute` charges 28% of a session to `writing`, and that share is the file *contents* — a
      stub still needs the Write that fills it, and a directory costs nothing to create. So a
      `--scaffold '<id>'` was not built. Confirm the share against your own run before you accept
      that; overturning it means naming what a stub removes that `--bless` and `--partition` do not.
- [ ] **The fan-out landed before this goal ran, and its width is the thing to confirm.** `rule:testing/four-proofs`'s
      work is file-disjoint by construction — three of the four proofs are attributed by a path derived
      from the feature's id — so `dossier.py --partition` cuts a group into worker briefs and refuses
      when two lanes would write the same path, and every generated goal's prose § *Running this goal
      wide* drives it. `FANOUT_WORKERS` is 8 and its docstring carries the derivation: a serial tail
      of 14.9 minutes per goal, against a session floor of 71,941 tokens and a subagent's 12,600, all
      measured over the 68 sessions in `.loop/logs`. Modelled over the emitter's real
      goal sizes that puts the whole program at **106 hours and 416 sessions serially against 34
      hours and 98 fanned out** — 3.1x, of which 1.4x is the smaller floor and 2.2x is the
      concurrency — and **$3,349 against $891**. **One input is an estimate and everything above
      rests on it**: 16 tool calls to close one feature's four proofs, which nothing on disk can
      price because no dossier goal has run. You are the session that can. Fan one group out, put
      the real per-feature figure in the report, and move `FANOUT_WORKERS` only if it says so —
      width is already flat there, 79% of the ceiling at six lanes and 80% at eight, so a wrong
      estimate moves the schedule and not the constant. What it cannot make safe is named in
      `tools/dossier.py` § *Running one group's features at once* and none of it is optional.
- [ ] **The acceptance sweep grows, the pack does not.** Each generated goal adds one
      `dossier.py --verify --group` check to the floor, so the last goal's sweep runs ~330 checks where
      today's runs 239. That cost is **wall clock, not context** — the checks are read by `loop.py` and
      never enter a session's pack — and both caches already exist (`.loop/dossier-green.json`, and the
      sweep asking last session's failing check first). Measure one sweep before deciding it is a
      problem; a slow sweep that is correct is not drift.

**Then look for what these four did not name.** You have the 92 goals, `loop-stats.py`,
`loop-stats.py --attribute`, `orient.py --audit --goal <a generated goal>`, and the emitter that wrote
all of them. Anything you change in `dossier.py` costs one commit and lands in 92 files; anything you
change in one generated file is discarded by the next emission.

- [ ] **Write the report to `.loop/optimization/report.md`**, in
      [optimization-prompt.md](../optimization-prompt.md) § *The report*'s shape. Its *Proposals* section
      is the valuable half: everything you saw and could not safely act on, for the sessions that come
      after and for the user's next by-hand pass.

## Standing decisions

**The perf proof stays on.** `--no-perf` drops 685 of the 3,035 owed proofs and is the obvious way to
make this smaller, and it is not taken: a figure per feature is what lets a later change be re-measured
against us rather than against PHP, which is `rule:testing/perf-two-mechanisms`'s
whole point. The user decided this knowing the size.

**`--per-goal` stays at 18** unless what you read in stage 3 says otherwise. It is a batch size, not a
slice budget: a session takes what fits under the 200k ceiling and the next one continues, exactly as
everywhere else. Changing it changes every file name, so decide once, in this session, and say why in
the commit. Modelled, it is also where the ceiling puts it: at 18 the fan-out's parent
peaks at 186k of the 200k, and 27 would save three hours of a 34-hour program while peaking at 203k.
Nine costs eleven hours and buys nothing.

**No ADR slot.** `rule:testing/four-proofs` is the decision and it is already written. The one thing this goal *did* decide
is recorded in `docs/agent/commands.md` § the dossier: the emitter may now be fired by a session, because
appending to the live chain and `Chain.refresh()` are what turn "then somebody restarts the driver" into
"the run continues". Nothing else here is new design.

**Do not start the proofs.** The first `Core` class belongs to the first emitted goal, with that goal's
manifest and that goal's floor. A session that writes a few examples here spends the session's fixed
cost twice for them. Stage 3
is the exception that proves it: writing *one* example and *one* attack to feel the shape is a
measurement, and it belongs in the report rather than in a commit under `docs/examples/`.

**The optimization cadence is not yours to raise.** `loop.py`'s `--optimize-every` is 25 and
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
