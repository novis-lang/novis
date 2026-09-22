---
milestone: dossier
position: last
---
# Loop goal 176 — every landed comment reads plainly

Every example, attack and bench is copied to the website, and its comments are read by somebody who
looked the feature up. Once this goal is green, **every comment in every one of those programs is
inside the bounds of `AGENTS.md` § *Text an end user reads*, and a program that
leaves them is a red check** for every goal that follows, the same as a feature without its test.

This goal rewrites comments in programs that are already on disk. It writes no new proof, changes no
line of code, and changes no `.out`.

## Why here

Behind every generated goal, because those goals are still writing the programs this one rewrites: a
sweep in the middle of them would be run again at the end. They carry the rule in their brief and
check their own files with `--comments`, so what is left for this goal is what landed before the
rule did and what slipped past a brief. In front of `ci-green`, because that goal proves the tree
the run ends on, and this one still changes it. Both say `position: last`, so `--emit-goals` keeps
both behind whatever it appends, in this order.

The gate is switched on here and not earlier for one reason: every walked dossier goal's `--verify`
is carried as floor, and most landed programs miss the bounds today, so a gate in front of the sweep
turns the floor red for every session until the sweep is done.

## Stage 1 — the floor

Goal `the-description-is-owed`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the sweep

`python tools/dossier.py --comments docs/examples tests/hostile benches/members` names every program
outside the bounds and every line in it. The work is closing that list, one feature directory at a
time, and the stage is green when the command exits 0.

One slice is **one chapter or one class** — `docs/examples/core/Str`, `tests/hostile/core/Str` and
`benches/members/core/Str` together — because the three trees share the feature's path and a reader
who has understood the member once rewrites all of its programs with that one understanding.

## Stage 3 — the gate

Add `comments = true` under `[all]` in `tools/data/dossier-policy.toml`. From then on `owed()` counts
a feature whose programs leave the bounds, `--gate` names it, and `--id` and a worker's brief say
which file and which line. Nothing in `tools/dossier.py` changes: the file is the repository's
durable answer, exactly as it is for `perf` and `about`.

This is last on purpose. Switched on before Stage 2 is green it fails the floor.

## Running this goal wide

The carve-out of the generated dossier goals holds here, for the same reason: a worker's paths are
derived from a feature's path, so two workers cannot name the same file, and the result is judged
mechanically. Hand each worker one chapter or class, the three directories it owns, and these rules:

- **`AGENTS.md` § *Text an end user reads* is the rule**, and it is in your context already. It
  carries a before and an after for an example and for an attack.
- **Change comment lines and nothing else.** Not a statement, not a string, not a blank line between
  statements, not a directive line (`// bench:`, `// hostile:`, `// covers:`, `// dossier:`,
  `// requires:`), not a `.out`.
- **Run `python tools/dossier.py --comments <your directories>` until it exits 0**, and nothing else:
  no `git`, no `cargo`, no `--run`, no `--bless`.
- **Hand back the list of files changed**, and nothing else.

Then, in this session and only after every worker has stopped:

1. `git diff -U0 -- docs/examples tests/hostile benches/members`, and read every changed line that
   does not open with `//`. There should be none. One that exists is reverted, not reviewed.
2. `python tools/dossier.py --run examples` over what changed. A comment cannot change what a
   program prints, so a red example here means a worker touched code.
3. `python tools/verify.py`, then the wrap. One commit per chapter or class.

## Standing decisions

- **The rule is not reopened here.** `AGENTS.md` § *Text an end user reads* is the standard, and
  `comment_problems` in `tools/dossier.py` counts the three bounds it states as numbers and reads no
  words. A bound that seems wrong goes in the handoff's `## Backlog`; it is not loosened to pass a
  file.
- **A comment the check names is written again from the code, never patched.** Read the lines under
  it, then write what they do and what the result is, in the words of the rule's table. Dropping a
  word, swapping a dash for a comma, or replacing "answers" with "returns" inside the same winding
  sentence all pass the check and fail the rule: the sentence was the problem, not the word.
- **The voice is the thing being removed.** These comments were written in this repository's essay
  voice — code that *asks* and *answers*, facts told by contrast, its own words for cast, syntax and
  method. The reader is somebody who looked a feature up, often not in their first language. The
  test for every rewritten comment is whether it would survive a word-for-word translation.
- **Passing the check is the floor, not the goal.** Every comment in a file the sweep opens is read
  against the rule, including the ones the check did not name: a short sentence with a figure of
  speech in it, a word only somebody who works on Novis would know. A beginner and an expert read it
  once and come away with the same picture.
- **What does not fit goes to `about.md` or goes away.** A top comment is up to four lines. If the
  explanation needs more, the description is where it belongs, and if the description already says
  it the comment does not repeat it. An `about.md` edited here stays inside its own word band.
- **A comment that was wrong is fixed, not just shortened.** If rewriting shows the comment claims
  something the program does not do, the comment is made true. If the *program* is wrong, that is a
  finding: `rule:testing/a-failing-proof-is-fixed-or-recorded` names the two answers.
- **A program with no comment problem is not opened.** This is a sweep of what the check names, not
  a rewrite of the tree.
- **No feature is excused.** There is no `[skip]` entry for `comments`; every program can carry a
  plain comment.
- **No numbered ADR is opened by this goal.** The decision is `rule:testing/feature-proofs`'s
  "plainly-commented", and the README section is where it is spelled out.
