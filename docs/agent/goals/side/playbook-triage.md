---
milestone: post-parity
---

# Side goal — every playbook bullet names its file and expires by itself, and a session is served the traps for the files it touches

When this goal is green, every bullet under `docs/agent/playbook/` names, in backticks, a file in the
tree by its path from the repository root. Every bullet ends with a trailer the tree can decide:
`gone`, `exists`, `test` or `rule`, and never `reviewed`. A bullet that was not true, not a trap, or
already said somewhere else is gone, and a fact about how a subsystem works has moved into that
module's own doc comment. `orient.py` serves a session the bullets that name the files its item
names, ranked from the whole playbook. It no longer serves only the handful a goal manifest picked
by hand.

## Why a side goal

It needs nothing the chain has not built, and nothing on the chain waits for it. Its files are the
playbook's bullet files and three tools the chain's goals do not change. Chain sessions still add
bullets while it runs, but each one is a new file, so a triage edit and a chain append never touch
the same file.

## What is on disk today, measured

Measured on 2026-09-23 at `3b56664c8`, after the playbook was split into one file per bullet.

- **1,532 bullets, 863,531 B**, in six sections: Tooling 422, Running things 163, Writing a test case
  546, Splitting a file that got too big 13, Writing Novis itself 351, Divergences and refusals
  already pinned 37. The count was 795 on 2026-09-06 and has grown by about 43 a day since.
- **1,178 end in `[until: reviewed <date>]`**, which nothing retires. 683 of them are more than
  `REVIEW_DAYS` (14) old and 46 are dated after the day they were written. 339 end in `gone`, 13 in
  `exists`, 2 in `test`.
- **533 bullets name no file at all**, and 247 more name one only as a crate (`nvs-ir`) or a bare file
  name (`verify.py`). 736 name a full path, in backticks or in a `gone`/`exists` trailer.
- **43 bullets can reach a session.** A bullet reaches one only when a goal manifest's
  `[context] playbook` selector picks it (`tools/orient.py:@run_playbook`), and the live manifests
  hold 11 distinct selectors. Promotion by the item's paths only reorders what the manifest picked.
- **Sessions write far more bullets than they read.** Over 46 sessions, 24 wrote a bullet, and
  about 15 tool calls read one (`playbook.py --show`, or the file directly).
- **The wrap already guards new bullets** (`tools/session.py:@reviewed_refusals`, and the anchor
  refusal in `validate`): a new bullet must name a file, may not take `reviewed` when it names one,
  and may not carry a date after today. So from `3b56664c8` on, every bullet the chain adds already
  meets this goal's bar, and only the bullets written before it owe anything.

## Stage 0 — the catch-up

The sentences on disk this goal makes wrong. Each is rewritten whole by the session that lands the
behaviour, and not before:

- `docs/agent/playbook.md` — "`orient.py` hands a session only the bullets its goal's
  `[context] playbook` selects and its item's own files promote" (Stage 3 changes it).
- `docs/agent/loop-authoring.md` § 2's `playbook` row — a manifest selector is no longer the only
  way in (Stage 3).
- `tools/playbook.py`'s module doc — the `reviewed` kind stays for `guard-name-debt.md` and
  `carried-refusals.md`, and is no longer allowed on a playbook bullet (Stage 9). `--triage` joins
  its usage block (Stage 2).
- `docs/agent/commands.md` — wherever it lists what `playbook.py` and `orient.py` do (Stages 2 and 3).
- `tools/orient.py`'s comment on `PROMOTED_WHOLE` — the bound now applies to the whole playbook
  (Stage 3).

## Stage 1 — the floor

Main's carried floor, which a side run is always checked against (`tools/side.py`). Never traded.

## Stage 2 — the triage tool

One file set: `tools/playbook.py`.

- **`python tools/playbook.py --triage <section directory>`** prints, for every bullet in that
  section, what a session needs to decide it without opening anything else:
  - the bullet's file, its selector and its whole text;
  - the files it names (`anchors`), and for a crate name or a bare file name, the full path it most
    likely means;
  - a proposed trailer: `gone <path>:<word>`, where `<word>` is a backticked word of the bullet that
    is in that file today;
  - `LIVE MANIFEST` when a live goal's `[context] playbook` selector reaches the bullet. Its lead-in's
    opening words must stay as they are, and it may not be deleted;
  - `NO NAMED WORD IN ITS FILE` when none of its backticked words is in any file it names. That
    bullet is most likely dead;
  - the other bullets `--dupes` pairs it with.

  It ends with one line: `triage: <dir>: <n> bullet(s), <m> still owe a decision` while any bullet
  takes `reviewed` or names no file, and `triage: <dir>: every bullet names a file and declares a
  condition the tree decides` when none does. It exits 0 either way, and it changes no file.
- **`--check` reports two new findings.** `BULLETS THAT NAME NO FILE` ends in `none -- every bullet
  names a file in the tree`, and `BULLETS THAT TAKE reviewed` ends in `none -- no bullet takes
  `reviewed``. Both are reports until Stage 9 makes them gate.

