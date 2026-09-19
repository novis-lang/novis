# Handoff

## State

Goal `lang-concurrency`'s own gate is green: `python tools/dossier.py --group lang:concurrency`
reports all ten features complete, nothing owed.

**The goal cannot be reached right now, and no session can change that.** The working tree does not
compile — debug or release — from an **uncommitted** edit under `crates/nvs-lsp/` that no loop
session made: `server.rs:976` and `server.rs:1076` call `completion::at` with six arguments and
`completion.rs:308` declares five. `git status --porcelain` shows four modified files there, made
while this session ran. So `dossier.py --verify` has no binary to judge a proof against, `verify.py`
would stop at `build` for the same reason, and the full gate was not reached. That edit belongs to
the person at the keyboard: do not repair it, and do not commit it.

Landed this session: `tools/relink.py`, and the two builders that now use it. An editor whose
`nvs.path` points into this tree runs `nvs lsp` for the life of its window and holds
`target/release/nvs.exe`, which Windows will not let cargo delete, so every release build in that
window failed at the link — and `dossier.py` reported it as *the tree does not build in release*.
A build that fails that way is now retried once with the running copy renamed aside, which Windows
does allow. Proven against a real holder, and against one that was not holding.

## Next group

**Stage 2: the dossier** — one file set: `tools/dossier.py`, `tools/loop.py`, `tools/relink.py` and
the check's own argv (`rule:testing/feature-proofs`).

- [ ] **Re-run the goal's check once `cargo build` is green again** — `python tools/dossier.py
      --verify --group lang:concurrency`. Nothing owed and `0 failed` twice is the goal reached, so
      run the three DONE gates before claiming it. `tools/dossier.py:508`
- [ ] **First read `git status --porcelain`.** While `crates/nvs-lsp/src/server.rs:976` still hands
      `completion::at` six arguments the tree is mid-edit by a person, and the right move is to hold
      rather than to repair somebody else's file. `crates/nvs-lsp/src/server.rs:976`
- [ ] **Re-point the playbook bullet that tells a person to stop the server before a release
      build** — the tools free the binary themselves now, so what is left for a person is a bare
      `cargo build --release` typed by hand. `docs/agent/playbook.md:7658`

## Backlog

- One `target/release/nvs.exe.held-N` stays on disk per live editor window; the next successful
  release build deletes it (`tools/relink.py:sweep`).
- `tools/bench.py` builds nothing by design, so the warm-start figure can still be taken on a
  binary from before a change (`tools/loop.py:release_cli`).
- `[context] modules` names nothing under `tools/`, so this session's file set was not in the pack.
