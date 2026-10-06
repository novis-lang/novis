# The reference chapters — what `docs/novis.md` is generated from

[`docs/novis.md`](../novis.md) is the one-file reference to everything Novis has, written for a
reader who has never seen this repository: a search engine, a language model, a person. **It is
generated, never edited** — `bun nv reference` builds it from three inputs and runs every
example in it against the binary; `bun nv verify` does the same after every build, so it
follows the code without anyone remembering to.

| Input | Owner | What it contributes |
|---|---|---|
| `nvs meta --json` (the built binary) | the `Core` registry in `crates/nvs-stdlib` | every class, member signature, reference card, constant and enum; the exception tree; the global interfaces; the compiler-recognized attributes; the `nvs.toml` directives — **all of Part B**, and the generated tables inside chapters |
| `lang/*.md` | hand-written, this directory | Part A — one chapter per language topic, in filename order |
| `tools/*.md` | hand-written, this directory | Part C — one chapter per tool |
| `core/<Class>.md` | hand-written, this directory | the introduction and one worked example at the top of a class's Part B section. Optional: a class without one gets its member cards alone |

## Who reads this, and what that means for the prose

A reader of `novis.md` has **only that file**. They cannot follow a link to an ADR, do not know what
a milestone is, and do not care why a decision was made — they need to know what the language
*does*, completely and once. So:

- **Complete, not introductory.** A chapter states every rule of its topic, including the spellings
  Novis refuses where a PHP reader would expect them. If a rule exists in the shipped compiler and
  is not in some chapter, the reference is wrong.
- **Once.** A fact has one section. A `Core` member is documented by its card in Part B, generated
  from the registry — a chapter *uses* members in examples and names them, but never describes
  their parameters; that is the card's job. Two chapters do not explain the same construct.
- **Only what ships.** Nothing planned, nothing from a milestone that has not landed, no "will".
  The binary is the test: if `nvs run` refuses it, it is not in the reference. A chapter that must
  mention a door that is closed says it is closed, not that it will open.
- **Short sentences, concrete spellings, no rationale.** Write "`and`/`or` do not parse; write
  `&&`/`||`", not why. The ADR that owns a paragraph may be cited for *this repository's* readers
  in an HTML comment — `<!-- src: `rule:expressions/no-keyword-logical-operators` -->` — which the generator strips.
- **No links out of the file.** Cross-reference another section by its heading text in backticks
  or by its anchor, `[types](#lang-types)`; anchors are `lang-<id>`, `tools-<id>`,
  `core-<class>` (`core-core-str`), `core-<class>-<member>` (`core-core-str-length`),
  `enum-<name>` (`enum-core-order`).

## A chapter file

