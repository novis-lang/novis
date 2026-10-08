---
milestone: M10
position: last
---
# Loop goal — `nvs code` answers questions about a project from the editor's own index: where a name is used, what it is, what implements it

A coding agent works in a Novis project without an editor, so today it reads the project through
`grep`, which cannot answer the questions it needs most. `->addItem(` on two different classes is the
same text. A class reached through `autoload` has no `require` line to follow. An inferred type is
written nowhere. The language server already answers all of these from one reference index
(`rule:ide/five-features-are-one-reference-index`). This goal gives that index a command:
**`nvs code`**, five read-only verbs that take a fully-qualified name or a file and print what the
editor would show, as text or as JSON.

```text
$ nvs code refs 'Shop\Cart::addItem'
src/Cart.nvs:9:21          declared  public function addItem(int $price): void
src/Checkout.nvs:14:15     use       $cart->addItem($line->price);
tests/CartTest.nvs:6:11    use       $cart->addItem(1200);

$ nvs code show 'Shop\Cart::addItem'
Shop\Cart::addItem  method, declared at src/Cart.nvs:9
public function addItem(int $price): void
Adds one item's price to the cart's total.

$ nvs code impls 'Shop\Payment'          # every class that implements it, and where
$ nvs code outline src/Cart.nvs          # what the file declares, one line each, with its line
$ nvs code types src/Checkout.nvs:12-20  # the inferred type of every variable those lines bind
```

It is a command of its own and not a fifth verb of `nvs agent`: `nvs agent` answers about the
language with no project open, and every answer it gives is true of the installed binary. `nvs code`
needs a project, reads its files and answers about code that may not compile. People and CI scripts
can use it as well. The user chose the name, and that the command is separate.

## Why here

The user asked for it on 2026-10-08 and put it directly after goal `class-scoped-enums`. That goal
changes the symbols the index records: an enum becomes a class member, spelled `Shop\Order::Status`.
Landing it first means this goal's symbol spelling, its tests and its examples are written once,
against the names that stay. It goes in front of goals `pdf` and `spreadsheet`, which add surface
and touch none of these files. Its file set is `nvs-lsp`'s server, index and readers, one new module
in `nvs-cli`, `nvs agent`'s protocol text, and the tools chapters. It carries `position: last`
because every goal on the chain is pinned there.

## Stage 0 — the catch-up

The sentences on disk this goal makes untrue, each rewritten whole by the session that lands what
makes it untrue, and not before:

