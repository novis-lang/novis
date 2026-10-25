# Working on Novis

Novis is a JIT-compiled, memory-safe language for web servers and the command line. It exists to run web
requests and CLI programs **securely and fast**.

This half of the file holds only what looper's managed block below it does not say: the priority
ordering, the rules only this project has, how text an end user reads is written, what a loop session
owes on top of the block's *Session workflow*, and where Novis departs from the stack files. The block
holds the routing table, the shared rules, the context budget, the session workflow, goals and commits.

## The priority ordering

Highest first. A lower item is spent to buy a higher one, never the reverse. Reasoning and bounds:
[`rule:programs/memory-priority`](docs/rules/programs.md#programs-memory-priority).

1. **Security and request isolation** — not traded for anything.
2. **Latency and throughput on the request path.**
3. **Simplicity** — of the language surface first, then of the implementation.
4. **Memory footprint** — last, and spent deliberately to buy any of the above.

Cite a priority by its name, never its number. What a program observably does is not in the ordering:
the rules state it and the conformance cases pin it.

When choosing between designs:

- Prefer the safer, faster or simpler one even when it holds more memory. Novis is **not** a low-footprint
  runtime; "allocates less" is not on its own a reason to change anything.
- If a change spends memory, **say what it spends** — per request or per task — in the doc comment or
  rule fragment that records it.
- Memory must stay attributable to a request and under an enforceable cap, and must be O(in-flight) rather
  than O(requests served). Growth with total traffic is a leak, not a trade-off.
- Bytes *moved* are not cheap. An allocation or extra cache miss on a hot path is a latency question,
  not a footprint one.
- Saving memory at the cost of an invariant every future contributor must remember is the wrong direction —
  that is the account the unsafe modules are already drawing on.

## Novis works without looper

**Every Novis developer, and CI, can do all of the project's work without looper installed.** The
block's § *The project carries its tools* is the rule. What Novis adds to it:

- **Every gate is written once in `bun nv`:** the build, the tests, verify, the guard, the prose and
  commit checks, and the doc and layout checks. looper's copy runs each through `looper.toml`'s
  `[verify]` and `[gates]`. Where looper's check does more, extend `bun nv` and name it there.
- **A cleanup of this file keeps every `bun nv` command it names.** Where no `looper` program is
  installed, `bun tools/looper/main.ts <command>` runs each `looper` command the files name.
  [commands.md](docs/agent/commands.md#without-looper) § *Without looper* says what runs differently
  without looper's service.

## The rules only this project has

Each is here because not knowing it exists is the entire cost. The file named after each is its home.

- **Find the file that owns a topic with `bun nv brief --where <keyword>`.** It searches the chapters
  under [docs/rules/](docs/rules/), every rule's title, and the homes that are not rules: the goals,
  the tools, the benches and the perf ledgers. [docs/ground-rules.md](docs/ground-rules.md) is one line
  per rule. A rule's fragment under `docs/rules/<topic>/` is the rule, and it is always currently true.
  There are no decision records: a rule holds its reasons and its Declined list, and `bun nv rules
  --check` fails on a record or a citation of one.
- **A debug cargo command never takes `-p`.** `cargo build`, `cargo test` and `cargo clippy
  --all-targets` are the whole tree and warm in seconds. A `-p <crate>` resolves features over that one
  package and writes a second copy of every workspace crate, and a day of those filled the disk.
  Narrow what runs instead: `bun nv verify -p <crate>`,
  or `--test <name>` or `--lib` under `cargo test`. `--release -p` is the cost guards' alone
  ([commands.md](docs/agent/commands.md) § *A debug cargo command never takes `-p`*).
- **Read with `bun nv peek`, and write a run of three or more edits with `bun nv splice --patch
  <file>`.** `bun nv peek A.rs:120-160 B.rs:@sym C.md:"## 4"` reads as many targets as you have
  questions in one call, and `--locate <symbol> ...` gives `file:line` anchors alone. One patch edits
  any number of files, all of it or none of it ([commands.md](docs/agent/commands.md) § *One shell call
  runs one command, and its exit status is the last one's*).
- **A goal is verified once, in its last stage.** Every other stage checks with `cargo build` and
  `cargo clippy --all-targets -- -D warnings` alone. A test you wrote may run once, by name, and a run
  a stage's item names as its requirement runs too; nothing else does. The gate stage runs the full
  gate, and every failure it names is fixed in one pass before it runs again.
  `bun nv verify --fast -p <crate>` is for that fix pass, to rerun one crate's tests
  ([commands.md](docs/agent/commands.md) § *Verifying*).
- **The goal's gate is `bun nv affected --run`.** It runs `bun nv verify` over what the change
  reaches, then the acceptance checks it reaches that are not green yet. In the loop, `looper verify`
  runs `bun nv verify` and joins a run already going ([commands.md](docs/agent/commands.md)
  § *Verifying*).
- **The guards refuse a shell habit a tool covers, and each refusal names the call to make instead.**
  A `>` redirect into the tree, a whole read of a big file, `cargo test -p`, `sleep` and a run wider
  than the change are among them ([commands.md](docs/agent/commands.md) § *What the guards refuse*).
- **A scratch file is deleted by whoever wrote it, and a worktree by whoever made it**, once its work is
  done, and a worktree never with a forced form ([commands.md](docs/agent/commands.md) § *Scratch files
  and worktrees*).
- **A feature is finished when its feature proofs exist, not when it works**: `about.md`, tests from
  Novis and from Rust, three examples, one bench with its growth, one attack, and its help in the
  binary. `bun nv proofs --id '<feature>'` prints what it still owes and the path of each, and every
  `.nvs` and `about.md` among them is held to § *Text an end user reads* below
  (`rule:testing/feature-proofs`).
- **A `HANG:` line stops what you are doing until you know what hangs.** Every program `bun nv` starts
  is watched, and a tree that stalls or spins with no output is killed early, with a report under
  `.agent-tmp/hangs/`. Read the report, run the hung unit alone, and fix the cause before the slice
  goes on. A longer bound is a fix only for a program that waits on purpose
  ([commands.md](docs/agent/commands.md) § *A hang stops the slice*).
- **Nothing a reporter told you is written into the tree under its own name.** A bug report, a pasted
  configuration or a log names somebody's modules, namespaces, directories, hosts, URLs and counts, and
  all of it is theirs. The test, the rule, the example, the comment and the commit message that come out
  of the report use neutral names instead — `Blog`, `Shop`, `Framework`, `example.com` — and say "many"
  where the report gave a figure. Reproduce the *shape* of what was reported, never its words: this
  repository is public, and a name copied from a report publishes a piece of somebody's private
  codebase.

## Text an end user reads

**This section is the whole rule, and it is here because nothing checks the wording: it is followed at
the first write, not repaired afterwards.** It covers **everything a user sees**: every page of the
website, every `about.md` and `details.md`, every `Core` reference card (`rule:core-api/reference-card`), every comment
in a `.nvs` file the website publishes — under `docs/examples/`, `tests/hostile/`, `benches/members/`
and `website/snippets/` — the help text `nvs` prints for a command or a flag, and the message of every
diagnostic. Help text and diagnostics are held to it when they are written or changed. The docs only
agents read stay out of it: the rules, the goals, the files under `docs/agent/`
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
  | field, for a variable of a class | property; *class property* where the sentence does not already make it clear that it is about a class. "Field" stays for an anonymous object's members, an attribute's arguments, a form, JSON, CSV and a database |
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
  migration guide. "now that casts are gone" is history and tells them nothing they can use. The
  comparison page under *In depth* is the one exception: it names PHP, Bun and Go wherever its tables
  and results need them.
- **No internals and no history.** No rule ids, no crate or Rust names, no
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

## In the loop

What a loop session owes on top of the block's § *Session workflow*:

- **A denied call is not retried in another form.** The loop runs under `--permission-mode auto`, so a
  call outside `.claude/settings.json`'s allowlist is reviewed and may be refused. Do the work with an
  allowed command, or name the call you needed in the handoff.
- **Do not re-read what the pack printed**: the handoff, the stage, the decisions, the traps.
- **A read-heavy search goes to an `Explore` subagent**, which has no Edit or Write tool. Delegate
  *where is X*, *how many Y*, *which syntax the corpus uses*, over files you will not otherwise open;
  never a write, a file you are about to edit, a design question, whether a check passed, or the wrap.
  Its prompt is the question, the place to look and the answer's shape (`file.rs:NN` and one line each,
  no excerpts), never the project, because it reads its own copy of this file.
- **In the gate stage, start `looper verify` with `run_in_background` before you write the handoff,
  then end your turn to wait.** Its report starts your next turn, and nothing polls. It joins the tree's verify run if
  another session started one, so never wait for or kill another session's verify.
- **A design call never stops the loop, however large.** The user takes the session's recommendation,
  so the session picks the option the priority ordering favours and builds on it, as the block's step
  6 says. A change to the `Core` API, a new checker mechanism, a scheduler run-order change and a fix
  that breaks callers are all design calls.
- **Before `DONE`, the goal-end gates.** `bun nv verify --doc` and every broken doc link it names
  fixed, then `bun nv owners --closes <slug>` and every gap it names closed or given another owner,
  then `bun nv ci --check`, each red fixed. It runs in about a minute the checks the goal end's push
  runs, which are half of what CI finds: rustfmt, the proofs gate and the website on a binary built from
  the tree, the license file and the rest. Its proofs gate and website render are also floor checks, so a
  goal is not reached while CI's `proofs` or website render job would be red.
  A reached goal's file is deleted, so first move each decision only its prose holds to a rule or a
  module doc, and turn each check that proves more than a suite does into a test.
- **Every goal end rebuilds the local release binary, `target/release/nvs`.** looper's `after_retire`
  hook runs `bun nv relink --build --hook` once the goal is retired. A goal ended outside the loop runs
  `bun nv relink --build` once its file is deleted.
- **A repair fixes the cause, not the symptom.** The loop's tooling under `tools/` is in scope, and so
  is a check that is itself wrong; renaming a test, removing a check or loosening a bound until it
  passes is not a fix. A verdict already fixed on disk needs no change, and the status line says so.

## Where Novis departs from the stack files

`docs/agent/rust.md` and `docs/agent/bun.md` are looper's defaults, and Novis's rules win in two places.

- **Unsafe code is forbidden in every crate but the unsafe ones.** The workspace `Cargo.toml` sets
  `unsafe_code = "forbid"`. Each crate on `bun nv lints`'s `UNSAFE_CRATES` restates the table with
  `deny` and allows each audited block with a reason. The workspace `Cargo.toml`'s lint comment is
  the home, and the header of `tools/nv/cmd/lints.ts` the reasoning.
- **`package.json` takes its dependencies at caret ranges,** and the committed `bun.lock` pins each one.

<!-- looper:begin agents v5 -->
## Where to look

| You are | Run or read |
|---|---|
| A session of the unattended loop | Nothing. The driver piped the orientation pack in behind your prompt. |
| Working interactively | `looper orient --full`: the live goal, its handoff, its decisions and the traps |
| Working without looper's service | [commands.md](docs/agent/commands.md) § *Without looper*: what runs differently without it |

- **[docs/agent/commands.md](docs/agent/commands.md)**: how the project is driven, and the shell rules.
- **[docs/agent/conventions.md](docs/agent/conventions.md)**: the shape of a commit, a comment, a test
  and text a user reads.
- **[docs/agent/doc-style.md](docs/agent/doc-style.md)**: how to write anything in `docs/`.
- **[docs/agent/prose.md](docs/agent/prose.md)**: how text an end user reads is written.
- **[docs/agent/testing.md](docs/agent/testing.md)**: what to test, and how.
- **[docs/agent/grounding.md](docs/agent/grounding.md)**: what an answer owes the person who asked.
- **[docs/agent/playbook.md](docs/agent/playbook.md)**: traps a session already paid for once.
- **Each stack's own rules** are in `docs/agent/<stack>.md`, such as `docs/agent/rust.md`.

**Every fact has exactly one home.** If two files state the same thing, the home wins and the other is a
bug. Fix the bug rather than reconciling the two in your head.

## The project carries its tools

- **Every `looper` command is the project's own.** The kit's copy in `tools/looper/` runs with Bun
  alone, so every clone and CI run the same checks with the same verdict.
- **The copy is synced, never edited.** `looper sync --from <kit>` writes it from looper's kit, and
  `looper lint` fails on a hand edit. A change to it goes into looper's kit.
- **looper's service only spans projects:** the heavy-job slots, the usage wall, the registry and the
  web UI. [commands.md](docs/agent/commands.md) § *Without looper* says what runs differently without it.

## The rules you will otherwise break

Each is one sentence here, because not knowing it exists is the entire cost. The file named after each
rule is its home.

1. **A shell never carries file content into the tree.** Create and edit files with Write and Edit,
   never a heredoc, a `>` redirect, `Set-Content` or `sed -i` (commands.md).
2. **One shell call runs one command.** Use `&&` between dependent steps (commands.md).
3. **Read a big file in the region you need.** Read a file under about 400 lines whole, and Grep for
   the anchor of a bigger one (commands.md).
4. **A goal is verified once, at its end.** Its last stage, the gate stage, runs `looper verify`, and
   every other stage checks with a build alone (testing.md, commands.md).
5. **A session finishes in one pass,** as *Session workflow* below says.
6. **A comment says what the code does now, and is rewritten as a whole** (conventions.md).
7. **Docs describe the present only.** They carry no dates and no "previously" (doc-style.md).
8. **Text an end user reads follows the prose rules:** at most 25 words, one idea, and no dash joining
   two sentences (prose.md).
9. **Verify a claim before you make it, not when it is questioned** (grounding.md).
10. **Nothing is written outside this repository's folders.** Every scratch file goes under
    `.agent-tmp/`, and whoever writes one deletes it (commands.md).
11. **A change that adds, changes or removes a feature states its trade-offs** in performance, memory,
    usability and simplicity. Decide it yourself, however large, and ask the user only a call you
    cannot decide (conventions.md).
12. **A manual step done twice becomes a command.**
13. **Look up a fast-moving API in the installed version's reference,** never from memory.
14. **Name a goal by its slug, never by its number.**

## The context budget

- A loop session works under **200k** tokens of context, and an interactive session or a subagent under
  **250k**.
- Take no new slice past **120k**. At the ceiling, hand off.
- Work that will not fit is split across subagents before it starts.
  - The session keeps the decisions, the check of each result and the wrap.
  - Each subagent gets one narrow deliverable and a brief that names its ceiling.
  - Read-only subagents go out together, on smaller models. `Explore` finds code, and `reader` takes a
    read that needs judgment, such as a CI log. `.claude/agents/` defines both.
  - A writing subagent runs on the session's model.
  - Writing subagents whose files are disjoint run side by side in the same tree. None of them builds,
    and the session builds once they all end. Writing subagents that share a file run one after
    another, or each in its own worktree.
  - A subagent's report is a pointer to check, not evidence.
- **Wide work goes to subagents, whatever its size.** A search across many files, the list a sweep
  covers and a CI log go to read-only subagents. A batch of similar files, such as pages, examples or
  test cases, goes to writing subagents once the session has landed what they build on.

## Session workflow

Every session runs the same steps, in this order, and **stops**:

1. **Orient in one call.** In the loop the driver has done it already. Interactively, run
   `looper orient --full`.
2. **Do the work: as much of the group as fits.** The handoff names a group of related slices and the
   files they share. Keep taking slices while the next one touches files already loaded and you are under
   120k with the previous one committed. Before you take an item, write two to six checkpoints under it
   in the goal file, each a step you can tick alone. Tick each as you reach it, and the item once all are
   ticked. Tick on from checkpoints an earlier session left.
3. **Check with a build, and verify only in the gate stage.** A goal is verified once, in its last
   stage. In every other stage a build is the only check. No test and no verify runs there, except a
   test the session wrote, run once by name, and a run an item requires. A slice whose change breaks
   an existing test updates that test as it goes. The gate stage runs `looper verify` over the whole
   goal, fixes every failure in one pass, and runs it once more.
4. **Write the handoff once for the whole group.** Overwrite the live goal's `## Handoff` with where the
   work stands and the next group with its anchors. Name the tests you expect to go red. Tick what you
   finished. Add a playbook bullet if a trap cost you time.
5. **Commit, one commit per slice, once its build is green,** and in the gate stage once verify is
   green. Stage each slice's own files.
6. **In the loop, write the status line** the prompt names: `CONTINUE <what landed>`,
   `DONE <goal reached>` or `BLOCKED <what the loop cannot do itself>`. A design call never blocks:
   decide it, record it under `## Standing decisions`, and build on it. Only a call you cannot decide
   goes to the user, with `looper decision ask --built-on <n>`.

**After the last step, stop.** Writing prose cannot break a build.

## Goals

- **The goals are [docs/goals/](docs/goals/README.md),** in looper's goal format. The chain, which is the
  order the loop walks, is the front matter of `docs/goals/README.md`, and its first slug is the live
  goal. `looper render` writes the README's body, so never edit it by hand.
- **A reached goal is not harvested: its file is deleted.** Only a check that sets `floor: true` moves
  into the floor.
- **A goal is written so its sessions run fast:** stages that fit one session, a complete `### Next`,
  gates, settled choices, and named splits and lanes. The shape is
  [conventions.md](docs/agent/conventions.md) § *A goal*. A session follows its stage's `**Split:**` line.

## Commits

- **Conventional Commits:** `type(scope): what is now true`. The shape is
  [conventions.md](docs/agent/conventions.md) § *A commit message*.
- **No trailers, ever.** The `commit-msg` hook in `tools/git-hooks/` rejects them.
- **One commit per slice, once its build is green,** and in the gate stage once verify is green.
  Commit with `git commit -F <file>`.
- **Never `--no-verify`, and no force operations.**
<!-- looper:end agents -->
