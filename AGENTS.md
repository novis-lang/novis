# Working on Novis

Novis is a JIT-compiled, memory-safe language for web servers and the command line. It exists to run web
requests and CLI programs **securely and fast**.

This file is inlined into every agent's context before it does anything, so it holds only what cannot be
looked up on demand: the routing table's three entries, the priority ordering, the rules whose whole cost
is that you did not know they existed, the one rule nothing can check — how text an end user reads is
written — and the session workflow. **Everything else is one call away.**

**Every fact in this repository has exactly one home.** If two documents state the same thing, the one
named as the home is authoritative and the other is a bug — fix it rather than reconciling it in your head.

## Where to look

Do not read the docs tree breadth-first: most of it is reasoning you only need when you are about to
overturn a decision. **One call orients you**, and which one depends on why you are here:

| You are | Run |
|---|---|
| A session of the unattended loop | Nothing — the driver ran `bun nv orient` and piped the pack in ahead of your prompt: everything below, narrowed to the current goal's `[context]` manifest and its current item. Run it yourself only if it is genuinely absent. |
| Working interactively, on anything | `bun nv orient --full` — the same pack with no narrowing: where the run and the work stand, one line per module, the rules, the shapes and the plan's fields |
| Looking for the file that owns a topic | `bun nv brief --where <keyword>` — a search over the rulebook: the chapters under [docs/rules/](docs/rules/) and every rule's title, plus the homes that are not rules — the plan, the tools, the benches, the perf notes. With no keyword, the chapter list |

Those three route to everything else. The five files behind them, none of which is read in full by default:

- **[docs/ground-rules.md](docs/ground-rules.md)** — generated, one line per rule, each linking the rule
  it states. The rule's fragment under `docs/rules/<topic>/` is the rule; the chapter
  `docs/rules/<topic>.md` that renders it is what you read.
- **[docs/agent/commands.md](docs/agent/commands.md)** — how this repo is driven: `nv peek`, `nv verify`,
  `nv session`, `nv splice`, `nv plan`, `nv disk`, WSL, valgrind, and the two shell rules below in full.
- **[docs/agent/doc-style.md](docs/agent/doc-style.md)** — how to write anything in `docs/`, and the length
  targets nothing enforces.
- **[docs/agent/conventions.md](docs/agent/conventions.md)** — the *shape* of a commit message, a `.nvst`
  case, a `Core` member, a decision record, a diagnostic. Read this instead of opening an example to
  copy.
- **[docs/agent/grounding.md](docs/agent/grounding.md)** — what an answer owes the person who asked for
  it: how a claim carries its evidence, and why a proposal is verified before it is offered rather than
  when it is questioned.

**A rule's fragment under `docs/rules/` is the rule, and it is always currently true.** A record under
`docs/decisions/` is frozen rationale, reached through a rule's `because`, and is never edited to track a
later decision. If a fragment and a record disagree, the fragment is the rule and the record is history.

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
- If a change spends memory, **say what it spends** — per request or per task — in the doc comment or
  decision record that records it.
- Memory must stay attributable to a request and under an enforceable cap, and must be O(in-flight) rather
  than O(requests served). Growth with total traffic is a leak, not a trade-off.
- Bytes *moved* are not cheap. An allocation or extra cache miss on a hot path is a latency question
  (priority 3), not a footprint one.
- Saving memory at the cost of an invariant every future contributor must remember is the wrong direction —
  that is the account the unsafe modules are already drawing on.

## The rules you will otherwise break

Each is one sentence here because not knowing it exists is the entire cost. The mechanism, and why, is in
[docs/agent/commands.md](docs/agent/commands.md), except rules 7 to 9, whose homes are
[docs/agent/conventions.md](docs/agent/conventions.md) § *A code comment*,
[docs/agent/grounding.md](docs/agent/grounding.md) and `rule:testing/feature-proofs`, and rules 10 and 11,
which are whole where they stand.