```
---
id: types
title: Types and declarations
summary: every type, how a binding declares one, and the `as` conversion
keywords: int, uint, float, decimal, string, bytes, bool, mixed, array<T>, nullable, ?T, union, var, as
---

Body in markdown. Top-level headings inside the body are `#` and are demoted by the generator
(`#` → `###` under the chapter's own `##`), so start at `#` and nest from there.
```

`id` is the anchor and must be unique across `lang/` and `tools/`; `title` is the heading;
`summary` is the index's one line; `keywords` is a comma-separated list a searcher might type,
including PHP spellings the topic replaces. Files are concatenated in filename order — number them
`10-`, `20-`, … so a chapter can be inserted between two.

A chapter may place one of the generated tables where its prose wants it, on a line of its own:

```
<!-- generated: exceptions -->     the exception tree: class, parent, own properties
<!-- generated: interfaces -->     the global interfaces and their type parameters
<!-- generated: attributes -->     the compiler-recognized `#[...]` names
<!-- generated: directives -->     every `nvs.toml` directive, its class and when it applies
```

A table no chapter places is appended at the end of the file, so nothing the binary declares is
ever lost — but place them, because a table inside its chapter is what a reader finds.

## Marking a section for the primer

`nvs agent primer` is the short document a coding agent reads before it writes anything, and it is
generated from these chapters rather than hand-written
(`rule:tooling/a-primer-claim-is-executed`). A chapter opts a section in with a comment on the line
directly above that section's heading:

```
<!-- primer -->
# `[capabilities]`
```

The section it heads is lifted whole — its prose, its tables and its examples, down to the next
heading of the same or a shallower level — so a section that stops being true stops being rendered
rather than becoming a lie. The marker is stripped from `docs/novis.md`, where it would mean
nothing.

What the primer carries is fixed: the lookup protocol, one complete worked program with every shape
in it annotated, the capability model and the smallest `nvs.toml` that grants a file read, and the
chapter map from this front matter. A section that is none of those puts
prose in front of a reader spending a token budget on it, and a part of the primer that no section
expresses is a chapter missing it — the edit belongs in the chapter.

## A class introduction file

`core/<Class>.md`, named by the class after `Core\` with `\` written `-`: `Str.md`,
`Time-DateTime.md`, `Task-Channel.md`. Same front matter, with only `summary` and `keywords` read
(`keywords` here are *extra* — every member name is added automatically; put the PHP names the
class replaces, and the concepts). The body is a two-to-four sentence introduction — what the class
is for, the one rule a user must know (grapheme counting, copy-on-write, `?T` for absence) — and
**one** worked example that touches the members a user reaches for first. Not one example per
member: the member cards below it carry every signature, and an example that repeats the cards is
the duplication this file exists to avoid.

## Examples: the fence grammar

Every fenced block in a chapter or introduction is read by the generator. A block marked `nvs` is
**run** (`nvs run`) in a fresh directory, and the `output` block directly after it is what it must
print, byte for byte after trailing whitespace is trimmed. An example that fails is a failed
`nv verify` — so every example is proof, and there is no way to ship a wrong one.

    ```nvs
    <?nvs
    echo "Hello", "\n";
    ```
    ```output
    Hello
    ```

| Fence | Meaning |
|---|---|
| ```` ```nvs ```` | the entry program, run with `nvs run main.nvs`; must exit 0 |
| ```` ```nvs exit=3 ```` | the same, expected to exit with that status (an uncaught throw, an `exit`) |
| ```` ```nvs error ```` | must **fail** `nvs check`; an `output` block after it is a substring the diagnostic must contain |
| ```` ```nvs test ```` | run with `nvs test main.nvs` (a `#[Test]` program); each line of the `output` block must appear somewhere in what it printed |
| ```` ```nvs skip ```` | shown, never run: a fragment, a signature, a spelling that needs context |
| ```` ```nvs file=lib/Greeter.nvs ```` | a sibling file written beside the next entry program, at that relative path; any number may precede one entry |
| ```` ```toml file=nvs.toml ```` | the same for a configuration file, or any other text file (`csv`, `txt`, `json`) — the entry runs with the example's directory as its working directory, so `nvs.toml` there is the one it reads |
| ```` ```output ```` | the expectation for the program before it; optional — a program with none must merely exit as declared |

Rules that keep examples honest and cheap:

- **Every runnable example is a complete program** that starts with `<?nvs` and prints something,
  so a reader can copy it out and run it. Multi-file examples name the other files with `file=` and
  `require` or `autoload` them from the entry, which is `main.nvs`.
- **No output that varies**: no clock, no random value, no path of this machine. Print a derived
  fact instead (`Core\Uuid::v4()` → print its length).
- **No capability the example's own `nvs.toml` does not grant.** A file read needs a
    `toml file=nvs.toml` fence granting `fs.read`; without one the example runs with no
  configuration at all.
- Keep each under ~40 lines; two small examples beat one that does everything.
- `bun nv reference --examples-only --only <chapter-file-substring>` runs one chapter's
  examples while writing it; `--keep` leaves the directories under `.agent-tmp/reference-examples/`
  to look at.
