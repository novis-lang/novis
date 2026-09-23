# Handoff

## State

**Side goal `playbook-triage` — every playbook bullet names its file and expires by itself, and a
session is served the traps for the files it touches — has just started; nothing of it has landed
yet.** It runs in its own worktree under `loop.py --side playbook-triage`, over main's carried floor,
and lands on `main` when it is green.

The design is settled and is the user's. The goal file's § *Standing decisions* answers every call
a session meets, the triage rubric included, and no session re-decides one. § *What is on disk
today, measured* is the starting point; the playbook is already one file per bullet, and the wrap
already holds every new bullet to this goal's bar.

## Next group

**Stage 2: the triage tool** — one file set: `tools/playbook.py`.

- [ ] **`--triage <section directory>`** — per bullet: file, selector, text, `anchors`, the full
      path a crate or bare file name most likely means, a proposed `gone <path>:<word>`,
      `LIVE MANIFEST`, `NO NAMED WORD IN ITS FILE`, and its `--dupes` partners. The closing line is
      spelled exactly as the goal's `.toml` wants it. It writes nothing.
- [ ] **`--check`'s two new reports** — `BULLETS THAT NAME NO FILE` and `BULLETS THAT TAKE
      reviewed`, each ending in the `none -- ...` line the `.toml` names. Reports only; Stage 9
      makes them gate.
- [ ] **The module doc's usage block** names `--triage`.

## Backlog

- **Stage 3: served by file** — `tools/orient.py:@run_playbook` ranks the whole playbook by the
  item's paths through one ranking function in `tools/playbook.py`; `orient.py --traps <path>...`.
- **Stage 4: triage `divergences` and `splitting-a-file`** — 50 bullets; proves the rubric.
- **Stages 5 to 8: triage `running-things`, `writing-novis`, `tooling`, `writing-a-test-case`.**
- **Stage 9: `--check` gates on both findings; the contract and the docs say what is true.**
- When the last check goes green the side run lands itself on `main`.
