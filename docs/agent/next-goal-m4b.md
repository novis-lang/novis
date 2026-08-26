# Loop goal — M4B (staged; not yet live)

**This file is not the running goal.** It is written ahead of time so the switch is a rename rather than
an authoring session. When the M4S run reaches its acceptance list, do the four things in
*Switching to this goal* at the foot of this file, and this becomes `docs/agent/loop-goal.md`.

---

Finish **M4B** — [docs/plan/m4b.md](../plan/m4b.md) is the scope and this file does not restate it. Read
that milestone paragraph; the decisions behind it are
[ADR 0040](../adr/0040-vscode-deep-tooling-and-resilient-parsing.md) and
[ADR 0099](../adr/0099-the-resilient-tree-is-the-ast-plus-trivia.md), and 0099 is the current rule
wherever the two ever seem to differ.

What is different about this goal, and what every session should hold: **for the first time the loop is
verifying something that is not a program's stdout.** An LSP answer is not printed by anything, and the
driver's stop path is exit codes and exact output only. The mechanism that closes that gap is a `.lspt`
case and `mwl lsp-test`, and it lands in Stage 2 **before any request handler does**, because a request
built before its case format exists is a request nobody can prove.

Three things this goal is *not*: it is not M10's deep half (no rename, no extract, no workspace symbol
search, no inlay hints, no signature help, no Test Explorer, no profiler, no debugger UI), it is not
`mwl fmt` (M10, [ADR 0039](../adr/0039-canonical-code-formatting.md)), and it is not PhpStorm (M10,
[ADR 0016](../adr/0016-ide-integration.md)). A slice that reaches for one of those is off path: it goes in
`## Backlog` in the handoff and the session moves on.

## Stage 0 — catch up before anything else

Four M4 language holes. Each is here rather than in the backlog for one reason, and it is not tidiness:
**each one silently limits what a case can be written in**, and this goal writes hundreds of cases. The
playbook already carries bullets whose entire content is "you cannot spell this"; closing an item here
deletes its bullet rather than growing it. Until this stage is empty a session takes its work from here,
in this order.

1. **`Class::method(...)` does not panic `mwl-ir`.** The first-class callable
   ([ADR 0027](../adr/0027-callable-is-closures-only.md)) reaches lowering and panics outright, which is
   why every existing case that needs a helper declares a `class` with a `public static function` and
   calls it directly. `mwl-ir` gap 1.
2. **A closure is callable through the variable holding it** — `$f(...)`. Only native `Core` code calling
   back through `mwl_runtime::call_closure` works today. `mwl-ir` gap 9. Same file set as item 1: both are
   `mwl-ir`'s call lowering, so they are **one group**.
3. **`do`/`while` lowers.** The one M4 control-flow statement that does not; every terminator it needs
   already exists. `mwl-ir` gap 1.
4. **`bool as int` and `bool as string` lower**, and `false as string` renders `""` rather than nothing at
   all — [ADR 0007](../adr/0007-explicit-type-system.md) § 2's conversion table. `mwl-ir` gap 4.

Each item finishes with a `.mwlt` case under `tests/conformance/lang/` and the playbook bullet it
obsoletes **deleted**, not reworded.

## Stage 1 — the non-regression floor

The whole of the M4S goal's acceptance list, **unchanged and never traded for anything above it**: all
twenty-three fixtures at their frozen output, both `mwl test` suites at their final counts, every
`cargo-named` guard, the WSL leg and the valgrind sweep. It is copied verbatim into
`loop-goal.toml` at switch time with only its `stage` field relabelled — see *Switching to this goal*.
Nothing in M4B touches `mwl-stdlib`, so a failure here is a real regression and never a scope question.

## Stage 2 — the keystone: one lossless tree, and the format that can check it

Two things, and neither can be deferred behind the other. The tree is what every request reads; the case
format is what proves any of it. **No request handler is written until both are green.**

5. **Trivia.** `Lexer` gains a recording flag; `skip_trivia` ([lexer.rs:326](../../crates/mwl-syntax/src/lexer.rs))
   pushes a `Trivia { kind, span }` instead of only advancing. `TriviaKind` is `Whitespace`,
   `LineComment` (`//` and `#` alike) and `BlockComment`. That one function is the whole difference
   between the current token stream and a lossless one — there is no second site where a byte disappears,
   so do not go looking for one. Finishes with the **losslessness test**: concatenating every token's and
   every trivium's text in offset order reproduces the file byte-for-byte, over `examples/`, `tests/` and
   the `php-src` corpus `corpus_parse.rs` already walks.
6. **Explicit recovery.** `MemberName::Missing(Span)` beside `MemberName::Ident`
   ([expr.rs:762](../../crates/mwl-syntax/src/parser/expr.rs) is where the synthesized one is made today),
   and a span on `ExprKind::Error` naming what it stood in for. A consumer must never infer "did the user
   write this, or did the parser invent it at the cursor" from an empty span — completion's entire
   behaviour hangs on that distinction. Same file set as item 5: **one group.**
