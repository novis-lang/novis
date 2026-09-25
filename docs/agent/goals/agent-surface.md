---
milestone: M10
---
# Loop goal 28 — An agent learns Novis from the binary, in three calls

An agent that has never seen Novis reads about two thousand tokens once, then pays a few hundred per
lookup, and writes code that compiles — instead of reading a repository it cannot fit. `nvs agent`
answers from the registry the binary already carries: a generated primer, one line per member, a
search over those lines, and one member's card. `nvs agent init` puts a pointer to that surface in a
user's project, one short file per harness, none of which states a language fact of its own.

It sits here, three entries after the live goal, because every goal after it is a consumer: a session
writing a `.nvst` case or an example currently greps the spec for a member's spelling, and this
replaces that with one bounded call. The benefit compounds over everything that follows, which is the
only argument strong enough to put an entry that *adds* surface in front of ten that close gaps.

Goal `record-origin`'s whole acceptance list is this goal's floor, and it is never traded.

## Why here

The user's, after asking how an agent will write a language no model has distilled. Three agents
were given the same task and three different documents; all wrote correct Novis quickly, and all
then lost most of their budget to one wall — the one table declaring what each member needs, shipped
and tested and rendered by nothing, which stage 0 audits and stage 2 finally reads.

## What is wrong today, in one line each

Read out of the tree rather than inferred. Items 1–3 are what
[0167](../../decisions/0167.md)'s three-arm investigation measured; item 4 is a shipped rule with no
consumer, found while writing it.

1. **The complete reference cannot be read.** `docs/novis.md` is ~1.2 MB — more than a context
   window — and it is correct by construction, which is why the answer is a smaller *selection* and
   never a second document. Stage 3.
2. **There is no way to ask for one member.** `nvs meta --json` is ~604 KB and all-or-nothing;
   `docs/novis.md` is greppable only by someone who already has it on disk. An agent wanting
   `Core\Str::length`'s card has no call that returns just it. Stage 2.
3. **An unknown member suggests nothing.** `Core\Str::lenght` reports `E0309` from
   `crates/nvs-hir/src/members.rs:1014` with no *did you mean*, while the registry holding `length`
   is in the same process. Stage 4.
4. **`rule:security/capability-declaration-is-one-table` is `shipped` and names two consumers that do
   not exist.** It says the metadata command renders `crates/nvs-stdlib/src/registry.rs`'s
   `CAPABILITIES` and the reference prints it beside a member's card.
   `crates/nvs-cli/src/meta.rs` never reads the table, `nvs meta --json` has no `capabilities` key
   beside its six, and `docs/novis.md` prints the word beside no card. Stage 0 and stage 2.
5. **A capability denial says what happened and not what to do.**
   `crates/nvs-runtime/src/capability.rs:109` names the member, the capability and the resource, and
   never names `nvs.toml`. Every arm of the investigation was stopped here, one of them for half its
   total effort. Stage 4.

## Stage 0 — the catch-up

Item 4 above, and only its audit half. Confirm against the tree that the table has no renderer, then
decide **which document the roster joins** — `nvs meta --json`'s own, as a seventh top-level roster
beside `exceptions`, `interfaces`, `attributes` and `directives`, which is
[0167](../../decisions/0167.md) § 3's answer and the shape the other four already have.

Two rules in this stretch are marked `designed` while the binary ships them —
`rule:tooling/nvs-doc-renders-and-decides-nothing` (`nvs doc <entry> --out <dir>` writes a page per
class today) and `rule:tooling/meta-json-takes-a-program` (`nvs meta --json <entry>` emits the
`program` key today). Verify each against the binary and flip the ones that hold. A rule that turns
out not to hold is written down and left `designed`; it is not this goal's work to make it true.

Nothing else on disk contradicts this goal's rules — the four they create are new.

## Stage 1 — the floor

Goal `record-origin`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded for
anything above it.

## Stage 2 — the keystone: the registry answers a query

File set: `crates/nvs-cli/src/main.rs:186` (the `Command` enum), `crates/nvs-cli/src/meta.rs`, and a
new `crates/nvs-cli/src/agent.rs`. `crates/nvs-stdlib/src/registry.rs` is read and not edited.

1. **`crates/nvs-cli/src/meta.rs`** — the document gains `capabilities`, the `CAPABILITIES` table
   rendered as a roster: class, member, capability. Read from
   `crates/nvs-stdlib/src/registry.rs`; **no field is added to a member row**, which
   `rule:security/capability-declaration-is-one-table` refuses and this goal does not reopen.
2. **`crates/nvs-cli/src/agent.rs:@index`** — one line per member, joined to that roster:
   `Core\IO::read(string $path): string  [fs.read]`. One line per enum with its cases, per exception
   with its parent, per attribute. Nothing cached, nothing written to disk.
3. **`crates/nvs-cli/src/agent.rs:@find`** — the index lines matching a query, case-insensitive over
   the class and member name. It exists because a namespaced name loses its backslash to the shell
   before `grep` sees it, and the empty result that follows cannot be told from a name that does not
   exist.
4. **`crates/nvs-cli/src/agent.rs:@show`** — one member's card: signature, each parameter with its
   description, the return, what it throws, and its capability. A symbol that resolves to nothing
   exits non-zero naming the nearest matches rather than printing an empty card.
5. **`crates/nvs-cli/src/main.rs`** — the `Agent` subcommand and its four verbs, plus `init`, whose
   body is stage 5's.

`docs/reference/tools/10-cli.md` gains each command as it lands; that chapter is what puts them in
`docs/novis.md`.

## Stage 3 — the primer, generated, and every claim in it executed

