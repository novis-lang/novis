# Writing docs here

The docs are optimised for an agent that reads one file and starts working. Keep them that way.
[AGENTS.md](../../AGENTS.md) points here rather than keeping a copy.

- **State a fact once.** Put it in the home named in [docs/adr/README.md](../adr/README.md) § *Where to
  look*, and link to it from everywhere else. Summarising an ADR into another document creates a second
  copy that will silently go stale.
- **Never quote a measured number outside the ADR that owns it.** Numbers live with their guard test.
- **Fold, never overlay.** When a decision changes, edit the ADR that stated it so its body is true, and
  leave a one-line cross-link. Never add a paragraph to one file describing what another file changed —
  that is what makes a reader apply patches in their head, and it is how ADR 0007 came to carry eighteen of
  them at once. `git log` is the changelog.
- **Front-load.** Decision first, reasoning below it. Assume the reader stops after the first screen.
- A choice that would be expensive to reverse gets its own numbered ADR if the reasoning is subtle or
  contested; otherwise a paragraph in *Decisions taken at project start* in
  [docs/adr/README.md](../adr/README.md). Either way it earns one bullet in
  [docs/adr/ground-rules.md](../adr/ground-rules.md) and one row in the routing table.
- Crates for later milestones are created when their milestone starts, not left sitting empty.

## Length targets, and why nothing enforces them

These are the shapes that keep a doc readable at a glance. **Every one is guidance addressed to you, not
a check.** Nothing in this repository, in `tools/brief.py`, in `tools/orient.py`, or in CI measures a line,
a field or a file against a number, with the one exception stated below the table:

| Thing | Aim for |
|---|---|
| One field of the plan's status block | ~400 bytes, one overwritten paragraph |
| One `### Mn — title` milestone heading | one short line |
| One ADR index **Decision** cell | one sentence, ~160 bytes |
| One `ground-rules.md` bullet | one sentence, plus the link |
| One guard test's name + bounds | one line |
| One module's `//!` first sentence | one line — it is the map's entry for that file |
| One Rust module | ~1,500 lines of code; past that, split it at a **spec-shaped** seam |
| One function | ~250 lines; past that, an arm with a rule of its own becomes a call |
| [handoff.md](handoff.md) | ~60 lines — state, not a changelog and not the playbook |
| [playbook.md](playbook.md) | no target; it grows a bullet at a time and that is correct |
| A goal's `[context]` manifest | ~40 lines, and the pack it selects under 20k tokens |

**Spec-shaped** is the whole of the split rule: cut where an ADR or a grammar layer already draws a
line — `parser/{ty,expr,stmt,decl}`, `expr/{quals,operators,members}` — never at a line count, because a
seam the code's own decisions do not follow gets crossed by the next slice and the split has to be redone.
A directory's `mod.rs` then owes a **charter** in its `//!` header saying what it keeps and what it
delegates; without one it grows back into the file that was split.

Write to the target, and if a line lands a little over, **leave it**. This used to be a hard check that
failed CI, and the cost was not the bytes: it was five and ten iterations per session spent shaving prose
to clear a tripwire, at the end of a session, when the real work was already done. The same applies to the
context budget `tools/orient.py --audit` prints: it is a number to look at when you write a goal, never an
exit code, and never something to trim prose against mid-run.

The one exception is a gate on **growth**, not on size. `python tools/session.py --wrap` refuses an edit
that leaves a plan field both over its ceiling — the plan's header comment is the home of that number, as
of the aim — *and* bigger than it was. A field that shrinks or holds its size is always taken, so there
is nothing to shave to clear it: adding a sentence to a full field costs dropping one, which is what
"overwrite a field in place" meant all along. It exists because `Open now` reached 52 KB — 46% of the
orientation pack — under the advisory note it replaces, and `--check` and `--template` print the headroom
ahead of the wrap so a session that reads either never meets the refusal at all.

The one structural rule that *does* still matter is not about length: `tools/brief.py` and
`tools/orient.py` may only print text bounded by a **count of entities** — one line per milestone, per ADR,
per guard test, per module in scope — never by a length of prose.