7. **`Parsed` and `SyntaxIndex`.** `parse_file_resilient` beside
   [parse_file](../../crates/mwl-syntax/src/parser/mod.rs) at `parser/mod.rs:475`, returning
   `{ stmts, trivia, index }`; the index is built by one walk and answers `at(offset) -> NodePath` — the
   innermost node plus its ancestors. `parse_file` becomes a thin wrapper so **no existing call site
   changes**, and `mwl check`/`mwl run` keep their behaviour exactly.
8. **The prefix sweep, and the fuzz target.** Every prefix of every `examples/*.mwl` at a token boundary:
   no panic, a `SyntaxIndex` answer at the final offset, and a diagnostic on each prefix that is genuinely
   incomplete. Then a `fuzz_target` over truncated and mid-edit inputs, separate from the existing
   whole-file `parse` target because a truncated input is a different shape of input
   ([ADR 0099](../adr/0099-the-resilient-tree-is-the-ast-plus-trivia.md) *Verification*). Same file set as
   item 7: **one group.**
9. **`.lspt`, and `mwl lsp-test`.** Extract `mwl_test`'s section lexer
   ([case.rs:171](../../crates/mwl-test/src/case.rs) `header`, `case.rs:226` `parse`) into a module both
   formats read, then build the `.lspt` case: `--TEST--`, `--FILE--`, `--FILE <path>--`, a `<|>` cursor,
   `--REQUEST--` and a frozen `--EXPECT--`. The canonical rendering of every response kind lives in
   `mwl_lsp::render` and **nowhere else**, so no case invents a spelling. `mwl lsp-test <paths>` walks
   directories for `*.lspt` and prints `N passed, M failed` — the exact line `tools/loop.py`'s `mwl-suite`
   kind parses, which is why the driver needs no change to gate editor behaviour.
   `--coverage` prints the request × construct matrix, **inferred from the node each cursor resolved to
   and never declared by the case**, and `every_request_answers_every_construct` fails naming each empty
   cell. That guard is this goal's real definition of done, the way
   `every_part_one_spec_member_is_registered` was the last one's: a count of cases is a proxy, a test that
   enumerates the grammar is not.

## Stage 3 — the server

`crates/mwl-lsp`, then one request at a time. **Every request slice ships its own `.lspt` cases**; a
handler landing without them is not a finished slice.

10. **The crate, the subcommand and the handshake.** `lsp-server` + `lsp-types`, synchronous, stdio,
    behind `mwl lsp`. `initialize` declares exactly the capabilities of *Stage 3*'s items and no others.
    Position encoding is negotiated per LSP 3.17 — offer `utf-8` and `utf-16`, take `utf-8` when the
    client offers it — which needs `utf16_col` and `offset_of` beside
    [line_col](../../crates/mwl-diagnostics/src/source.rs) at `source.rs:95`, whose column counts `char`s
    and is therefore neither encoding. **Position arithmetic has one home and it is `mwl-diagnostics`.**
11. **The document store.** `Full` sync, open buffers overlaid on the `require`/`autoload` graph the
    document is the entry point of, a debounce (150 ms, `mwl.lsp.debounce`) and cancellation of an
    analysis whose document version nobody is looking at any more. Same file set as item 10: **one group.**
12. **`publishDiagnostics`** — the existing `mwl check` pipeline, at the negotiated encoding, with `code`
    and `codeDescription` filled from `Code`. Published for **open documents only**.
13. **`hover`** — the declared type under the cursor from `mwl_types::ExprTypeTable`
    ([expr_table.rs:433](../../crates/mwl-types/src/expr_table.rs)); for a `Core` member its
    `mwl_stdlib::registry` signature row ([registry.rs:637](../../crates/mwl-stdlib/src/registry.rs)); for
    a declaration, its own doc comment out of the trivia layer item 5 built.
14. **`definition`** — within the document or anywhere in its resolved graph.
15. **`completion`** — keywords filtered by position; members off a resolved receiver, instance and
    static, user classes and `Core` registry classes alike; enum cases after `Type::`; in-scope variables.
    **No workspace symbol search.** Items 13–15 share the `SyntaxIndex`-plus-type-table path, so they are
    **one group of three** if context allows and never fewer than one.
16. **`semanticTokens/full`** — the legend of [ADR 0099 § 4](../adr/0099-the-resilient-tree-is-the-ast-plus-trivia.md),
    `defaultLibrary` on a `Core` class and the `tainted`/`secret` modifiers included. That pair is the
    point of this item, not a detail of it.