File set: `crates/nvs-cli/src/agent.rs`, `docs/reference/lang/*.md` (markers only), and
`tools/reference.py`.

1. **A chapter marks primer material** with a comment on a line of its own, the mechanism
   `tools/reference.py` already uses for its generated tables — `<!-- primer -->` above a section
   lifts that section whole. Marking is how a chapter opts in, so a section that stops being true
   stops being rendered rather than becoming a lie.
2. **`crates/nvs-cli/src/agent.rs:@primer`** renders, in this order: the lookup protocol; one
   complete worked program with a typed local, a `foreach` binding, an options bag at a call site
   and a conversion, each annotated; the capability model and the smallest `nvs.toml` that grants a
   file read; the refusal table; the chapter map from the chapters' own front matter.
3. **The content is fixed by what the investigation caught**, and each of these was a wrong guess by
   an arm that had everything else: the options bag has no call-site spelling anywhere
   (`{header?: bool}` reads as named parameters and is one `options:` shape argument), a `foreach`
   binding declares a concrete type and `var` is refused there, and a program that reads a file does
   not run without a grant.
4. **Every refusal the primer states is fed to `nvs check` and must be refused; every example runs
   and must print what the primer says.** The harness is `tools/reference.py`'s, which already does
   exactly this for `docs/novis.md`'s examples — reuse it rather than writing a second one.

## Stage 4 — the two diagnostics

File set: `crates/nvs-hir/src/members.rs:1014` and `crates/nvs-runtime/src/capability.rs:109`.

1. **An unknown member suggests the nearest registered name.** Edit distance over the class's own
   members, one suggestion, and none at all past a threshold — a confident wrong suggestion is worse
   than none, because the agent will take it.
2. **A capability denial names where a grant is written**: a `help:` line naming `nvs.toml` and the
   `[capabilities.fs]` table for the capability that was refused. The message keeps its subject and
   its wording; this is a line under it.

## Stage 5 — `nvs agent init`, and the adapters

File set: `crates/nvs-cli/src/agent.rs`, and a new `docs/reference/tools/50-agents.md`.

1. **`nvs agent init`** writes an `AGENTS.md` stanza, and beside it one adapter per harness it finds
   — a Claude Code skill at `.claude/skills/novis/SKILL.md` when `.claude/` is present. `--all`
   writes every adapter regardless; re-running is idempotent and refuses to clobber a stanza a user
   has edited.
2. **No adapter states a language fact** — not a signature, not a refusal, not a type. Each names the
   four commands and the `nvs check` loop. This is the whole of
   `rule:tooling/an-adapter-carries-protocol-and-never-language` and the reason an adapter needs no
   maintenance when the language changes.
3. **`docs/reference/tools/50-agents.md`** is the chapter: what the surface is, the install, and a
   worked session showing the three calls and the check loop. It documents only what stages 2–5
   landed, and its examples run like every other chapter's.
4. **This repository installs it too.** `AGENTS.md` gains the stanza, so the loop's own sessions stop
   grepping the spec for a member's spelling.

## Stage 6 — the rulebook

`rule:tooling/an-agent-asks-the-binary`, `rule:tooling/the-index-is-one-line-per-member`,
`rule:tooling/a-primer-claim-is-executed` and
`rule:tooling/an-adapter-carries-protocol-and-never-language` move from `designed` to `shipped`, and
their `guardedBy` names the cases stages 2–5 landed.

`rule:tooling/meta-json`'s fragment is edited here and not earlier: its body says **"Four rosters the
compiler declares outside the registry sit beside `classes` and `enums`"**, and stage 2 makes that
five. The rule is `shipped`, so the sentence stays true until the fifth exists.
`rule:security/capability-declaration-is-one-table` keeps its text — stage 2 gives it the renderer it
always claimed, so nothing in it changes.

## Standing decisions

- **The surface is `nvs agent`, not an extension of `nvs doc`** ([0167](../../decisions/0167.md)
  § 1). `nvs doc <entry>` takes a positional path and a symbol is not a path; a query form under it
  would be ambiguous at the argument parser before it was ambiguous to a reader.
- **Nothing is written to disk and nothing is cached** ([0167](../../decisions/0167.md) § 1). If a
  stage finds a render slow enough to want a cache, that is a finding to write down, not a cache to
  add: an answer that can be stale is the failure this whole surface exists to avoid.
- **No per-member capability field** ([0167](../../decisions/0167.md) § 3, and
  `rule:security/capability-declaration-is-one-table`'s own second paragraph). The join is at render
  time. If the join turns out to be awkward, the fallback is to render the roster and let the reader
  join — never to push a field onto the member rows.
- **The primer is generated and never hand-written** ([0167](../../decisions/0167.md) § 2). A
  section that cannot be expressed as a marked chapter section is a signal the chapter is missing it,
  and the edit belongs in the chapter.
- **No size gate on the primer** ([0167](../../decisions/0167.md) *Alternatives rejected*). Its
  budget is met by what it selects. Do not add a byte-count check, and do not trim a selected
  section's prose to hit a number.
- **An adapter carries no language content**, in any harness, for any reason
  ([0167](../../decisions/0167.md) § 4). A stage that finds an adapter would be more useful with a
  refusal table in it has found the argument this rule already rejected.
- **`nvs agent init --embed` is not built** ([0167](../../decisions/0167.md) *Revisiting*). Nothing
  has asked for it and it reintroduces the staleness the design removes.
- **This goal opens no ADR number.** [0167](../../decisions/0167.md) is already accepted and its
  `changes:` block names the four rules it creates and the two it modifies.
- **Static capability checking is not in scope.** `nvs check` never building the grants its own
  diagnostic needs is a carried gap owned elsewhere; stage 4 improves the *runtime* message and
  nothing more.
