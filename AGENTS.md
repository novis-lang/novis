# Working on Novis

Novis is a JIT-compiled, memory-safe language for web servers and the command line. It exists to run web
requests and CLI programs **securely and fast**.

This file is inlined into every agent's context before it does anything, so it holds only what cannot be
looked up on demand: the routing table's two entries, the priority ordering, the rules whose whole cost is
that you did not know they existed, and the session workflow. **Everything else is one call away.**

**Every fact in this repository has exactly one home.** If two documents state the same thing, the one
named as the home is authoritative and the other is a bug — fix it rather than reconciling it in your head.

Any agent, any harness. The root carries no harness's file: `.claude/CLAUDE.md` is a pointer to this one,
because Claude Code discovers `CLAUDE.md` and never `AGENTS.md`. Nothing here is Claude-specific.

## Where to look

Do not read the docs tree breadth-first: most of it is reasoning you only need when you are about to
overturn a decision. **One call orients you**, and which one depends on why you are here:

| You are | Run |
|---|---|
| A session of the unattended loop | Nothing — the driver ran `python tools/orient.py` and piped the pack in ahead of your prompt: everything below, narrowed to the current goal's `[context]` manifest and its current item. Run it yourself only if it is genuinely absent. |
| Working interactively, on anything | `python tools/brief.py` — the plan's status, one line per milestone and per module, the definitions most often grepped for, the guard tests, what is on disk |
| Looking for the file that owns a topic | `python tools/brief.py --where <keyword>` — the routing table in [docs/adr/README.md](docs/adr/README.md) § *Where to look*, filtered |

Those three route to everything else. The four files behind them, none of which is read in full by default:

- **[docs/ground-rules.md](docs/ground-rules.md)** — one sentence per settled decision, with its
  ADR. The index of what has already been decided; the linked ADR's body is the rule.
- **[docs/agent/commands.md](docs/agent/commands.md)** — how this repo is driven: `peek.py`, `verify.py`,
  `session.py`, `splice.py`, `plan.py`, `disk.py`, WSL, valgrind, and the two shell rules below in full.
- **[docs/agent/doc-style.md](docs/agent/doc-style.md)** — how to write anything in `docs/`, and the length
  targets nothing enforces.
- **[docs/agent/conventions.md](docs/agent/conventions.md)** — the *shape* of a commit message, a `.nvst`
  case, a `Core` member, an ADR, a diagnostic. Read this instead of opening an example to copy.

**An ADR's body always states the current rule.** A later decision is folded into the earlier ADR's text,
never left as an overlay you have to apply while reading. If a body disagrees with a cross-link, the body
is the bug — fix it.

## The priority ordering

Highest first. A lower item is spent to buy a higher one, never the reverse. Reasoning and bounds:
[ADR 0004](docs/decisions/0004.md).

1. **Security and request isolation** — not traded for anything.
2. **Correctness of language semantics** — PHP-compatible observable behaviour.
3. **Latency and throughput on the request path.**
4. **Simplicity** — of the language surface first, then of the implementation.
5. **Memory footprint** — last, and spent deliberately to buy any of the above.

When choosing between designs:

- Prefer the safer, faster or simpler one even when it holds more memory. Novis is **not** a low-footprint
  runtime; "allocates less" is not on its own a reason to change anything.
- If a change spends memory, **say what it spends** — per request or per task — in the doc comment or ADR
  that records it.
- Memory must stay attributable to a request and under an enforceable cap, and must be O(in-flight) rather
  than O(requests served). Growth with total traffic is a leak, not a trade-off.
- Bytes *moved* are not cheap. An allocation or extra cache miss on a hot path is a latency question
  (priority 3), not a footprint one.
- Saving memory at the cost of an invariant every future contributor must remember is the wrong direction —
  that is the account the unsafe modules are already drawing on.

## The five rules you will otherwise break

Each is one sentence here because not knowing it exists is the entire cost. The mechanism, and why, is in
[docs/agent/commands.md](docs/agent/commands.md).