17. **`documentSymbol`** — namespace, class, interface, enum, method, property, class constant, type
    alias. One walk of the tree item 13 already needs: **group with item 16.**
18. **The two code actions** — casing and `(int)$x` → `$x as int`, both translations of a `Suggestion` the
    `Diagnostic` already carries, registered under `source.fixAll.mwl`. **A code action whose fix the
    checker would have to compute is off path**; that boundary is the whole content of this item.
19. **The latency guard** — a full re-analysis of a ~1,000-line document under a named bound, in the shape
    `benches/abi-probe/tests/perf_guards.rs` already uses. Stage 2 traded incremental reparse away; this
    is the measurement that says the trade still holds, and it is not optional.

## Stage 4 — `mwl ast --json`

20. **`--json` and `--resilient`** on [run_ast](../../crates/mwl-cli/src/main.rs) at `main.rs:187`. A node
    is `kind`, `span` as `[start, end]`, its own scalar fields and `children`; trivia and recovery nodes
    are **included**, because the panel is least useful on a file that compiles. `--resilient` is the
    default. Frozen by a snapshot test over `examples/`. ADR 0040 § 3 assumed this already existed; it
    does not, and `{stmts:#?}` has no stability contract.

## Stage 5 — the extension, and colour

`editors/vscode`. TypeScript, outside the Cargo workspace, exactly where
[ADR 0016 § 5](../adr/0016-ide-integration.md) puts it.

21. **The package.** `.mwl` registration (**and not `.php`**), `language-configuration.json`, `tsconfig`,
    lint, npm scripts, and its own test asserting `package.json` declares what the extension claims **and
    depends only on the allowlist** — that test is how "the extension holds no language logic" stops being
    a promise and starts being a check.
22. **The TextMate grammar**, and its headless snapshot test through `vscode-textmate` +
    `vscode-oniguruma` — plain Node, no editor, no display, so it runs on both legs. Everything it must
    colour, and the four constructs it must **not** colour as valid, is
    [ADR 0099 § 4](../adr/0099-the-resilient-tree-is-the-ast-plus-trivia.md)'s list; do not re-derive it
    and do not shorten it. This is the largest single item in the goal and it is expected to take more
    than one session — split it by construct family, not by file.
23. **The client** — spawning `mwl lsp` via `vscode-languageclient` with a configurable path falling back
    to `PATH`, the `LanguageStatusItem` for health and version, and the settings block. Plus the headless
    **protocol round-trip** that drives the real binary from Node.
24. **Tasks** for `mwl run` and `mwl test`, and **the AST panel** over `mwl ast --json --resilient`.
    Group with item 23: same `src/`, same test harness.
25. **The extension host and the artifact** — the `@vscode/test-electron` suite (activation on `.mwl` and
    not `.php`, Tasks present, status item rendering, panel populating, and **the semantic-token legend
    the client registers equal to the one the server declares**, which no unit test on either side alone
    can see), `.vsix` packaging, and the CI job. Memoized against the green tree the way the valgrind
    sweep already is; **not** on the per-iteration path.

## Acceptance

**The checks themselves live in [`loop-goal.toml`](loop-goal.toml), and only there** — every fixture, its
exact expected output, both suites, the `.lspt` suite, the `command` checks and every named guard, as
data the driver reads directly. `python tools/loop.py --list` prints it as a summary. What the check kinds
mean and how the legs are ordered: [coordinator.md](coordinator.md) § *The acceptance test*.

The one thing to hold about this goal's stop condition: **`every_request_answers_every_construct` is the
gate, not a case count.** A `min_passing` on the `.lspt` suite is a proxy that can be satisfied by six
hundred cases about the same construct; that guard reads the grammar and fails naming the cells nobody
covered. [loop-authoring.md](loop-authoring.md) § 3 is why.

**The expected output in that file is frozen; a case's *source* is not.** Correcting a `.lspt` case's
document, cursor or request is a bug fix and needs no permission. Editing its `--EXPECT--` to make it pass
is the thing that must never happen — re-freeze deliberately, say why in the commit message, move on.

## Standing decisions — pre-authorized, do not stop the loop for these

Every one was settled with the user before the run. Implement it; do not re-open it.

- **Decide and record; never `BLOCKED` for a design call.** Record in the home AGENTS.md already names —
  the crate's own module doc, or a paragraph in `docs/adr/README.md` § *Decisions taken at project start*.
  **Do not open a numbered ADR.** Reserve `BLOCKED` for a decision expensive to reverse *and* with no safe
  default.
