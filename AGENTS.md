# Working on MWL

MWL is a JIT-compiled, memory-safe language for web servers and the command line. It exists to run web
requests and CLI programs **securely and fast**.

This file is inlined into every agent's context before it does anything, so it holds only what cannot be
looked up on demand: the routing table's two entries, the priority ordering, the rules whose whole cost is
that you did not know they existed, and the session workflow. **Everything else is one call away.**

**Every fact in this repository has exactly one home.** If two documents state the same thing, the one
named as the home is authoritative and the other is a bug — fix it rather than reconciling it in your head.

Any agent, any harness. `CLAUDE.md` at the root is a pointer to this file; nothing here is Claude-specific.

## Where to look

Do not read the docs tree breadth-first: most of it is reasoning you only need when you are about to
overturn a decision. **One call orients you**, and which one depends on why you are here:

| You are | Run |
|---|---|
| A session of the unattended loop | `python tools/orient.py` — everything below, already narrowed to the current goal's `[context]` manifest and its current item. Nothing else. |
| Working interactively, on anything | `python tools/brief.py` — the plan's status, one line per milestone and per module, the definitions most often grepped for, the guard tests, what is on disk |
| Looking for the file that owns a topic | `python tools/brief.py --where <keyword>` — the routing table in [docs/adr/README.md](docs/adr/README.md) § *Where to look*, filtered |

Those three route to everything else. The four files behind them, none of which is read in full by default:

- **[docs/adr/ground-rules.md](docs/adr/ground-rules.md)** — one sentence per settled decision, with its
  ADR. The index of what has already been decided; the linked ADR's body is the rule.
- **[docs/agent/commands.md](docs/agent/commands.md)** — how this repo is driven: `verify.py`, `splice.py`,
  `plan.py`, WSL, valgrind, and the two shell rules below in full.
- **[docs/agent/doc-style.md](docs/agent/doc-style.md)** — how to write anything in `docs/`, and the length
  targets nothing enforces.
- **[docs/agent/conventions.md](docs/agent/conventions.md)** — the *shape* of a commit message, a `.mwlt`
  case, a `Core` member, an ADR, a diagnostic. Read this instead of opening an example to copy.

**An ADR's body always states the current rule.** A later decision is folded into the earlier ADR's text,
never left as an overlay you have to apply while reading. If a body disagrees with a cross-link, the body
is the bug — fix it.

## The priority ordering

Highest first. A lower item is spent to buy a higher one, never the reverse. Reasoning and bounds:
[ADR 0004](docs/adr/0004-memory-for-simplicity.md).

1. **Security and request isolation** — not traded for anything.
2. **Correctness of language semantics** — PHP-compatible observable behaviour.
3. **Latency and throughput on the request path.**
4. **Simplicity** — of the language surface first, then of the implementation.
5. **Memory footprint** — last, and spent deliberately to buy any of the above.

When choosing between designs:

- Prefer the safer, faster or simpler one even when it holds more memory. MWL is **not** a low-footprint
  runtime; "allocates less" is not on its own a reason to change anything.
- If a change spends memory, **say what it spends** — per request or per task — in the doc comment or ADR
  that records it.
- Memory must stay attributable to a request and under an enforceable cap, and must be O(in-flight) rather
  than O(requests served). Growth with total traffic is a leak, not a trade-off.
- Bytes *moved* are not cheap. An allocation or extra cache miss on a hot path is a latency question
  (priority 3), not a footprint one.
- Saving memory at the cost of an invariant every future contributor must remember is the wrong direction —
  that is the account the unsafe modules are already drawing on.

## The four rules you will otherwise break

Each is one sentence here because not knowing it exists is the entire cost. The mechanism, and why, is in
[docs/agent/commands.md](docs/agent/commands.md).

1. **A shell never carries file content into the tree.** Create and edit files with Write and Edit — never
   a heredoc, a `>` redirect or a `sed -i`, because the shell parses your apostrophes and backticks before
   it runs anything. An edit those tools cannot express goes through `python tools/splice.py`.
2. **One shell call runs one command** — a `;`-chain reports only the last one's exit status — **but
   independent calls go out together in one message**, and that batching is the largest unclaimed saving
   this repository has.
3. **Read a big file in the region you need.** Whole file under ~400 lines; past that, `grep -n` for the
   anchor and read around it. Context, not the clock, is what caps a session.
4. **Verify with one call, once, at the end:** `python tools/verify.py` — build, test, clippy and fmt in
   order, stopping at the first failure.

## Session workflow

Every session runs the same five steps, in this order, and **stops**:

1. **Orient in one call** — `python tools/orient.py` in the loop, `python tools/brief.py` interactively.
   That is the map, where the work stands, the traps that apply to these files, the shapes you are about to
   write, and the rules this goal lives inside. It is narrowed on purpose: if you find yourself needing
   something it did not print, that is a gap in the goal's `[context]` manifest — say so in the handoff.
2. **Do the work — as much of the group as fits under the context ceiling.** The handoff names a group of
   related slices and the file set they share. **Take the first. Take a second only if it touches files
   already loaded and the first left you well short of the ceiling. Never take a third.** Context is the
   binding budget here, not the clock: an agent degrades well before its window is full, so the ceiling is
   a fixed **200k**.
3. **Verify what you touched, once, at the end of the group** — `python tools/verify.py`, plus whatever the
   change specifically warrants (a `valgrind` run for a new refcount edge). **This is the only place
   verification happens**, and a group shares one run: the build is the same build.
4. **Write the docs and the handoff, once for the whole group.** Update the plan's status block with
   `python tools/plan.py --set` and any doc the change invalidates, then overwrite
   `docs/agent/handoff.md` with where the work stands now. It is *state* — a fact that will still be true
   in ten sessions belongs in the playbook, an ADR, or a crate's module doc instead. Naming the **next**
   group, and the file set it shares, is this step's job: you are the only one holding the context to
   decide it cheaply.
5. **Commit — one per slice, all after step 3 is green**, staging each slice's own files so `git log`
   still reads a slice at a time. Then you are done.

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