1. **A shell never carries file content into the tree.** Create and edit files with Write and Edit — never
   a heredoc, a `>` redirect or a `sed -i`, because the shell parses your apostrophes and backticks before
   it runs anything. An edit those tools cannot express goes through `bun nv splice`.
2. **One shell call runs one command** — a `;`-chain reports only the last one's exit status. Independent
   calls may go out together in one message, but measured sessions never do it, so use the two tools that
   batch for you instead. **Read with `bun nv peek A.rs:120-160 B.rs:@sym C.md:"## 4"`** — as
   many targets as you have questions, one call, and `--locate <symbol> ...` for `file:line` anchors
   alone. **Write a run of three or more edits with `bun nv splice --patch <file>`** — one patch,
   any number of files, all of it or none of it.
3. **Read a big file in the region you need.** Whole file under ~400 lines; past that, `grep -n` for the
   anchor and read around it. Context, not the clock, is what caps a session.
4. **Verify with one call, once, at the end:** `bun nv affected --run`. It names what your change
   reaches, runs `bun nv verify` — build, fmt, test, the `.nvst` trees and clippy, each step only when
   what it reads changed — and then only the acceptance checks your change reaches that are not green
   yet. A loop session runs `bun nv verify` alone, because the driver sweeps the rest. Never choose a
   wider command yourself: `nv guard` refuses one, and `bun nv affected` without `--run` says what
   would run and why.
5. **A debug cargo command never takes `-p`.** `cargo build`, `cargo test` and `cargo clippy
   --all-targets` are the whole tree and warm in seconds; a `-p <crate>` resolves features over that one
   package and writes a second copy of every workspace crate beside the first, and a day of those
   filled the disk. Narrow what *runs* — `bun nv verify -p <crate>`, or a `--test <name>`
   or `--lib` filter under `cargo test` — and leave `--release -p` to the cost guards.
6. **Finish with one call:** `bun nv session --wrap <file>` applies steps 4 and 5 below — plan
   fields, playbook bullet, handoff, one commit per slice, status — or refuses and changes nothing.
7. **A comment says what the code does now, and is rewritten as a whole** — never edited by leaving the
   old sentence beside the new one, so no comment ever reads as a changelog. `git log` is the only
   history this repository keeps: no dates, no counts of things that can change, no measured figures
   the code does not enforce. A comment an end user reads is the exception, and § *Text an end user
   reads* below is its rule.
8. **Verify a claim before you make it, not when it is questioned.** Every claim carries a `file:line`
   or the words *not checked*, and advice you volunteered meets the same bar as the answer — a
   follow-up that sends you to the code and changes what you said means the answer went out early.
9. **A feature is finished when its feature proofs exist, not when it works** — `about.md`, tests from
   Novis and from Rust, three examples, one bench, one attack, and its help in the binary. `bun nv proofs --id '<feature>'`
   prints what it still owes and the path of each, and every `.nvs` and `about.md` among
   them is held to § *Text an end user reads* below.
10. **Nothing is written outside the project's folders.** Every scratch file — a patch, a probe script, a
   throwaway case, a commit message, a log — goes under `.agent-tmp/` at the repository root, which is
   git-ignored and is where `nv verify` already writes its logs. Never the system temp directory, a
   harness's own scratchpad or a home directory, whatever a harness offers by default: the user set this
   for every agent and every session, and a file outside the tree is one they cannot see, review or
   clean up. **Whoever writes a scratch file deletes it when the work it served is done** — a session
   before it stops, a subagent before it reports, a test in its own `afterAll` or `finally` — and leaves
   only what a tool reads back on its next run (the `verify-*` logs and timings, `peek-ledger.json`,
   `select-rec/discard/`, `bg/`). Delete only the files you wrote yourself: another
   agent may be working in the tree at the same time. A git worktree is scratch too: it goes under
   `.agent-tmp/worktrees/<branch>`, and the agent that made it removes it and deletes its branch the
   moment that branch is merged into `main`, with `git worktree remove` and `git branch -d`, never a
   forced form: one of them refusing means work would be lost, and that is a question for the user.
