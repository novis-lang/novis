# Handoff

## State

**Every `Core` name now hovers to its card and jumps to a generated stub**: three commits, built on
branch `core-stubs` in the worktree `.agent-tmp/worktrees/core-stubs`, rebased onto `main` past the
server chapter and fast-forwarded on 2026-09-22 by the user's word, verified green by one `python
tools/verify.py` and the extension's headless tier. ADR 0214 is the record (0213 landed on `main`
from another session while this one ran) and `rule:ide/the-stub-tree-is-where-core-is-declared` the
rule.

What landed, by commit: (1) `nvs_lsp::card` renders every `Core` target's card and hover routes
`Target::Type`/`Target::Constant` through it, a namespace segment hovers to what it contains, the
checker records what an attribute's name resolved to and infers every payload value, the server walks
attribute payloads itself (`definition::payload_path`), the thirteen compiler attributes carry a card
(`nvs_types::derive::ATTRIBUTE_DOCS`), semantic tokens colour the name before `::` and attribute
names; (2) `nvs_lsp::stubs` generates the tree with a line table, `definition::at`/`type_at` fall back
to it, `nvs.stubs.dir` reaches the server and the extension fills it from its own storage, a stub
document is published with no diagnostics, `nvs stubs --out <dir>`; (3) `build.rs` compiles the 56
reference intros in as `CoreClass::intro`, shown by the class hover and the stub header.

**No live editor run was made.** What to click in VS Code, once the branch is merged and the extension
rebuilt, to confirm each behaviour — every one is frozen as a `.lspt` case, so what is unconfirmed is
the client, not the answer:

- **Class:** in `var $n = Core\Str::length("a");` hover `Str` — a `class Core\Str` block, then the
  intro page's text. Ctrl+click `length` — `stubs/Core/Str.nvs` opens at `public static function
  length`, read-only on disk, with no squiggles.
- **Enum case:** in `Core\Http\Method::Get` hover `Get` — `Core\Http\Method::Get` and "Reads a
  resource; safe…"; hover `Method` — the enum's card with all eight cases; ctrl+click `Get` — the
  case's line in `Core/Http/Method.nvs`.
- **Attribute:** above a method write `#[Core\Route(path: "/", method: Core\Http\Method::Get)]`;
  hover `Route` — the attribute as it is written and what it does; ctrl+click `Route` —
  `Core/Route.nvs` at `type Route = {…}`; hover `Get` inside the payload — the case line.
- **Namespace:** hover `Http` in `Core\Http\Method` — `namespace Core\Http` and the thirteen names
  under it. Ctrl+click on it still opens `Method`.

**Where the tree is written on this machine.** By the extension: under its global storage,
`%APPDATA%\Code\User\globalStorage\novis-lang.nvs\stubs\0.0.1\` (the extension's own version names
the directory; the server's stamp inside it decides whether the files are rewritten). By a bare
`nvs lsp`: `%LOCALAPPDATA%\novis\stubs\0.0.1\`. Neither exists yet. `nvs stubs --out` wrote a copy
to `D:\mwl\.agent-tmp\core-stubs\tree` for inspection; the unit test and the `.lspt` runner write
under `%TEMP%` and remove what they wrote.

**Registry docs that read badly in a stub**, for the user to decide whether a rewrite goal joins the
chain: 120 of the 125 stub files contain "rather than", and most cards explain a design choice
before they say what the member does. The clearest: `Core\Http\Method`'s line ("eight of them, safe
ones first so that the four the CSRF check covers are the contiguous tail from `Post` on; `CONNECT`
is deliberately absent"), `Core\Order`'s cases ("`rsort`, `arsort` and `krsort` as one option rather
than three names"), and the **Throws** line of `Core\Cli::ask`. The rule for what a stub reader
needs is `AGENTS.md` § *Text an end user reads*; nothing in this work rewrote a card.

## Next group

Nothing is scheduled from this work. The one thing the user may want first is the live run above; a
goal is worth adding only if the cards are to be rewritten in the plain voice.

## Backlog

- **Read-only is on disk only.** VS Code shows the read-only badge for such a file only under
  `files.readonlyFromPermissions: true`; otherwise a save into a stub fails with the editor's own
  message, and the file is written over at the next stamp change.
- **`nvs_syntax::walk`'s module doc claims an attribute's nested expressions are children of the
  declaration; they are not, and making them so reformats the corpus** (ADR 0214 § *Alternatives
  rejected*). The doc sentence is left as it was; fixing it is a one-line docs slice.
- **A generic `Core` class is spelled without its type parameters in its stub** (`final class
  ObjectMap`), and a method's `T` is written as a bare name the parser reads as a class; an options
  bag with a reserved-word key (`default`) and a shape of callables are spelled `object`. Every stub
  parses (`every_stub_parses`), and the card beneath still names every key.
- **The extension names the stub directory by its own version, not the server's**, since the server's
  is known only after `initialize`; the stamp inside the directory makes that a namespace and nothing
  more.
- `an-import-answers-none.lspt` was left as it is: its import resolves to nothing, and that still
  answers `none`. The `Core` import case is `an-import-of-a-core-type-answers-its-stub.lspt`.
- `Core\Str` and every other class still owe a `ClassDoc` `short` — goal `core-class-cards` — so a
  class hover shows the intro alone where a page exists, and the declaration line alone where none.
