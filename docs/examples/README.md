# The example tree — three small programs per feature

Every feature Novis ships owes three real-world examples
(`rule:testing/four-proofs`), and this is where they live. They are
**the website's copy**, kept in the repository so that the same sweep that tests a feature writes
them: `npm run sync:examples` in [`website/`](../../website/README.md) copies this tree into the
site, and nothing edits them there.

`python tools/dossier.py --run examples` is what runs them. This file owns what an example **is**;
that tool owns how it is checked, the way [`benches/userland/README.md`](../../benches/userland/README.md)
and `tools/bench.py` already split the same way.

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
program prints. `python tools/dossier.py --id 'Core\Str::length'` prints the directory for any
feature rather than making you derive it.

## The description — `about.md`

Each feature's directory also holds `about.md`: **the first thing a person sees on the website when
they look the feature up**, above its examples. It is written before the examples, and the examples
are then written to build on it.

A beginner and an expert should both read it once and come away with the same picture.

- **One lead sentence** saying what the feature does, in plain words. It has to work alone, as a
  search result.
- **One or two short paragraphs** on what somebody needs to know to use it correctly. 40 to 160
  words for the whole file; most features want about 80.
- **`**In plain words:**`, only where it is needed.** When the honest explanation is technical —
  taint, a bounded channel, a limit's ceiling — add one or two sentences with an everyday picture.
  A feature whose lead sentence is already plain gets none.
- **`**Good to know:**`, optional.** One or two real surprises. Never a list of every edge case;
  those are pinned in `tests/conformance/`.
- **`**The examples below**`, only where the prose is hard to follow without code.** One closing
  sentence naming what the examples show, in order. It is a promise: the examples in the same
  directory show exactly that. A feature the prose fully explains gets no such sentence.
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
many values the channel holds when you create it. When it is full, the sender waits until there is
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
  something would otherwise surprise. § *How a comment is written* below is the whole rule.
- **Every example prints.** The `.out` file is the proof it still works, so an example that computes
  something and shows nothing cannot be checked.
- **Nothing is asserted.** An example is not a test and never carries `Core\Test`. What it *shows*
  is pinned by its `.out`; what the feature *guarantees* is pinned in `tests/conformance/`, and the
  two are not the same file for a reason — a reader should be able to copy the whole example.

## How a comment is written

**This section is the rule for every `.nvs` file a feature's proofs are made of** — the examples
here, the attacks under `tests/hostile/` and the programs under `benches/members/`. Those two trees'
READMEs point here instead of restating it.

A comment in one of these files is read by the same person `about.md` is written for: somebody who
looked the feature up and has never seen this repository. So it is written the way the description
is. **A beginner and an expert should both read it once and come away with the same picture.**

- **Short sentences, one idea each.** If a sentence has to be read twice, it is two sentences. A
  sentence that needs a dash, a semicolon and a "rather than" to get to its end is three.
- **Everyday words, and the literal thing.** Say "this setting can only be changed by whoever runs
  the server", not "this one is not a program's to change". No figures of speech: not "the same
  figure buys more here", not "earns a longer clock", not "works the boundary from both sides".
- **Say what the next lines do and why somebody would care**, in the reader's words — "your
  program", "the server", "the person who runs it". The words the implementation uses for itself
  stay out: shard, tier, slot, row, longest match, single filler, refcount, lowering, the registry.
- **A name the reader will type is welcome** — the setting, the class, the member, the error they
  will catch — in backticks. A term they need and may not know gets a few plain words the first
  time: "a grapheme (what a person counts as one character)".
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

A directive line — `// bench:`, `// hostile:`, `// covers:`, `// dossier:`, `// requires:` — is
read by a tool, is not prose, and is left exactly as its own README spells it.

The same comment, first the way it goes wrong and then the way it is written:

```nvs
// Unlike almost everything else in a configuration file, this one is a program's
// to change while it runs — a job that knows its own work is quick can say so,
// and the answer is `yes` rather than a silent refusal.
```

```nvs
// Your program may change this setting while it runs. Most settings do not allow that.
```

And an attack:

```nvs
// Attack: the ceiling is the only thing standing between a request and as much
// of this host as it cares to ask for, and it is one longest-match row away
// from the block the request *is* allowed to write.
```

```nvs
// Attack: a request tries to raise its own memory limit. Only the person who runs
// the server may set that limit, so every attempt below should be refused.
```

If a comment would only make sense to somebody who works on Novis, it is the wrong comment.

`python tools/dossier.py --comments <file or directory> ...` judges what a script can: the line
bounds above, a sentence longer than a plain one gets, a dash joining two sentences, and the
implementation's words. It reads and never runs a program. Passing it is the floor, not the rule —
a short sentence can still be one nobody follows — and the answer to a line it names is to write
two sentences, never to trim a word until the count fits. `[all] comments = true` in
`tools/data/dossier-policy.toml` makes it part of `--gate`; goal `plain-comments` writes that line
once the programs that landed before this section are inside the bounds.

## Creating the `.out`

    python tools/dossier.py --bless docs/examples/core/Str/length/01-count-characters.nvs

That runs the program and writes what it printed, then prints it back so you read it. **Blessing is
how an expected output is created, never how a red example is made green** — the same rule the loop
goals state for fixtures. An example that has stopped printing what it used to has either found a
regression or needs rewriting, and re-blessing decides which without looking.

Where the disagreement is the binary's fault and the fix is larger than the slice, record it rather
than re-blessing: a `# Known gaps` entry in the owning crate's module doc, and a marker on the
example naming it — `// dossier: known-gap crates/…/foo.rs -- what is wrong`. The sweep counts it as
`known-gap` instead of a failure, and a marked example that passes fails the sweep, so the marker
comes off with the fix.

## `// requires: unimplemented`

An example for a feature that does not run yet carries that line and is skipped rather than failed.
It is the website's own marker, honoured unchanged, and it exists so an example can be written
against a designed surface ahead of its implementation. Remove the line when the member lands.
