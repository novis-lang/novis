# The example tree — three small programs per feature

Every feature Novis ships owes three real-world examples
(`rule:testing/feature-proofs`), and this is where they live. They are
**the website's copy**, kept in the repository so that the same sweep that tests a feature writes
them: `npm run sync:examples` in [`website/`](../../website/README.md) copies this tree into the
site, and nothing edits them there.

`bun nv proofs --run` is what runs them. This file owns what an example **is**;
that tool owns how it is checked, the way [`benches/userland/README.md`](../../benches/userland/README.md)
and `tools/nv/cmd/bench.ts` already split the same way.

## Where an example goes

One directory per feature, and the path is the same one the attack tree and the bench tree use — so
nothing registers an example anywhere:

| Feature | Directory |
|---|---|
| `Core\Str::length` | `core/Str/length/` |
| `Core\Db\Row::bool` | `core/Db-Row/bool/` |
| a language feature | `lang/<chapter>/<heading-slug>/` |
| a `Core` exception, enum or interface | `types/<Name>/` |
| a CLI or config topic | `tools/<chapter>/<heading-slug>/` |
| an `nvs.toml` directive | `config/<key-with-dots-as-dashes>/` |

Inside it, `01-slug.nvs` … `03-slug.nvs`, each with a sibling `01-slug.out` holding exactly what the
program prints. `bun nv proofs --id 'Core\Str::length'` prints the directory for any
feature rather than making you derive it.

## The description — `about.md`

Each feature's directory also holds `about.md`: **the first thing a person sees on the website when
they look the feature up**, above its examples. It is written before the examples, and the examples
are then written to build on it.

A beginner and an expert should both read it once and come away with the same picture.

- **One lead sentence** saying what the feature does, in plain words. It has to work alone, as a
  search result.
- **One or two short paragraphs** on what somebody needs to know to use it correctly. 40 to 160
  words for the whole file; most features want about 80. The check accepts up to 200, so a
  description a few words over 160 that reads well is left as it is.
- **`**In plain words:**`, only where it is needed.** When the honest explanation is technical —
  taint, a bounded channel, a limit's ceiling — add one or two sentences with an everyday picture.
  A feature whose lead sentence is already plain gets none.
- **`**Good to know:**`, optional.** One or two real surprises. Never a list of every edge case;
  those are pinned in `tests/conformance/`.
- **`**The examples below**`, only where the prose is hard to follow without code.** One closing
  sentence naming what the examples show, in order. It is a promise: the examples in the same
  directory show exactly that. A feature the prose fully explains gets no such sentence.
- **Plain English, the same as a comment.** [`AGENTS.md`](../../AGENTS.md) § *Text an end user
  reads* is the rule for both, and its word table holds here too.
- **No code, no internals, no history.** No ADR numbers, no crate names, no "the registry". One
  "replaces PHP's `sort`, `usort`, …" hint is welcome where it helps somebody arriving from PHP.

The file is plain Markdown with no front matter and no heading — the page supplies the title.

A feature that needs neither optional part:

```markdown
Counts the characters in a string, the way a person would count them.

An emoji, an accented letter or a flag each count as one, even though the computer stores them as
several bytes. Every `Core\Str` member counts the same way, so a position you get from one member is
safe to hand to another.

**Good to know:** this is not the size of the string in memory. If you need bytes, for a file size
or a network limit, use `Core\Bytes`.
```

And one that needs both:

```markdown
A queue that lets tasks running at the same time hand values to each other safely.

One task puts values in with `send`, another takes them out with a plain `foreach`. You choose how
many values fit in the channel when you create it. When it is full, the sender waits until there is
room, so a fast producer can never flood a slow consumer or fill up memory. Call `close` when you
are done sending; the receiving loop finishes what is queued and then ends.

**In plain words:** a conveyor belt with a fixed number of slots between two workers. If the belt is
full, the first worker pauses. If it is empty, the second one waits.

**Good to know:** a sender that never calls `close` leaves its receiver waiting forever.

**The examples below** build this up step by step: a producer and a consumer first, then what
"full" looks like, then closing cleanly.
```

## What an example is

**A reader who has never seen this repository is the audience.** Not a test, not a specification —
a program somebody skims for fifteen seconds and then writes their own version of.

- **Small and self-contained.** One file, no framework, no setup, runs with `nvs run <file>`. If it
  needs a database or a socket it is the wrong example for the website; find the version of the same
  idea that needs neither.
