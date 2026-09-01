# Handoff

## State

**`Core\Process\Result` is no longer the thinnest class in the tree.** Its three members now stand
at the floor of three cases each over five case files, and the three questions the landed pair did
not ask are all answered: the absence edge (a child that writes nothing), two results alive across
each other, and every member asked twice. `python tools/gaps.py` ranks the class at depth 3.0 /
floor 3, alongside `Core\Ast\Node`, `Core\Password` and `Core\Script\ExitReport`.

- **The empty capture is *measured*, not tested for truthiness.** `Core\Bytes::length` is printed,
  because the failure this is written against is a one-byte answer — a shell that echoed a line, or
  a member that terminated the capture itself — and every "is it empty" spelling reads `"\n"` as
  output that is merely small. `:` and `exit 0` are the two shells' own null commands; `echo` with
  no argument writes the one byte the case refuses.
- **Every one of the three new cases seeds its variables with a *non-empty*, mutually unequal
  sentinel**, because the platform two-`try` shape means a run in which neither candidate started
  would otherwise print exactly the zeroes and the agreements the cases are looking for. The
  `started=` count is still the first line asserted, as in the landed pair.
- **The double-read case pins the first answer as well as the agreement.** Two empty captures agree
  and so do two zeroes, so a member that drained on first read would pass an agreement test that
  only compared the two answers; each half asserts the marker is there *and* that the second call
  still says so.
- All three run green on both legs — Windows `target/debug/nvs.exe` and the WSL `nvs` under
  `/var/tmp/nvs-target-wsl` — and nothing in `crates/nvs-stdlib/src/process.rs` changed, so there is
  no new refcount edge and no valgrind run behind them.

**Two `orient.py` warnings this session are spurious and worth one look**: it reported that the
`[context] modules` patterns `crates/nvs-stdlib/src/fatal.rs` and `crates/nvs-stdlib/src/script.rs`
matched no module, yet printed both in the scoped map immediately below. The manifest is right; the
matcher is what to check.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside
this goal and is not a regression.

## Next group

**`Core\Password` is the thinnest security-bearing class left — three members, three case files,
one question each — and it is the one class on `gaps.py`'s floor whose members are all bound by the
same stored-hash format, so a case about any of them is a case about that format. The file set is
`crates/nvs-stdlib/src/password.rs` and `tests/conformance/core/`. Read the three helper bodies
first: they are within ninety lines of each other and the cost parameter each reads is the thing
all three slices turn on.**

- [ ] **A case that asks each member twice and asserts agreement** — `verify` on the same pair, and
      `needsRehash` on the same hash, must answer the same thing twice, while two `hash` calls over
      one password must *differ*, since the salt is fresh per call and a hash that repeated itself
      is the defect. `crates/nvs-stdlib/src/password.rs:307` and
      `crates/nvs-stdlib/src/password.rs:340`.
- [ ] **A case that names both sides of `needsRehash`'s bound** — a hash made at the configured cost
      answers `false` and one made at a lower cost answers `true`, named together, because a member
      that answered `false` always reads as correct against either half alone.
      `crates/nvs-stdlib/src/password.rs:396`.
- [ ] **A case that pins `verify`'s refusals** — a wrong password, an empty password against a real
      hash, and a stored string that is not a hash at all, each asserted to be a `false` rather than
      a throw or a fatal, so the member's whole failure surface is one answer.
      `crates/nvs-stdlib/src/password.rs:340`.

## Backlog

- `Core\Ast\Node`, `Core\Script\ExitReport` and `Core\Csrf` are the next three at the floor —
  `python tools/gaps.py`.
- `Core\Task::afterResponse` is the last member with a PHP twin and no oracle case
  (`fastcgi_finish_request`), and it belongs in `tests/differential/` — `docs/agent/conventions.md`.
- `orient.py`'s two spurious `[context] modules` warnings — `docs/agent/loop-goal.toml`.
- Stage 10's `every_part_two_spec_member_is_registered` needs spec §§ 15-19, which are goal 6's —
  `docs/agent/loop-goal.toml`.
