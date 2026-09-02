# The example tree — three small programs per feature

Every feature Novis ships owes three real-world examples
([ADR 0134](../adr/0134-every-shipped-feature-owes-four-proofs.md)), and this is where they live. They are
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

## What an example is

**A reader who has never seen this repository is the audience.** Not a test, not a specification —
a program somebody skims for fifteen seconds and then writes their own version of.

- **Small and self-contained.** One file, no framework, no setup, runs with `nvs run <file>`. If it
  needs a database or a socket it is the wrong example for the website; find the version of the same
  idea that needs neither.
- **Real work, not `foo`/`bar`.** A cart total, a log line, a slug, a config key, a retry — the
  thing a person is actually holding when they reach for this feature. Three examples means three
  *different* uses, and the third is the one that earns its place: make it the one somebody does at
  work.
- **Comments are plain.** One sentence at the top saying what the example shows, and a line where
  something would otherwise surprise. No ADR numbers, no "the registry", no taint vocabulary, no
  milestone. If a comment would only make sense to somebody who works on Novis, it is the wrong
  comment.
- **Every example prints.** The `.out` file is the proof it still works, so an example that computes
  something and shows nothing cannot be checked.
- **Nothing is asserted.** An example is not a test and never carries `Core\Test`. What it *shows*
  is pinned by its `.out`; what the feature *guarantees* is pinned in `tests/conformance/`, and the
  two are not the same file for a reason — a reader should be able to copy the whole example.

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