1. **A shell never carries file content into the tree.** Create and edit files with Write and Edit — never
   a heredoc, a `>` redirect or a `sed -i`, because the shell parses your apostrophes and backticks before
   it runs anything. An edit those tools cannot express goes through `python tools/splice.py`.
2. **One shell call runs one command** — a `;`-chain reports only the last one's exit status. Independent
   calls may go out together in one message, but measured sessions never do it, so use the two tools that
   batch for you instead. **Read with `python tools/peek.py A.rs:120-160 B.rs:@sym C.md:"## 4"`** — as
   many targets as you have questions, one call, and `--locate <symbol> ...` for `file:line` anchors
   alone. **Write a run of three or more edits with `python tools/splice.py --patch <file>`** — one patch,
   any number of files, all of it or none of it.
3. **Read a big file in the region you need.** Whole file under ~400 lines; past that, `grep -n` for the
   anchor and read around it. Context, not the clock, is what caps a session.
4. **Verify with one call, once, at the end:** `python tools/verify.py` — build, fmt, test, the `.nvst` trees and clippy in
   order, stopping at the first failure.
5. **Finish with one call:** `python tools/session.py --wrap <file>` applies steps 4 and 5 below — plan
   fields, playbook bullet, handoff, one commit per slice, status — or refuses and changes nothing.

## Session workflow

Every session runs the same five steps, in this order, and **stops**:

1. **Orient in one call** — already done for you in the loop (the driver pipes the pack in with the
   prompt), `python tools/brief.py` interactively.
   That is the map, where the work stands, the traps that apply to these files, the shapes you are about to
   write, and the rules this goal lives inside. It is narrowed on purpose: if you find yourself needing
   something it did not print, that is a gap in the goal's `[context]` manifest — say so in the handoff.
2. **Do the work — as much of the group as fits under the context ceiling.** The handoff names a group of
   related slices and the file set they share. **Keep taking slices from that group while both hold: the
   next one touches files already loaded, *and* you are under 120k with the previous one committed. Stop at
   the first slice that fails either test.** Context is the binding budget here, not the clock: an agent
   degrades well before its window is full, so the ceiling is a fixed **200k**. **This paragraph is the cap's only home** — every other file points here rather than
   restating a number, because three files holding three different numbers is how it last went wrong.

   **The gate is the budget, not a count**, because a slice's cost is not fixed and a count prices every
   slice as the most expensive one. A session pays a **fixed cost** — orienting at the front, collecting
   the verification and applying the wrap at the back — that is the same for a three-line slice as for a
   three-hundred-line one, so a slice that only writes a test over landed work buys that whole fixed cost
   a second time when it gets a session to itself. `python tools/loop-stats.py`'s `fixed cost per session`
   line is what it currently is; **the number is not copied here on purpose.** It moved from 39% to 22%
   the day the tail was measured correctly — `verify.py --start` fires mid-work, so every call after it
   had been counted as wrap-up — and a copy in this file was wrong between every pair of optimization
   passes that refreshed it. Read the tool.
   One lowering slice spends the 120k by itself; five test-writing slices over one file set do not, and a
   rule counting slices cannot tell those apart. This replaced a cap of 2, which `python
   tools/loop-stats.py` had derived three ways at once — and could not have derived otherwise, because it
   prices a slice from sessions that each did one hard one. Take the **ceiling** from that tool after any
   run that changes what a session reads: its projection opens where the *next* session will open, so a
   pass that cuts the pack shows up immediately. Leave the number of slices to the 120k gate.
3. **Verify what you touched, once, at the end of the group** — `python tools/verify.py`, plus whatever the
   change specifically warrants (a `valgrind` run for a new refcount edge). **This is the only place
   verification happens**, and a group shares one run: the build is the same build.
