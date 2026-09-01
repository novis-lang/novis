# Handoff

## State

**`Core\Ast\Node` is off `gaps.py`'s thinnest list.** Its three members went from three cases each to
six, and the three added here are the three shapes the class had no case for: agreement under a second
call, the leaf edge named on both sides, and a counting invariant over the whole walk. `python
tools/gaps.py` no longer ranks the class in its first twenty-five; `Core\Crypto`,
`Core\Process\Result` and `Core\Script\ExitReport` are the floor now.

- **The second slice's premise as the handoff wrote it was wrong, and the case pins the corrected
  thing.** The item asked for `nodes` on a leaf to answer "a list of exactly one"; `nodes` excludes the
  receiver, so a leaf walks **zero**, and the landed
  `core-ast-parse-answers-the-compilers-own-tree.nvst` already pins that leaf's `0`/`0` pair. So the
  case names the bound *adjacently* instead — one descent by last child prints the last node that has
  children immediately above the first that has none — and adds the claim no case had: "no children"
  is not "not a node", asserted by identity, since the leaf is still one of its parent's children and
  still in the file's walk.
- **Identity is what makes the agreement case more than a re-read.** `children` hands back the very
  instances `parse` built, so both calls' elements collapse into one entry of a
  `Core\ObjectSet<Core\Ast\Node>`; `nodes` allocates a fresh array per call but fills it with those
  same objects. Two `parse` calls are the mirror: every answer agrees and the two trees share no node
  at all.
- Nothing in `crates/nvs-stdlib/src/ast.rs` changed — three `.nvst` files and nothing else — so there
  is no new refcount edge and no valgrind run behind them.

**The two `orient.py` warnings are still there**: the `[context] modules` patterns
`crates/nvs-stdlib/src/fatal.rs` and `crates/nvs-stdlib/src/script.rs` are reported as matching no
module and are then printed in the scoped map anyway. The manifest is right; the matcher is what to
check.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate over
a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside this
goal and is not a regression.

## Next group

**`Core\Script\ExitReport` is the thinnest class left whose every member is on the floor — `reason`,
`status` and `error`, three cases each — and all three are bound by one rule, ADR 0127's: the report
*observes* the ending rather than changing it, and the ending is one fact every hook in the queue sees
the same way. The file set is `crates/nvs-stdlib/src/script.rs` and `tests/conformance/core/`; the
three helper bodies are within twenty lines of each other. The landed cases are
`script-on-exit-runs-its-queue-fifo-after-the-last-statement.nvst`,
`script-on-exit-sees-the-exit-no-finally-can-see.nvst` and
`script-on-exit-carries-the-live-throwable-of-an-uncaught-throw.nvst` — read those first, both because
the FIFO question is already asked and because the second of them is how a case reaches an ending
other than the normal one.**

- [ ] **A case that asks each member twice and asserts agreement** — `reason`, `status` and `error`
      answered twice inside one hook must agree, and two hooks in one queue must see the same report,
      since the ending is a fact recorded before the queue runs rather than something each hook
      recomputes. `crates/nvs-stdlib/src/script.rs:684`, `crates/nvs-stdlib/src/script.rs:692` and
      `crates/nvs-stdlib/src/script.rs:700`.
- [ ] **A case that names the status bound on both sides** — the status of a normal ending beside the
      first non-zero one an `exit` names, with `error` `null` on one side and a real `Throwable` on
      the other, so a hook cannot read "no error" as "no report".
      `crates/nvs-stdlib/src/script.rs:692` and `crates/nvs-stdlib/src/script.rs:700`.
- [ ] **A case that asserts the roster invariant by counting** — over the endings ADR 0127 admits, the
      `reason` is one of the closed `Core\Script\ExitReason` cases every time and `status`/`error`
      agree with it, counted across the endings rather than read off one line.
      `crates/nvs-stdlib/src/script.rs:684` and `crates/nvs-stdlib/src/script.rs:700`.

## Backlog

- `Core\Crypto` has only the *agreement* shape left — one slice, not a group; its edges and its sweep
  are landed (`crates/nvs-stdlib/src/crypto.rs:447`).
- `Core\Process\Result` — `exitCode`, `stderr`, `stdout`, all three on the floor — is the next
  three-member group after `ExitReport` (`crates/nvs-stdlib/src/process.rs`).
- `Core\Task::afterResponse` is the last differential gap, and it needs `tests/differential/`, never
  `tests/conformance/` (`docs/agent/conventions.md`).
- `orient.py`'s `[context] modules` matcher reports `fatal.rs` and `script.rs` as matching nothing
  while printing them — `tools/orient.py`.
- 112 unasserted error paths, of which 5 are catchable throws — `python tools/gaps.py --errors`.