## Stage 3 — a session is served the traps for its own files

One file set: `tools/orient.py`, `tools/playbook.py`.

- **`orient.py` ranks the whole playbook against the paths the session's item names**, with
  `playbook.score`, and prints the best `PROMOTED_WHOLE` (20) in full. A bullet the manifest picked
  and that is not among them is listed on one line, as today. With no item paths, the manifest's
  picks are all there is, as today.
- **The ranking has one home**, a function in `tools/playbook.py` that `orient.py` calls, so
  `--match` and the pack cannot disagree about which bullets a file set implies.
- **`python tools/orient.py --traps <path>...`** prints the traps section exactly as a pack would
  for an item naming those paths. Its source line says `ranked from the whole playbook by the <n>
  path(s)`.

## Stages 4 to 8 — the triage, one section at a time

One file set per stage: that section's directory under `docs/agent/playbook/`, plus the module whose
doc comment a moved bullet lands in. The smallest sections go first, so the rubric is proven on 50
bullets before it meets 546.

| Stage | Sections | Bullets |
|---|---|---|
| 4 | `divergences`, `splitting-a-file` | 50 |
| 5 | `running-things` | 163 |
| 6 | `writing-novis` | 351 |
| 7 | `tooling` | 422 |
| 8 | `writing-a-test-case` | 546 |

A session runs `--triage <dir>` and decides every bullet it prints, as many as fit under the context
gate, by the rubric in § *Standing decisions*. It commits each batch as its own slice. A stage is
done when `--triage` prints its `every bullet names a file` line for each of its sections.

## Stage 9 — the gate and the contract

One file set: `tools/playbook.py`, `docs/agent/playbook.md`, `docs/agent/conventions.md`
§ *A playbook bullet*, `docs/agent/commands.md`, `docs/agent/loop-authoring.md`.

- **`--check` exits 1 on a bullet that names no file or takes `reviewed`**, beside the two findings
  it gates on today. CI's `docs` job runs it, so a bullet written by hand outside the wrap is caught
  there.
- **The contract says what is true now**: `playbook.md` and `conventions.md` say a bullet names its
  file and expires by itself, and how a session is served them. The module doc drops `reviewed` for
  the playbook.

## Standing decisions

These are the user's calls, made on 2026-09-23. No session re-decides one.

- **One file per bullet**, under `docs/agent/playbook/<section>/<lead-in words>.md`. It landed on
  `main` in `4a4def86b` and is not reopened.
- **A bullet reaches a session by the files it names.** A goal manifest's selectors still add
  bullets on top, and `PROMOTED_WHOLE` stays 20.
- **Every bullet names a file and ends with `gone`, `exists`, `test` or `rule`.** `reviewed` is not
  used on a playbook bullet. There is no size ceiling, and none is added: the triage rubric decides
  what stays.
- **The triage rubric**, for each bullet, in this order:
  1. **Delete** it when the trap is no longer true (`NO NAMED WORD IN ITS FILE` is the usual sign;
     read the file at that spot when unsure); when a tool now refuses the mistake it describes, so
     the tool's message is its home; when it says what another bullet says (keep the clearer one);
     when its fact already has a home in a rule, a module doc, `AGENTS.md`, `conventions.md` or
     `commands.md`; or when it is a session's story rather than a trap.
  2. **Move** a fact about how one subsystem works, which has no home yet, into that module's doc
     comment, rewritten as a whole (AGENTS.md rule 7), and delete the bullet. A rule every agent
     needs whatever it edits goes into `docs/agent/commands.md` or `docs/agent/conventions.md`.
  3. **Keep** everything else. It names its file by full path in backticks, and ends with a
     mechanical trailer: `gone <path>:<word>` on a word in that file the trap depends on, `exists`
     for a fix not there yet, `test <fn>` for a hole a test will close, `rule <topic>/<slug>` for an
     undecided rule. Its text may be tightened to the three-sentence shape.

  **When unsure between keep and delete, keep**, with a `gone` trailer on the word the trap depends
  on: from then on the tree retires it, and `git log -S` holds every deleted bullet anyway.
- **A bullet a live chain manifest reaches is kept**, and its lead-in's opening words do not change.
  Its trailer and the rest of its text may. A side session never edits a chain goal's files, so it
  cannot prune the selector that names it.
- **This goal never writes `AGENTS.md`, a rule fragment or a decision record.** Those are the user's.
  A bullet that reads as a rule nothing records yet is kept with its mechanical trailer, and the
  handoff lists it for the user.
- **A bullet is deleted or edited by hand, never with `playbook.py --retire`**, because `--retire`
  also prunes the chain's manifests. The wrap's own `retire_expired` still runs, and a bullet the
  tree retires during this goal is simply gone.
- **The cost, stated.** The triage is about ten sessions of reading, with no change to the language
  or the runtime. A floor check that every bullet names a file means that a chain session renaming a
  file also fixes a bullet that names only that file. The user accepted that: such a bullet names a
  path that no longer exists, which `--check` already reports.