11. **Nothing a reporter told you is written into the tree under its own name.** A bug report, a pasted
   configuration or a log names somebody's modules, namespaces, directories, hosts, URLs and counts, and
   all of it is theirs: the test, the decision record, the rule, the example, the comment and the commit
   message that come out of the report use neutral names instead — `Blog`, `Shop`, `Framework`,
   `example.com` — and say "many" where the report gave a figure. Reproduce the *shape* of what was
   reported, never its words: this repository is public, and a name copied from a report publishes a
   piece of somebody's private codebase.

## Text an end user reads

**This section is the whole rule, and it is here because nothing checks the wording: it is followed at
the first write, not repaired afterwards.** It covers **everything a user sees**: every page of the
website, every `about.md`, every `Core` reference card (`rule:core-api/reference-card`), every comment
in a `.nvs` file the website publishes — under `docs/examples/`, `tests/hostile/`, `benches/members/`
and `website/snippets/` — the help text `nvs` prints for a command or a flag, and the message of every
diagnostic. Help text and diagnostics are held to it when they are written or changed. The docs only
agents read stay out of it: the rules, the decision records, the goals, the files under `docs/agent/`
and this file. Everything it covers is read by somebody who looked a feature up and has never seen this
repository. **A beginner and an expert should both read it once and come away with the same picture.**

**Write it the way a good manual does, not the way this file does.** This file, the orientation pack,
the rules and the goals are written in a dense essay voice, and an agent that has just read them writes
comments in it. That voice is the thing to avoid. Many readers do not have English as their first
language, and the text has to work for them on the first read.

- **Say what the line does, then what the result is.** That is the whole comment, in that order:
  "`as ?int` converts the value to a whole number. If that is not possible, the result is `null`."
  Add why somebody wants this only when it is not obvious. Name the real value where there is one —
  "the result is `null`", "this prints `3`" — so the reader can check it against the output.
- **The subject is the code or the reader.** "`Core\Str::length` returns …", "This loop adds …",
  "You can change …". Code does not *ask*, *answer*, *hand back*, *refuse*, *decide*, *hold* or *know*,
  and a value does not *become a surprise*. It returns, gives, prints, throws, stops, is, has.
- **Say what happens, not what does not.** "The result is `null`" — not "`null` rather than a
  silently wrong number". No "rather than" at all: a comparison makes the reader hold two ideas to get
  one. Where the difference really is the point, give it a sentence of its own: "PHP returns `3` here."
- **Use the word programmers already know.** cast, syntax, method, function, variable, returns,
  throws an error, `null`, loop, string. These are the easy words for this reader, and a paraphrase
  of one is harder than the term. This repository's own vocabulary stays out:

  | Not | Write |
  |---|---|
  | spelling | syntax, "the way to write" |
  | answers, hands back, hands you | returns, gives |
  | asks, asking, asks for | checks, tests, needs |
  | stands in for | replaces |
  | lets go of, drops | frees, deletes |
  | refuses, is refused, a refusal | throws an error, does not compile, is not allowed |
  | member | method, function |
  | binding, name | variable |
  | holds | is, has, contains |
  | mark, marked | say what it is: "tainted", "secret", and explain it once |
  | shard, tier, slot, refcount, lowering, the registry | nothing — the reader never needs these |

- **No idioms and no figures of speech.** Not "a wall of emoji", "goes through", "at the edge",
  "further down", "earns its place", "on purpose", "the rest of that family". If a phrase would not
  survive a word-for-word translation, write the literal thing.
- **Short sentences, one idea each.** No sentence over 25 words, and no dash joining two sentences:
  write two. If a sentence has to be read twice, it is two sentences.
- **A name the reader will type goes in backticks** — the setting, the class, the method, the error
  they will catch. A term they need and may not know gets a few plain words the first time:
  "a grapheme (what a person counts as one character)".
- **PHP is mentioned once, at most, and at the top.** Most readers looked up a Novis feature, not a
  migration guide. "now that casts are gone" is history and tells them nothing they can use.