- **A feature about another file carries that file in a subdirectory.** `require` and `autoload`
  cannot be shown in one file, so the companion goes under the example's own directory — `parts/`,
  `app/`, `packages/`. It has to be a *subdirectory*: the sweep counts `*.nvs` at the directory's
  top level only, so a companion beside the examples would be read as a fourth one, while the
  website mirror walks the whole tree and ships a companion below it. Write each companion so that
  running it on its own does nothing and succeeds.
- **Real work, not `foo`/`bar`.** A cart total, a log line, a slug, a config key, a retry — the
  thing a person is actually holding when they reach for this feature. Three examples means three
  *different* uses, and the third is the one that earns its place: make it the one somebody does at
  work.
- **Comments are plain.** One sentence at the top saying what the example shows, and a line where
  something would otherwise surprise. [`AGENTS.md`](../../AGENTS.md) § *Text an end user reads* is
  the whole rule.
- **Every example prints.** The `.out` file is the proof it still works, so an example that computes
  something and shows nothing cannot be checked.
- **Nothing is asserted.** An example is not a test and never carries `Core\Test`. What it *shows*
  is pinned by its `.out`; what the feature *guarantees* is pinned in `tests/conformance/`, and the
  two are not the same file for a reason — a reader should be able to copy the whole example. The
  one exception is a feature that **is** `Core\Test`: an assertion member and
  `lang:errors/assertion-failures` can only be shown by a program that calls one, and such a program
  still shows what it prints rather than asserting anything about itself.

## How a comment is written

[`AGENTS.md`](../../AGENTS.md) § *Text an end user reads* is the whole rule, for every `.nvs` file a
feature's proofs are made of — the examples here, the attacks under `tests/hostile/` and the programs
under `benches/members/` — and for `about.md`. It lives there because every agent has it in context
before it writes anything, and nothing checks the words afterwards.

`bun nv proofs --comments <file or directory> ...` counts the three bounds that rule states
as numbers: the lines in a comment block, the words in a sentence, and a dash joining two sentences.
It reads and never runs a program. `"comments": true` in the `all` override of `data/proofs/policy.json` makes
it part of `--gate`; goal `plain-comments` writes that line once the programs that landed before the
rule are inside the bounds.

## Creating the `.out`

    bun nv proofs --bless docs/examples/core/Str/length/01-count-characters.nvs

That runs the program and writes what it printed, then prints it back so you read it. **Blessing is
how an expected output is created, never how a red example is made green** — the same rule the loop
goals state for fixtures. An example that has stopped printing what it used to has either found a
regression or needs rewriting, and re-blessing decides which without looking.

Where the disagreement is the binary's fault and the fix is larger than the slice, record it rather
than re-blessing: a gap record under `data/gaps/`, and a marker on the example naming it —
`// proof: gap nvs-stdlib/a-slug-naming-what-is-wrong`. The sweep counts it as
`known-gap` instead of a failure, and a marked example that passes fails the sweep, so the marker
comes off with the fix.

## A file for standard input — `<name>.in`

An example runs with nothing on its standard input, so a program reading it sees the empty string
at once. An example that shows reading input puts that input beside it, under its own name with
`.in` in place of `.nvs`: `01-count-the-lines-of-the-input.in`. `--bless` and the sweep then send
the file to the program's standard input, byte for byte, and the website shows it beside the
example. `tests/hostile/` and `benches/members/` read a `.in` file the same way.

## `// proof: exit 1`

An example runs to its last line and exits `0`, and that is the default because an example is a
program a reader copies. A feature whose whole subject is the **ending** — an uncaught throw, a
limit stopping the program, `exit($n)` — has no such program to write, so it declares the status it
ends with:

    // proof: exit 1

and the runner then requires exactly that status, the way `tests/hostile/`'s `ends-early` marker
already does for an attack. The status is exact rather than merely non-zero, so an example that
lands on a different one is still a failure. Nothing else changes: the `.out` beside it is still
frozen standard output, byte for byte, and what the program wrote to standard error is not part of
it. Without this line, the feature could carry no example at all — which is a `[skip]` entry and a
website page with nothing on it, for a feature a reader is especially likely to look up.

## `// requires: unimplemented`

An example for a feature that does not run yet carries that line and is skipped rather than failed.
It is the website's own marker, honoured unchanged, and it exists so an example can be written
against a designed surface ahead of its implementation. Remove the line when the member lands.