4. **Write the docs and the handoff, once for the whole group.** The plan's status block, a playbook bullet
   if a trap cost you time, and `docs/agent/handoff.md` overwritten with where the work stands now. The
   handoff is *state* — a fact that will still be true in ten sessions belongs in the playbook, an ADR, or
   a crate's module doc instead. Naming the **next** group, and the file set it shares, is this step's job:
   you are the only one holding the context to decide it cheaply, and `python tools/peek.py --locate` turns
   its `file.rs:NN` anchors into one call.
5. **Commit — one per slice, all after step 3 is green**, staging each slice's own files so `git log`
   still reads a slice at a time. Then you are done.

**Steps 4 and 5 are one call.** Write a single wrap file — plan fields, playbook bullet, handoff, a
`## commit:` per slice, the status line — and apply it with `python tools/session.py --wrap <file>`
(`--help` is the format, `--check` first says what the tree still owes). It applies everything or refuses
everything, so there is no half-written tail. Measured before the tool existed, this was 33 of a session's
98 calls and 42% of its token bill, because context is at its peak by then.

**After step 5, stop.** Do not re-run `cargo build`/`test`/`clippy`/`fmt`, do not re-read the orientation,
do not re-check a doc against a length. Writing prose cannot break a build, so there is nothing a second
test run could discover. If step 4 or 5 turned up a real problem, fix it and re-verify *that* — otherwise
the session is over.

## Keep each slice small, commit every one of them

Step 5 above, in detail:

- Always commit your work before you exit, and never leave a slice uncommitted. You don't need to review
  the history first — stage each slice's own files, and let the last commit sweep whatever is left.
- The handoff is `docs/agent/handoff.md`: **overwrite it**, never append, so it describes where the work
  stands now rather than the path taken to get here. Its shape is in
  [docs/agent/session-prompt.md](docs/agent/session-prompt.md). Then show the user the same prompt in chat.
- **The playbook is the opposite file.** [docs/agent/playbook.md](docs/agent/playbook.md) is append-mostly:
  add a bullet when a trap costs you time, edit one when it stops being true, and otherwise leave it
  alone. Never reword it to say the same thing differently — this lore lived inside the handoff until it
  was two thirds of it, regenerated in full every session, and the rewording was the whole cost.
- The docs accumulate rationale bloat as ADRs are added. Periodically — the user fires this by hand, never
  you automatically — re-run the pass in [docs/agent/doc-cleanup.md](docs/agent/doc-cleanup.md).
- Every time we add, change or remove a feature, decide and say what the tradeoffs are in performance,
  memory, usability and simplicity for developers using the language. If there are large tradeoffs, notify
  the user and ask for agreement before proceeding. If there are only benefits, go ahead.
- [docs/implementation-plan.md](docs/implementation-plan.md)'s leading status block has a **fixed field
  set** — `Status`, `Done`, `On disk`, `Toolchain`, `ADR slices landed`, `Open now`, `Blocking`. Overwrite
  a field in place each session; never append a paragraph, and never add a field name. Session-by-session
  history lives in `git log`; per-file known-gap detail belongs in that crate's own module doc comment, not
  in the plan.
- **The plan is that index plus one file per milestone**, under [docs/plan/](docs/plan/), with the frozen
  half — the pre-M0 decisions, the architecture, the verification strategy — in
  [docs/plan/design.md](docs/plan/design.md). Never open the directory to find one: `python tools/plan.py
  --show M8` prints a milestone, `--show M8:verify` its acceptance paragraph, `--amend M8 --from <file>`
  rewrites one, and `session.py --wrap` takes a `## milestone: M8` section for the same thing.
- **The schedule is the chain, not the milestone table.**
  [chain.toml](docs/agent/goals/chain.toml) is the order the loop walks; a milestone is an **identity
  tag** one or more goals carry, and the plan's `Carried by` cells are derived from it. So **say "goal
  19", never "in M7"** — M7's work alone sits at goals 6, 16, 18 and 19, so a milestone number says
  nothing about what is next or finished. `brief.py` prints the live goal; `plan.py --check` gates it.