- **No internals and no history.** No ADR numbers, no rule ids, no crate or Rust names, no
  milestone, nothing about how the behaviour came to be.
- **Keep it short.** Up to four lines at the top of the file, one or two lines above a step. A
  configuration example may also show the block it is about. What does not fit belongs in
  `about.md`, and a comment never repeats what `about.md` already says.

What the top comment says depends on the tree, and the rest is the same everywhere:

| Tree | The top comment |
|---|---|
| an example | one sentence: what this program shows |
| an attack | `// Attack:` and then what it tries, and what should happen instead, in one or two plain sentences; each numbered step gets one line saying what it tries |
| a bench | one sentence: what is measured, and where somebody meets it in real code |

A directive line — `// bench:`, `// hostile:`, `// covers:`, `// proof:`, `// requires:` — is read by
a tool, is not prose, and is left exactly as its own README spells it.

The same comment, first the way it goes wrong and then the way it is written:

```nvs
// Unlike almost everything else in a configuration file, this one is a program's
// to change while it runs — a job that knows its own work is quick can say so,
// and the answer is `yes` rather than a silent refusal.
```

```nvs
// Your program may change this setting while it runs. Most settings do not allow that.
```

One that was already short, and still hard:

```nvs
// The conversion is checked. A price with a fraction is not a whole number,
// so asking with `?int` answers `null` rather than losing the fraction.
```

```nvs
// `as ?int` converts a value to a whole number. 3.9 is not a whole number,
// so the result is `null`.
```

And an attack:

```nvs
// Attack: the ceiling is the only thing standing between a request and as much
// of this host as it cares to ask for, and it is one longest-match row away
// from the block the request *is* allowed to write.
```

```nvs
// Attack: a request tries to raise its own memory limit. Only the person who runs
// the server may set that limit, so every attempt below should fail.
```

If a comment would only make sense to somebody who works on Novis, it is the wrong comment.

`bun nv proofs --comments <paths>` judges the three bounds a script can count — lines in a
block, words in a sentence, a dash joining two — and nothing about the words. Passing it says nothing
about whether the comment is plain; a line it names is written again from what the code does, never
trimmed until it passes.

## Session workflow

Every session runs the same five steps, in this order, and **stops**:

1. **Orient in one call** — already done for you in the loop (the driver pipes the pack in with the
   prompt), `bun nv orient --full` interactively.
   That is the map, where the work stands, the traps that apply to these files, the shapes you are about to
   write, and the rules this goal lives inside. It is narrowed on purpose: if you find yourself needing
   something it did not print, that is a gap in the goal's `[context]` manifest — say so in the handoff.
2. **Do the work — as much of the group as fits under the context ceiling.** The handoff names a group of
   related slices and the file set they share. **Keep taking slices from that group while both hold: the
   next one touches files already loaded, *and* you are under 120k with the previous one committed. Stop at
   the first slice that fails either test.** Context is the binding budget here, not the clock: an agent
   degrades well before its window is full, so the ceiling is a fixed **200k**. **This paragraph is the
   cap's only home** — every other file points here rather than restating a number.

   **The gate is the budget, not a count of slices.** A session pays a fixed cost — orienting, verifying,
   wrapping — that is the same for a three-line slice as for a three-hundred-line one, so small slices over
   one file set share a session and one hard slice fills it alone. `bun nv loop-stats` reports that fixed
   cost and where the next session will open; read them there, because this file copies no measured figure.
3. **Verify what you touched, once, at the end of the group** — `bun nv verify` in the loop, `bun nv
   affected --run` anywhere else, plus whatever the change specifically warrants (a `valgrind` run for a
   new refcount edge). **This is the only place
   verification happens**, and a group shares one run: the build is the same build.
4. **Write the docs and the handoff, once for the whole group.** The plan's status block, a playbook bullet
   if a trap cost you time, and the live goal's handoff overwritten with where the work stands now. The
   handoff is *state* — a fact that will still be true in ten sessions belongs in the playbook, a rule, or
   a crate's module doc instead. Naming the **next** group, and the file set it shares, is this step's job:
   you are the only one holding the context to decide it cheaply, and `bun nv peek --locate` turns
   its `file.rs:NN` anchors into one call.