- **By Stage 2:** `docs/rules/ide/five-features-are-one-reference-index.md:21-27` (the readers of the
  index gain `nvs code`, and the one construction site stays `server.rs`'s);
  `crates/nvs-lsp/tests/index.rs:153-158` (the comment counts the indexes `server.rs` builds).
- **By Stage 3:** `docs/rules/ide/one-server-two-thin-clients.md:1-3` and `:18-20` (`nvs code` is a
  third client of `nvs-lsp` that decides nothing about the language);
  `crates/nvs-cli/Cargo.toml:85-90` (the binary does more with `nvs-lsp` than start its server).
- **By Stage 4:** `docs/rules/tooling/an-agent-asks-the-binary.md` § its last paragraph (the
  protocol names `nvs code` for questions about the project); `crates/nvs-cli/src/agent.rs:1-35`
  and `:1177-1205` (the `PROTOCOL` text and the module doc that describes it);
  `docs/reference/tools/50-agents.md:4-5` and `:145` (the summary, the keywords and the stanza
  sketch).

The search that closes the stage, run after Stage 5:
`git grep -n -E "two (thin )?clients|six readers|four commands|nothing more" -- docs/rules docs/reference crates/nvs-cli crates/nvs-lsp`,
read line by line. Every hit is true as it stands, or rewritten. `docs/ground-rules.md` and the
`docs/rules/*.md` chapters are generated and are regenerated, never edited.

## Stage 1 — the floor

Whatever the chain carries when this goal is reached. Every check goal `class-scoped-enums` turned
green stays green.

## Stage 2 — the keystone: one workspace, and queries by name

**Does:** Lets a caller outside the server build the workspace index once, for one question, and ask
it by fully-qualified name. Every editor feature that answers the same question reads the same
function, so the editor and `nvs code` never disagree.

One file set: `crates/nvs-lsp/src/server.rs`, `crates/nvs-lsp/src/index.rs`, a new
`crates/nvs-lsp/src/query.rs`, `crates/nvs-lsp/src/lib.rs`, `crates/nvs-lsp/src/hints.rs`,
`crates/nvs-lsp/src/symbols.rs`, `crates/nvs-lsp/tests/`.

- **The one-shot workspace** is built in `server.rs`, beside the `.lspt` case index
  (`lenses_of_case`, `crates/nvs-lsp/src/server.rs:1292`, and `case_settings`, `:1276`): a `pub fn`
  that takes a root, makes a `Documents`, runs `Documents::survey` (`crates/nvs-lsp/src/document.rs:272`)
  so an autoloaded name resolves, and builds the index at `CheckScope::Workspace`. It returns one value
  that owns both. `SymbolIndex::build` stays called from `server.rs` alone, so the guard at
  `crates/nvs-lsp/tests/index.rs:159` holds unchanged.
- **The readers**, in `query.rs`, each over that value and a symbol string spelled as
  `crates/nvs-lsp/src/index.rs:10-24` spells it:
  - *references*: what `uses_of` (`server.rs:907`) answers, with its kind (declared, import, use).
    `uses_of` is rewritten to call it, so `textDocument/references` and `nvs code refs` are one function.
  - *implementations*: for a type, every subtype, transitively, from `subtypes` (`index.rs:519`); for
    a method, `overriders` and `overridden` (`index.rs:577`, `:537`). `implementation` (`server.rs:1532`)
    reads it too.
  - *declaration*: kind, site and the `///` run the declaration carries, the run hover reads
    (`crates/nvs-lsp/src/card.rs:81`).
  - *outline*: `declarations_in(path)` (`index.rs:644`), nested by owner, in source order.
  - *types*: the `: T` labels `hints::for_document` (`crates/nvs-lsp/src/hints.rs:88`) gives for the
    bindings in a line range, from `analyse_file` (`document.rs:661`) of that file.
  - *nearest*: the symbols closest to a name the index does not have, for the error.
- **Pinned by** the Stage 2 check.

## Stage 3 — the command

**Does:** Adds `nvs code` with its five verbs, their text and JSON output, its rule, and its help.

One file set: a new `crates/nvs-cli/src/code.rs`, `crates/nvs-cli/src/main.rs` (the `Command` enum
near `Agent`, `:829`, and its dispatch), `crates/nvs-cli/src/meta.rs` (the signature, below),
`crates/nvs-cli/tests/code.rs` (new), `docs/rules/tooling/`, `docs/rules/ide/`, `crates/nvs-cli/Cargo.toml`.

- **The verbs:** `show <symbol>`, `refs <symbol>`, `impls <symbol>`, `outline <file>` and
  `types <file>[:<line>[-<line>]]`, each with `--json`. The output is § *Standing decisions*'.
- **The signature** a line or a card prints is the one `nvs meta` writes for a program member
  (`crates/nvs-cli/src/meta.rs:937`, `param_signature` at `:948`). If that builder cannot be reached
  without a program that checks clean, it moves to one function that both commands call. A signature
  is never spelled a second way.
- **The rule:** create `tooling/nvs-code-answers-about-the-project` from § *Standing decisions*, with
  the tradeoffs and a **Declined:** list (a rename verb, a disk cache, a daemon, a fifth `nvs agent`
  verb). Rewrite `ide/one-server-two-thin-clients` (Stage 0).
- **Pinned by** the Stage 3 checks.

## Stage 4 — agents learn of it

**Does:** Makes every agent that `nvs agent init` set up learn that `nvs code` exists.

One file set: `crates/nvs-cli/src/agent.rs`, `crates/nvs-cli/tests/agent.rs`,
`docs/reference/tools/50-agents.md`, `docs/rules/tooling/`.

- **The protocol** (`PROTOCOL`, `crates/nvs-cli/src/agent.rs:1183`) gains the questions about the
  project, and names `nvs code refs`, `show` and `outline` as the way to ask them instead of `grep`.
  It names commands, never a language fact (`rule:tooling/an-adapter-carries-protocol-and-never-language`).
  The text changes, so every unedited pointer an earlier `init` wrote is reported outdated and is
  rewritten by the next `init`. That is the fingerprint mechanism the module doc describes, and it
  needs no legacy fingerprint.
- **The primer** names `nvs code` only if a `<!-- primer -->` section of a chapter says it
  (`rule:tooling/a-primer-claim-is-executed`). No sentence about it is written into `agent.rs`.
- **`nvs agent index`** lists `command: nvs code` and each verb from the `clap` definition
  (`crates/nvs-cli/src/agent/names.rs:13-15`). A test pins that they are there.
- **The rule:** rewrite `tooling/an-agent-asks-the-binary`'s last paragraph (Stage 0). The four verbs
  stay four.
- **Pinned by** the Stage 4 check.

## Stage 5 — the feature proofs

**Does:** Writes the reference section and the feature proofs for `nvs code`, and closes Stage 0.

One file set: `docs/reference/tools/10-cli.md` (a new `# nvs code` section after `# nvs ast`, `:574`),
`docs/examples/tools/cli/nvs-code/`, `tests/hostile/tools/cli/nvs-code/`.

- **The feature** is that section, which makes it `tools:cli/nvs-code` on the roster. A tool owes
  `about.md`, tests, one example and one attack, and no bench (`bun nv proofs --id 'tools:agents/nvs-agent'`
  is a complete one). `bun nv proofs --id 'tools:cli/nvs-code'` prints what is still owed.
- **The example:** a small project of two classes and an interface, whose comment shows the
  `nvs code refs` and `nvs code impls` lines that find a method's callers and an interface's classes.
- **The attack:** a project file whose `///` comment carries terminal escape bytes and a fake
  `path:line` line, and which requires itself. `nvs code show` and `nvs code outline` print the escape
  bytes as `\u{1b}`, print the fake line as part of the comment, and stop.
- **Every user-facing text** — the help, `about.md`, the example and attack comments, each error
  message — follows AGENTS.md § *Text an end user reads*.
- **Pinned by** the Stage 5 checks.

## Standing decisions

- **The user's calls, as instructions.** A separate command, named `nvs code`, with the five verbs
  above. Read-only. Placed after goal `class-scoped-enums`.
- **The goal writer's calls, not confirmed by the user, also standing.**
  - **The symbol** is the index's own spelling: `Shop\Cart`, `Shop\Cart::addItem`, `Shop\Cart::$total`,
    `Shop\Cart::MAX`, `Shop\Order::Status` after goal `class-scoped-enums`. There is no second spelling.
    A name typed without its backslashes (the shell removes them when unquoted) matches nothing. The
    error then lists the nearest names, and the backslashed one is among them.
  - **A `Core` name** is a valid argument to `refs` and `impls`, which list the project's uses of it.
    `show` of a `Core` name prints the card `nvs agent show` prints, from the same function.
  - **A position** is `path:line:column`. The path is relative to the working directory, and the line
    and column start at 1. The column counts characters, not bytes. JSON also carries the byte `start`
    and `end`.
  - **A `refs` line** is the position, the kind (`declared`, `import`, `use`) and the source line with
    its indentation removed. The declaration comes first, then the rest in path and line order.
  - **The JSON** follows `rule:ide/check-json-is-the-diagnostic-record-as-a-document`, as
    `nvs agent --json` does: `schemaVersion: 1` at the root, every key present, and a failure still
    prints its document (`error`, `nearest`) on standard output while the exit status is 1.
  - **The project** is the directory of the nearest `nvs.toml` at or above the working directory, and
    the working directory when there is none. Every `.nvs` file under it is read, as the editor reads
    a workspace.
  - **Code with errors still answers.** The index is built from what the analysis recovered.
    Standard error gets one line with the number of files that have errors, because a use inside one of
    them may be missing. The exit status is 0.
  - **Exit status:** 0 with an answer, and 0 with an empty answer (`refs` of a name nothing uses).
    1 for a name or file the project does not have, with the nearest ones.
  - **Text from the project** is printed with every control character but tab escaped as `\u{..}`, in
    the text and the JSON output. A file's own bytes must not reach a terminal or an agent's context as
    anything but text (Security and request isolation).
- **Out of this goal, each a Declined line in the new rule:** a `rename` verb or any verb that writes
  a file, until the language server has rename (`crates/nvs-lsp/src/capabilities.rs:156-158`); a disk
  cache of the index; a background daemon; a fifth `nvs agent` verb; call hierarchy, which
  `rule:ide/five-features-are-one-reference-index` declines for the editor too.
- **The cost of a call** is one cold workspace analysis, one `analyse_file` per `.nvs` file under the
  root (`crates/nvs-lsp/src/index.rs:373`, `:701`). No cache and no daemon are added in this goal: a
  cache is a second copy of the project that can be stale. If a session measures a call too slow for
  an agent loop on a large fixture, it writes a gap under `data/gaps/` naming `nvs-lsp` and this
  command, and does not build a cache.
- **No decision record.** The rule's fragment holds the reasons and the Declined list.
- **The tradeoffs**, stated here and in the rule because AGENTS.md asks. Performance: none on the
  request path, since this is tooling. A call costs one cold analysis of the project. Memory: none per
  request; one index for the length of one call. Usability: an agent finds a method's callers, an
  interface's classes and a variable's type in one call, without `grep`'s wrong answers. Simplicity:
  one new command over functions the editor already has, and `nvs agent` keeps its four verbs.
- **No new limit**, and no setting.
- **Neutral names only** in every test, example and rule: `Shop`, `Blog`, `example.com`.
