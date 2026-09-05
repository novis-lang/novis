# Handoff

## State

**Goal 6, M7 is green.** The driver logged `## goal reached: 6 server -- every check in its
acceptance list passes` after run 14:24's session 0001, and this session found nothing left for it in
the ledger. The next clean acceptance run switches the chain to **goal 21, carried-gaps**, which
overwrites this file with `docs/agent/goals/21-carried-gaps.handoff.md` — so everything below matters
only if the run stays on goal 6.

**The chain's own blocker is gone.** The 14:24 run ended on `21 carried-gaps's acceptance list is not
runnable -- ... a command check needs `argv``; commits `fd8cdc52` and `5e3b8ce6` fixed that class of
authoring error by hand at 15:09, and `python tools/chain.py --check` now reports **29 entries, all
walkable** (one cosmetic note about entry 7 sitting below 22 in the `--list` order). The playbook
bullet added this session is why the message looked like it named a file that disagreed with it.

**The status line is `CONTINUE`, not `DONE`, deliberately.** `loop.py:4171` only reads a session's
`DONE` on the branch where the acceptance check *failed*, where it stops the run as a false claim; on
a passing check the driver advances the chain and never looks at the line. `CONTINUE` is therefore
the same outcome when the goal is green and the safe one when it is not.

**`orient.py` has a `[context] spec` field now** (`tools/orient.py:801`), slicing `docs/spec/*.md`
by `"01 §15"` exactly as `adrs` slices an ADR. No goal names it yet — that is the first item below.
It is the fix for the "the pack prints no spec section" line two handoffs in a row carried.

**`benches/serve.json` is a JSON array capped at 100 runs**, and the driver's acceptance check
appends one every iteration. A session will find it dirty without having written it: committing the
newer measurement or checking it out are both fine, and neither is a regression.

## Next group

**The manifest field this session landed, and the ratchet it was bought for.** File set:
`docs/agent/goals/21-carried-gaps.toml`, `crates/nvs-stdlib/tests/migration-members-outstanding.txt`,
`crates/nvs-stdlib/src/registry.rs`.

- [ ] **Name `spec` in the incoming goal's `[context]`** — `docs/agent/goals/21-carried-gaps.toml:17`.
      The field is live and unused; `docs/agent/loop-authoring.md`'s § 2 table row is the spelling and
      the way to get it wrong. Pick sections, never a whole file: `01-core-library.md` is 1,200 lines.
- [ ] **`Core\Os` is specified and unwritten** — `crates/nvs-stdlib/tests/migration-members-outstanding.txt:23`
      names `hostname`, `memoryUsage` and `pid` against a class with no module in the crate at all.
      The five-edit shape is `docs/agent/conventions.md`; the roster is
      `crates/nvs-stdlib/src/registry.rs:1255`. The list only shrinks.
- [ ] **`Core\Process::spawn`** — `crates/nvs-stdlib/tests/migration-members-outstanding.txt:30`. The
      class is registered and `run` is on it; the streaming half `proc_open`/`popen` point at is the
      last of the five owed keys.

## Backlog

- The `--list` order note from `chain.py --check`: entry 7 is numbered below the entry before it —
  legal, since a number is an identity tag, `docs/agent/goals/chain.toml`.
- Goal 6's own `[context]` never gained a `spec` entry; the live `loop-goal.toml` is about to be
  replaced, so the durable half is the tool and the authoring-doc row, both landed.
