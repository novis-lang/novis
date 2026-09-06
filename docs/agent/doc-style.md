# Writing docs here

The docs are optimised for an agent that reads one file and starts working. Keep them that way.
[AGENTS.md](../../AGENTS.md) points here rather than keeping a copy.

- **State a fact once.** A rule's home is its fragment under [docs/rules/](../rules/), and `python
  tools/brief.py --where <keyword>` names the one that owns a topic; link to it from everywhere else,
  as the token `rule:<topic>/<rule>`. Summarising a rule or a decision record into another document
  creates a second copy that will silently go stale.
- **Never quote a measured number outside the record that owns it.** Numbers live with their guard test.
- **Edit the rule, never overlay it.** When a decision changes, edit the rule's fragment so its body is
  true now, and write the new decision record whose `changes:` block names that rule. Never add a
  paragraph to one file describing what another file changed — that is what makes a reader apply
  patches in their head, and it is how `rule:types/declaration` came to carry eighteen of them at
  once. `git log` is the changelog, and the records are the frozen reasoning; neither is a thing the
  rule's own text describes.
- **A comment is prose too.** Every line on this page binds a `//!` header, a `///` on a `Core` member and
  a `#` in a manifest exactly as it binds a file under `docs/`: present tense, rewritten whole rather than
  overlaid, and carrying no date and no measured number it does not own.
  [conventions.md](conventions.md) § *A code comment* is the home of the shape, and `python
  tools/prose.py --check` is what keeps it from drifting back.
- **Front-load.** Decision first, reasoning below it. Assume the reader stops after the first screen.
- A choice that would be expensive to reverse gets its own numbered decision record if the reasoning is
  subtle or contested; otherwise a paragraph in *Decisions taken at project start* in
  [docs/adr/README.md](../adr/README.md). Either way the rule it settles earns a fragment under
  [docs/rules/](../rules/), and [docs/ground-rules.md](../ground-rules.md) — one line per rule — is
  generated from it by `python tools/rules.py --render`, never written.
- Crates for later milestones are created when their milestone starts, not left sitting empty.

## Length targets, and why nothing enforces them

These are the shapes that keep a doc readable at a glance. **Every one is guidance addressed to you, not
a check.** Nothing in this repository, in `tools/brief.py`, in `tools/orient.py`, or in CI measures a line,
a field or a file against a number, with the one exception stated below the table:

| Thing | Aim for |
|---|---|
| One field of the plan's status block | ~400 bytes, one overwritten paragraph |
| One `### Mn — title` milestone heading | one short line |
| One decision record's H1 | one sentence — it is the title every index derives from |
| One rule fragment's opening sentence | one sentence — `ground-rules.md`'s line for the rule is cut from it |
| One guard test's name + bounds | one line |
| One module's `//!` first sentence | one line — it is the map's entry for that file |
| One Rust module | ~1,500 lines of code; past that, split it at a **spec-shaped** seam |
| One function | ~250 lines; past that, an arm with a rule of its own becomes a call |
| [handoff.md](handoff.md) | ~60 lines — state, not a changelog and not the playbook |
| One [playbook.md](playbook.md) bullet | three sentences, ~400 bytes, plus its `[until:]` trailer ([conventions.md](conventions.md) § *A playbook bullet*); the file itself has no target, because expiry prunes it |
| A goal's `[context]` manifest | ~40 lines, and the pack it selects under 20k tokens |

**Spec-shaped** is the whole of the split rule: cut where a rule or a grammar layer already draws a
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
`tools/orient.py` may only print text bounded by a **count of entities** — one line per milestone, per
rule, per record, per guard test, per module in scope — never by a length of prose.