- **One grammar, one tree — the `rowan` question is closed.** ADR 0099 § 1 settles it and names what was
  given up (incremental reparse) and what protects the trade (item 19's guard). If the implementation
  forces the opposite conclusion, keep this design, record *that* in `mwl-syntax`'s module doc with the
  reason, and put the CST in the handoff's Backlog — do not start a rewrite mid-run.
- **`lsp-server` and `lsp-types`, and no async runtime.** `tokio` does not enter this workspace. If a
  needed capability appears to require it, that is a real `BLOCKED` naming the capability — not a
  judgement call.
- **Dependencies: three are named, the rest are yours.** `lsp-server` and `lsp-types` for the server;
  `vscode-languageclient` for the extension. Everything else — the Node test stack, the grammar test
  libraries — you pick against [ADR 0051 § 4](../adr/0051-standard-library-tiers.md)'s two questions. A new
  **Rust** dependency owes three things (AGENTS.md): the `[workspace.dependencies]` line with a comment
  saying why that crate, `cargo deny check`, and `python tools/gen-attribution.py`. A new **npm**
  dependency owes the allowlist entry item 21 builds and nothing else; npm dependencies are `devDependencies`
  wherever they can be, because a runtime dependency of the extension ships to users and a test library does
  not.
- **The extension is `.mwl` only.** It does not claim `.php` even though `mwl-syntax` parses it — that
  fight is with every PHP extension a user already has, and losing it silently looks like MWL being broken.
- **Nothing is published.** `.vsix` as a CI artifact; no Marketplace publisher, no listing, no icon or
  branding work. ADR 0016 *Revisiting* keeps that open and this goal does not close it.
- **Six requests and two code actions, and that list is closed.** Anything else from ADR 0040 § 3's
  catalog is M10's. A session that finds a seventh request "would be easy" puts it in Backlog.
- **Colour is specified, not designed.** ADR 0099 § 4 lists what the grammar must colour and what it must
  refuse to colour, and lists the semantic token types and modifiers. Neither list is a starting point to
  improve on during the run; a gap in it is a handoff note.
- **The extension-host tier never gates an iteration.** It is memoized against the green tree. A session
  that finds it red fixes it like any other check; a session that finds it *unrunnable* (no display, no
  cached VS Code build) says so in the handoff and does not spend the session on the machine.
- **`.lspt` and `.mwlt` stay two suites.** They share a section lexer and nothing else. Do not add LSP
  sections to `.mwlt`: `mwl test`'s `N passed` is the number Stage 1's floor gates on, and making it count
  two unlike things breaks that gate and `conformance_coverage.rs` with it.
- **Backlog items are off-path unless the goal needs them.** `## Backlog` in the handoff, and move on.
- **Doc trimming is not loop work, ever**, and neither is a dependency sweep. Both are the user's to fire.

## The gaps that actually sit on the path

Named because none is visible from the milestone's text, and each is work rather than a question.

- **`mwl ast --json` does not exist** — ADR 0040 § 3 says it does. Item 20.
- **`SourceFile::line_col` counts `char`s** ([source.rs:95](../../crates/mwl-diagnostics/src/source.rs)),
  which is neither UTF-8 nor UTF-16. Every column an LSP answer carries is wrong the moment a line holds a
  multi-byte character, and it is invisible on ASCII — which is what every fixture in this repository is.
  Item 10 fixes it once, in `mwl-diagnostics`, and no other crate may do its own conversion.
- **`orient.py` cannot see `editors/`.** Its `[context] modules` globs `crates/*/src/**/*.rs` only, so a
  session working in TypeScript orients on nothing. The tooling change lands with this goal; if a session
  finds it still missing, that is item 21's blocker and belongs in the handoff.
- **The extension's tests are the first non-Rust, non-Python thing this repository runs.** `verify.py`
  grows a step and `loop.py` grows a `command` check kind. Both land with this goal.
- **`Diagnostic::suggestions` has been carried since M0 and read by nothing.** Item 18 is its first
  consumer; expect the field to be sparsely populated in practice, and expect that to be the actual work
  of that item rather than the code-action plumbing.
- **A `.lspt` case about completion is a case about a document that does not parse.** That is the point,
  and it means the case corpus is largely made of deliberately broken files. A case that only ever asks
  about valid code is not testing what M4B exists to test.

## Switching to this goal

Four steps, and the third is the one that is easy to skip and expensive to skip.

1. Confirm the M4S run reached its acceptance list: `python tools/loop.py --goal-only` is green.
2. `python tools/loop-stats.py` and `python tools/loop-stats.py --attribute` — [loop-authoring.md](loop-authoring.md)
   § 1 makes this step zero, and § 9 says the numbers move. Set the group cap from what it prints and
   **say which cap and why in the commit**.
3. `python tools/goal-switch.py docs/agent/next-goal-m4b.toml` — this copies every `[[check]]` out of the
   live `loop-goal.toml` into the new one as Stage 1's floor, relabelled, and writes the result. Doing it
   by hand is how a floor gets silently dropped.
4. Rename this file to `loop-goal.md`, drop this section, and rewrite `docs/agent/handoff.md` naming item 1
   as the first group.
