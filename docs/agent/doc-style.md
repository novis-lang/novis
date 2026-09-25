# Writing docs here

The docs are optimised for an agent that reads one file and starts working. Keep them that way.
[AGENTS.md](../../AGENTS.md) points here rather than keeping a copy.

- **State a fact once.** A rule's home is its fragment under [docs/rules/](../rules/), and `bun nv
  brief --where <keyword>` names the one that owns a topic; link to it from everywhere else,
  as the token `rule:<topic>/<rule>`. Summarising a rule or a decision record into another document
  creates a second copy that will silently go stale.
- **Never quote a measured number outside the record that owns it.** Numbers live with their guard test.
- **No date, anywhere under `docs/`.** When something was decided, accepted, retired, measured or changed
  is `git log`'s to answer, and it answers exactly; a date written into prose can be checked against
  nothing and tells a reader years later only that time has passed. This binds a record's body, a plan
  file, a goal manifest and a rule alike — it is the rule [conventions.md](conventions.md) § *A code
  comment* has always bound `crates/` to, and the reason the frozen records carry no `date:`. A date that
  is a **value the subject handles** — an ISO literal in a `Core\Time` example, a `#[Test(at:)]` fixture,
  a SQL zero date — is content and stays. The one deliberate exception is a `[until: reviewed <date>]`
  playbook trailer, which is an expiry the wrap acts on rather than a claim about the past.
- **Edit the rule, never overlay it.** When a decision changes, edit the rule's fragment so its body is
  true now, and write the new decision record whose `changes:` block names that rule. Never add a
  paragraph to one file describing what another file changed — that is what makes a reader apply
  patches in their head, and it is how `rule:types/declaration` came to carry eighteen of them at
  once. `git log` is the changelog, and the records are the frozen reasoning; neither is a thing the
  rule's own text describes.
- **A comment is prose too.** Every line on this page binds a `//!` header, a `///` on a `Core` member and
  a `#` in a manifest exactly as it binds a file under `docs/`: present tense, rewritten whole rather than
  overlaid, and carrying no date and no measured number it does not own.
  [conventions.md](conventions.md) § *A code comment* is the home of the shape.
- **Front-load.** Decision first, reasoning below it. Assume the reader stops after the first screen.
- A choice that would be expensive to reverse gets its own numbered decision record if the reasoning is
  subtle or contested; otherwise a paragraph in *Decisions taken at project start* in
  [docs/adr/README.md](../adr/README.md). Either way the rule it settles earns a fragment under
  [docs/rules/](../rules/), and [docs/ground-rules.md](../ground-rules.md) — one line per rule — is
  generated from it by `bun nv rules --render`, never written.
- **Name a goal by its slug, never by its number.** Goal `parses`, never `goal 21` — here, in a `//!`
  header, in a commit message, in a test's owner column, and in a line a tool prints to a console.
  A goal's number is its **position** on the chain, so inserting anything in front of it renumbers
  it and every sentence naming the old number now names whichever goal moved into it, silently, in
  files nobody opened; the slug is the half of the filename that never moves. A number beside a
  total — `29 of 43` — is that position and is fine; a number standing where the name goes is not.
  `bun nv chain` places a goal by its slug and renames no file, so the one place a number still
  stands is a goal's own `# Loop goal N —` or `# Goal N --` header. `bun nv chain --check` fails on
  any other in a file — except inside backticks, where the wrong form is being quoted rather than
  used, as it is twice in this bullet. It cannot see a string a tool builds as it runs,
  which is how the driver came to print `29 xml-tree` as though that were the goal's name; AGENTS.md
  § *The schedule is the chain* is the rule's home, and it binds both.
- Crates for later milestones are created when their milestone starts, not left sitting empty.

## Length targets, and why nothing enforces them

These are the shapes that keep a doc readable at a glance. **Every one is guidance addressed to you, not
a check.** Nothing in this repository, in `bun nv brief`, in `bun nv orient`, or in CI measures a line,
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
| `handoff.md` | ~60 lines — state, not a changelog and not the playbook |
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
context budget `bun nv orient --audit` prints: it is a number to look at when you write a goal, never an
exit code, and never something to trim prose against mid-run.

The one exception is a gate on **growth**, not on size. `bun nv session --wrap` refuses an edit
that leaves a plan field both over its ceiling — the plan's header comment is the home of that number, as
of the aim — *and* bigger than it was. A field that shrinks or holds its size is always taken, so there
is nothing to shave to clear it: adding a sentence to a full field costs dropping one, which is what
"overwrite a field in place" meant all along. It exists because `Open now` reached 52 KB — 46% of the
orientation pack — under the advisory note it replaces, and `--check` and `--template` print the headroom
ahead of the wrap so a session that reads either never meets the refusal at all.

The one structural rule that *does* still matter is not about length: `bun nv brief` and
`bun nv orient` may only print text bounded by a **count of entities** — one line per milestone, per
rule, per record, per guard test, per module in scope — never by a length of prose.