5. **Commit — one per slice, all after step 3 is green**, staging each slice's own files so `git log`
   still reads a slice at a time. Then you are done.

**Steps 4 and 5 are one call.** Write a single wrap file — plan fields, playbook bullet, handoff, a
`## commit:` per slice, the status line — and apply it with `bun nv session --wrap <file>`
(`--help` is the format, `--check` first says what the tree still owes). It applies everything or refuses
everything, so there is no half-written tail.

**After step 5, stop.** Do not re-run `cargo build`/`test`/`clippy`/`fmt`, do not re-read the orientation,
do not re-check a doc against a length. Writing prose cannot break a build, so there is nothing a second
test run could discover. If step 4 or 5 turned up a real problem, fix it and re-verify *that* — otherwise
the session is over.

## Keep each slice small, commit every one of them

Step 5 above, in detail:

- Always commit your work before you exit, and never leave a slice uncommitted. Stage each slice's own
  files, and let the last commit sweep whatever is left.
- The handoff is the live goal's record `data/goals/<slug>.handoff.json`, written by the wrap file's
  `## handoff` section. **Overwrite it**, never append: it says where the work stands now. Its shape is
  [docs/agent/session-prompt.md](docs/agent/session-prompt.md) § *The handoff's shape*. Then show the
  user the same prompt in chat.
- **The playbook is the opposite file**: traps, not history ([docs/agent/playbook.md](docs/agent/playbook.md)).
  Add a bullet in the shape [conventions.md](docs/agent/conventions.md) § *A playbook bullet* gives when a
  trap costs you time, edit one when it stops being true, and never reword one or write a session's story
  into it.
- The process docs drift as decisions land. Periodically — the user fires this by hand, never you
  automatically — re-run the pass in [docs/agent/doc-cleanup.md](docs/agent/doc-cleanup.md).
- Every time we add, change or remove a feature, decide and say what the tradeoffs are in performance,
  memory, usability and simplicity for developers using the language. If there are large tradeoffs, notify
  the user and ask for agreement before proceeding. If there are only benefits, go ahead.
- **The plan** is [docs/implementation-plan.md](docs/implementation-plan.md), an index whose status block
  has a fixed field set — `Status`, `Done`, `On disk`, `Toolchain`, `ADR slices landed`, `Open now`,
  `Blocking` — plus one file per milestone under [docs/plan/](docs/plan/) and the frozen design in
  [docs/plan/design.md](docs/plan/design.md). Overwrite a field in place; never append a paragraph or add
  a field. Known-gap detail belongs in the crate's module doc. `bun nv plan --show M8` prints a milestone,
  `--show M8:verify` its acceptance paragraph, and a `## milestone: M8` wrap section rewrites one.
- **The schedule is the chain, not the milestone table.** `data/chain.json` holds `goals`, the slugs in
  the order the loop walks them, and `live`, the goal the driver works on and moves on when one is
  reached. A goal is its prose at `docs/agent/goals/<slug>.md` and its record at `data/goals/<slug>.json`.
  A milestone is an identity tag goals carry, so its number says nothing about what is next. **A side
  goal is off the chain** and run by hand in a worktree of its own —
  [goals/README.md](docs/agent/goals/README.md) § *Side goals*.
- **Name a goal by its slug, never by its number** — in prose, a comment, a commit message, an owner
  column and what a tool prints. A position beside a total (`29 of 43`) is fine; a position moves when a
  goal is inserted, and a slug never does. `bun nv chain --check` fails on a goal named by its number.
- **`bun nv chain` reorders the chain**: `--new`, `--move` or `--remove <slug>` with one place —
  `--after <goal>`, `--before <goal>`, `--to <position>`, `--next` or `--end`. It edits `goals` only,
  never `live`; a goal's record and prose are written by hand around it.
