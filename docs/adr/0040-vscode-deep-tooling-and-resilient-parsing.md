# ADR 0040 — The VS Code extension is a deep, first-class client; `mwl-syntax` gains a resilient parse mode; a minimal `mwl-lsp` moves ahead of M10

- **Status:** Accepted
- **Date:** 2026-08-22
- **Scope:** the VS Code half of [ADR 0016](0016-ide-integration.md) only — the PhpStorm plugin, and
  0016's "one server, two thin clients" split, are untouched. Covers: (1) pulling a minimal `mwl-lsp` and
  `editors/vscode` ahead of M10 into a new milestone, **M4B**, right after M4's "usable CLI language"; (2)
  the full v1 feature catalog for the VS Code extension — inspections, refactorings, completion depth, a
  Test Explorer with coverage, an AST panel, profiler visualization, debugger UI — each tagged with the
  language/runtime feature it is staged behind; (3) a required addition to `mwl-syntax`: a second,
  error-recovering parse mode, alongside the existing strict one, needed for usable completion on a file
  that is mid-edit.
- **Amends:** [docs/implementation-plan.md](../implementation-plan.md) M4/M5/M10 — inserts M4B between M4
  and M5, and narrows M10's remaining VS Code scope to what M4B does not cover.
- **Relates to:** 0006, 0016, 0018, 0019, 0039

> **In short:** VS Code is the reference client, and it is getting real depth, not a thin LSP passthrough
> with a grammar file. A **minimal `mwl-lsp`** (diagnostics, hover, go-to-definition, basic completion) and
> the **`editors/vscode` extension** move out of M10 into a new milestone, **M4B**, placed right after M4 —
> the point the plan already calls "a usable CLI language" — so real programs can be written and tested in
> VS Code from that point on, not from M10 onward. M10 still exists and still lands `mwl fmt`/`mwl dap`/the
> profiler/`mwl pkg`, but its remaining VS Code work is now the *deep* half: inspections and quick fixes,
> refactorings, richer completion, a native Test Explorer with coverage, an AST/reflection panel, a
> profiler view, and DAP editor wiring — each one named below against the specific milestone or ADR that
> has to land first, so nothing here is a promise with no delivery date and nothing blocks on the hardest
> piece. The one change that reaches back into the compiler: `mwl-syntax` gains a second, **error-recovering
> parse mode** — a lossless tree that keeps a usable shape around a syntax error, the same design
> rust-analyzer's `rowan` crate popularized for exactly this reason — because a document mid-keystroke is
> syntactically invalid most of the time, and "keep completion working anyway" is not achievable by
> layering something on top of an all-or-nothing parser afterward.

## Context

- [ADR 0016](0016-ide-integration.md) scoped the VS Code extension as a thin `vscode-languageclient`
  wrapper — TextMate grammar, LSP process spawning, format-on-save — landing in M10, after extensions (M9)
  and stdlib/HTTP (M7/M8). It explicitly left a Test Explorer and debugger UI wiring out of scope.
- The goal now is different: real-world testing of the language should start the moment non-trivial CLI
  programs run correctly, which the plan already names as M4's exit criterion ("a non-trivial CLI program
  … runs correctly"). Waiting until M10 — after concurrency, config, the HTTP server, the full stdlib and
  the extension system — puts real usage six milestones later than it needs to be.
- The user also asked for the deepest tooling VS Code makes possible: inspections, refactorings,
  introspection, top-tier completion — not just "the LSP passthrough works."
- Research done for this ADR, to ground it in what VS Code actually supports rather than assumption:
  - **rust-analyzer's architecture** is the closest existing precedent for a from-scratch systems-language
    LSP. It parses into a **lossless CST** (via the `rowan` crate — a red/green tree design originating in
    Roslyn and adopted by Swift's `libsyntax`) that always produces *some* tree for *any* input, with parse
    errors, comments and whitespace kept in the tree rather than discarded. rust-analyzer's own docs call
    the underlying technique "infallible parsing": for IDE use, a parser must always produce output, or
    every diagnostic and every completion downstream of the cursor position goes dark the moment there is
    one syntax error above it. It then layers an incremental, salsa-based recompute engine on top so a
    single keystroke only re-derives what actually changed.
  - **VS Code's Test Coverage API has been finalized** (shipped as stable API, not proposed) — a
    `FileCoverage` model carrying per-file statement/branch/declaration counts, feeding VS Code's own
    gutter/summary UI. There is no need to build a custom coverage-gutter renderer; a test provider that
    reports `FileCoverage` gets the native UI for free.
  - **VS Code's `LanguageStatusItem` API** is the sanctioned place for "is the language server alive, and
    what version" — replacing the old pattern of a hand-rolled status-bar item, and used by
    Java/Python-family extensions for exactly this purpose today.
  - **VS Code's built-in flame-chart profiler view is JS/V8-specific** (`.cpuprofile`, wired to the
    JavaScript debugger) — it is not a generic profiler-visualization surface a third-party language can
    plug into directly. The generic, widely-supported answer is the open **speedscope** JSON format:
    speedscope.app renders it standalone, and a thin VS Code command can shell out to it or embed it,
    without MWL building a bespoke flamegraph renderer.
  - **Debug Adapter Protocol (DAP)** is a wire protocol, not a UI: any DAP-compliant adapter gets VS Code's
    existing breakpoints/call-stack/variables/watch UI for free via a `DebugAdapterDescriptorFactory` and a
    `launch.json` schema contribution. No custom debugger UI needs to be built — the work is entirely the
    adapter (`mwl dap`, already planned) plus the small amount of editor-side registration glue.

## Decision

### 1. Timeline: a new milestone, M4B, between M4 and M5

Inserted as **M4B** (not renumbered into the M5–M14 sequence) specifically to avoid renumbering the ~30
cross-references to M5 through M14 across other ADRs for a purely additive milestone — the same
low-churn instinct [AGENTS.md](../../AGENTS.md) already applies to documentation. M4B pulls forward,
**scoped down to a minimal subset**, work that M10 was going to do anyway:

- `crates/mwl-lsp` — created here, not at M10 — implementing only: `textDocument/publishDiagnostics` (by
  running the existing `mwl check` pipeline against the resilient parse tree from *Decision § 3*),
  `textDocument/hover` (declared types, from `mwl-types`), `textDocument/definition`, and
  `textDocument/completion` restricted to keyword completion and member completion off a resolved
  receiver type (no cross-file symbol search yet — that needs the workspace-indexing work M10 still owns).
  A `LanguageStatusItem` shows server health/version per the research above.
- `editors/vscode` — created here, not at M10 — the TextMate grammar, `.mwl` registration and
  `language-configuration.json` from ADR 0016 § 2, `mwl lsp` process spawning, and `mwl run`/`mwl test` as
  VS Code Tasks. **Not yet included:** format-on-save (`mwl fmt` doesn't exist until M10 —
  [ADR 0039](0039-canonical-code-formatting.md)), rename, code actions, and semantic tokens beyond what the
  minimal completion/hover data already supports.
- `mwl-lsp` and `editors/vscode` are **one crate/one package each across both milestones** — M10 extends
  the same crate and the same extension in place rather than standing up a second "real" implementation
  next to a throwaway M4B prototype. Building a disposable prototype and discarding it at M10 was
  considered and rejected — see *Alternatives rejected*.

M10's remaining VS Code scope after M4B lands is everything in *Decision § 4* below. M10's PhpStorm scope
is entirely unchanged from [ADR 0016](0016-ide-integration.md).

### 2. `mwl-syntax` gains a resilient, error-recovering parse mode

A live editor spends most of its time with a syntactically invalid document — mid-statement, an unclosed
brace, a half-typed identifier. M4B's completion/hover cannot go dark every time that happens, and it will
happen on nearly every keystroke. `mwl-syntax`'s existing parser is built for whole-file compilation
(`mwl check`/`mwl run`) and has no such mode today.

The addition, modeled on rust-analyzer's `rowan`-based approach from *Context*:

- A **second parser entry point**, not a rewrite of the existing one: the same lexer and the same grammar
  tables, run in a mode that never aborts on the first error. On a malformed construct it records an error
  node in the tree, at the source-position range where the failure occurred, and resumes parsing from the
  next syntactically recognizable point (the existing recursive-descent structure already tracks these
  synchronization points for its normal diagnostics; the resilient mode's addition is *not stopping* there
  instead of returning early).
- The output is a **lossless tree** — every byte of the source, including whitespace/comments, is
  recoverable from it — so `mwl-lsp` can map a cursor offset back to exactly the syntax node under it, even
  inside a malformed region, without a second position-mapping mechanism.
- `mwl check`, `mwl run`, and every other compile path are **unchanged**: they keep calling the existing
  strict, all-or-nothing parse. The resilient mode is additive and reachable only through `mwl-lsp` (and,
  later, `mwl fmt`/`mwl-ide`-adjacent tooling that wants the same tolerance) — one grammar, two entry
  points, not two grammars to keep in sync.
- **Where this lands relative to the priority ordering in [AGENTS.md](../../AGENTS.md):** this spends
  simplicity (priority 4) — a second parser mode to build and keep in step with the grammar while M2–M4
  still actively change it — and buys nothing on security, correctness, or the request path (priorities
  1–3), because it is never linked into the compiled artifact `mwl run` produces; it exists only in
  `mwl-lsp`'s process. That is a deliberate, bounded spend, not a trade against a higher priority.

This is scoped as a prerequisite for M4B's completion, and lands with it (not before it as separate
milestone work, and not deferred past it — a minimal LSP without resilient parsing would ship completion
that stops working on the first typo, which is the exact failure this ADR exists to avoid).

### 3. The v1 feature catalog, each item staged behind its dependency

Every feature below is a real commitment, not an aspiration — but each is tagged with what has to exist
first, so the catalog is honest about sequencing rather than implying all of it lands at once.

**Available from M4B (minimal `mwl-lsp`, no further dependency):**

- Syntax highlighting (TextMate baseline, semantic tokens once `mwl-lsp` responds), diagnostics, hover,
  go-to-definition, keyword/member completion, `mwl run`/`mwl test` as Tasks, a `LanguageStatusItem`.
- **An AST explorer panel**, backed by the CLI's existing `mwl ast` command (already shipped in M1 —
  `crates/mwl-cli`) rather than waiting on anything: a tree view rendering `mwl ast --json`'s output for
  the active file. This does not need `Core\Ast` ([ADR 0019](0019-reflection-and-ast-parsing-are-core-features.md))
  at all — that's the *language-level* reflective parse a running MWL program calls; the *editor* panel is
  simpler and can shell out to the CLI the same way `mwl check` already backs diagnostics.

**Gated on M10 (the LSP's remaining scope — workspace indexing, `mwl-fmt`, `mwl dap`, the profiler):**

- **Inspections and quick fixes** as LSP code actions, each backed by a diagnostic the checker already
  emits or will emit: a casing violation offers "rename to `camelCase`/`PascalCase`"
  ([ADR 0029](0029-identifier-casing-is-checked.md)/[0030](0030-no-leading-underscores-constructor-spelling.md));
  a legacy `(int)$x` cast offers "replace with `$x as int`" ([ADR 0034](0034-legacy-cast-syntax-rejected.md));
  a missing constructor property assignment offers to add it
  ([ADR 0022](0022-definite-property-initialization.md)); `include`/`require_once` offer "replace with
  `require`" ([ADR 0021](0021-single-file-inclusion-construct.md)); a `tainted`/`secret` value reaching a
  refusing sink offers the specific laundering call the diagnostic already names
  ([ADR 0024](0024-taint-tracking-for-injection-sinks.md)/[0033](0033-secret-qualifier-for-confidential-values.md))
  — offered as a suggestion the developer applies deliberately, never auto-applied on save, the same as any
  other code action.
- **Refactorings** as LSP requests: workspace-wide rename, extract-to-method/variable, organize-imports
  restricted to reordering and removing unused `use` statements — never introducing a rename or alias, per
  [ADR 0015](0015-no-name-aliasing.md)'s "nothing gets a second name."
- **Deeper completion**: signature help, cross-file/workspace symbol search, auto-import limited to
  inserting the correct fully-qualified name (never an alias, same ADR 0015 constraint), inlay hints for
  `var`-inferred types ([ADR 0037](0037-var-local-type-inference.md)) and call-site parameter names.
- Format-on-save and the format commands, wired to `mwl fmt` once it exists
  ([ADR 0039](0039-canonical-code-formatting.md)) — unchanged from ADR 0016 § 2.
- **A native Test Explorer**, using VS Code's finalized Testing API, wired to `mwl test`/`.mwlt`, with
  **coverage** fed through VS Code's own `FileCoverage` API from the Clover/lcov exporters M10 already
  builds ([ADR 0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)) — no custom
  gutter-rendering code, per the research in *Context*.
- **Profiler visualization**: `mwl run --profile` emits its sampling-profiler output in the open
  **speedscope** JSON format (in addition to whatever machine-readable format `mwl` itself wants); a "View
  Profile" command opens it in speedscope.app or an embedded webview that speaks the same format. No
  bespoke flamegraph renderer is built from scratch, per the research in *Context*.
- **Debugger UI wiring for VS Code**: a `DebugAdapterDescriptorFactory` and a `launch.json` configuration
  schema targeting `mwl dap`. This **reverses** [ADR 0016](0016-ide-integration.md) § 4's deferral for VS
  Code specifically — `mwl dap` already ships in M10, DAP is a protocol VS Code already renders a full UI
  for, and "staged behind its dependency" means once `mwl dap` exists in the same milestone, wiring it up
  is the small remaining step, not a separate fast-follow. **PhpStorm's debugger UI wiring stays deferred**
  exactly as ADR 0016 states — this ADR does not reach into the PhpStorm side at all.

**Gated on M9 (`.mwlx` extensions exist):**

- `mwl ext build`/`inspect`/`verify` surfaced as VS Code Tasks/commands, and a read-only webview rendering
  `mwl ext inspect`'s manifest/capability output. Lands whenever M9's tooling exists — in practice
  alongside or after M10's editor work, since M9 already precedes M10 in the plan.

**Named but not committed to any milestone (see *Revisiting*):**

- A live, debug-session object/value inspector built on `Core\Reflect` ([ADR 0019](0019-reflection-and-ast-parsing-are-core-features.md)) — distinct from the static AST panel above, this needs both the stdlib
  reflection surface (M7/M8) and `mwl dap` (M10) to exist together, and a design for how a DAP `variables`
  request surfaces a reflected object graph.
- A request-tree visualization for `spawn`/`spawn script` ([ADR 0006](0006-isolated-script-execution.md))
  during a debug session — this would need `mwl dap` to expose isolate/task-tree structure through a
  custom DAP extension, which is a real protocol-design question, not just editor glue.

## Consequences

**Positive**

- Real MWL programs can be written and debugged-by-inspection (diagnostics/hover/completion, even without
  a debugger yet) in VS Code from M4B onward — roughly five milestones earlier than ADR 0016's plan — which
  is the whole point: the language gets exercised by a real editor while M5–M9 are still being built,
  surfacing rough edges in the type system, stdlib shape and diagnostics wording while they are still cheap
  to change.
- The feature catalog in *Decision § 3* answers "how deep does VS Code support go" concretely, with no
  feature left as an unscoped "eventually" — each has either a milestone or an explicit *Revisiting* entry.
- Reusing VS Code's native Test Coverage API, `LanguageStatusItem`, and DAP's existing debugger UI, and the
  open speedscope format for profiling, means MWL avoids building and maintaining four different pieces of
  UI infrastructure that already exist and are already maintained elsewhere — a direct instance of
  [AGENTS.md](../../AGENTS.md)'s simplicity priority.
- The resilient parse mode in *Decision § 2* is additive to `mwl-syntax`, never touches the compiled
  artifact's code path, and is exactly the design (`rowan`-style lossless CST) an existing, heavily-used
  language tool (rust-analyzer) already validated at scale for the identical problem.

**Negative**

- `mwl-syntax` now carries two parser entry points sharing one grammar, and the resilient one has to be
  kept in step with every M2–M4 grammar change while the front end is still under active development —
  real, ongoing maintenance cost, not a one-time addition. This is the cost named in *Decision § 2*'s
  priority-ordering paragraph.
- M4B is new scope inserted into the plan, not free: a minimal `mwl-lsp` and `editors/vscode` have to be
  built, tested and kept working through M5–M9 even though nothing in those milestones depends on them —
  the same "keep it running" burden any early-shipped surface carries.
- M10's estimate needs revising: it no longer includes the *minimal* VS Code work (moved to M4B), but it
  gains real scope ADR 0016 previously deferred or excluded — inspections/refactorings, a Test Explorer
  with coverage, profiler visualization, and now VS Code's debugger UI wiring specifically. Net effect on
  the 14-week estimate is not assumed to be zero.
- VS Code's debugger UI wiring landing in M10 for VS Code but not PhpStorm means the two editor clients are
  now visibly asymmetric in a way ADR 0016 did not have (both were symmetric "LSP-bridge, no debugger UI").
  That asymmetry is intentional — the user's ask was to go deep on VS Code specifically — but it is a
  divergence a future PhpStorm-depth pass (out of this ADR's scope) will need to address or explicitly
  accept.

## Alternatives rejected

- **Leave VS Code entirely at M10, as ADR 0016 originally scoped it.** Rejected: conflicts directly with
  wanting real-world testing to start once M4 produces a usable CLI language, not after M5–M9.
- **Build a throwaway M4B prototype extension/LSP, discarded before M10's "real" implementation.** Rejected:
  two implementations of the same client/server pair drift and duplicate work, the identical reasoning
  [ADR 0016](0016-ide-integration.md) § 1 already applies to formatting/language-smarts logic, applied here
  to the editor packages themselves.
- **Skip resilient parsing; accept that completion stops working on a syntax error until it's fixed.**
  Rejected per the explicit goal of top-tier completion — a document is syntactically invalid for most of
  the time a developer is actively typing in it, so this would mean completion rarely works when it matters
  most.
- **Build custom UI for coverage gutters and profiler flamegraphs instead of VS Code's native Test Coverage
  API and the speedscope format.** Rejected: both already exist, are already maintained, and building
  MWL-specific equivalents is pure unnecessary scope against the simplicity priority.
- **Renumber M5–M14 to M6–M15 to fit M4B in sequence.** Rejected: ~30 cross-references to those milestone
  numbers exist across other ADRs; renumbering them is pure churn for a naming preference, not a
  correctness requirement — "M4B" reads unambiguously as "between M4 and M5."

## Revisiting

- **The equivalent deep-dive for PhpStorm** — this ADR is VS-Code-only by the user's explicit request to
  start there; PhpStorm stays exactly at [ADR 0016](0016-ide-integration.md)'s LSP-bridge scope, debugger UI
  included, until a future ADR does for PhpStorm what this one does for VS Code.
- **A live, `Core\Reflect`-backed debug-time object inspector**, and **a request-tree visualization for
  `spawn`/isolates during a debug session** — both named in *Decision § 3*'s uncommitted list, pending the
  stdlib/DAP prerequisites they need and, for the request tree, a DAP protocol-extension design.
- **Whether M4B's minimal `mwl-lsp` should also carry one or two cheap code actions early** (e.g., the
  casing quick fix, since the diagnostic already exists in M2) if they turn out to be low-cost — left open
  rather than decided now, so M4B doesn't scope-creep back toward M10's catalog.
- **Whether the resilient parse mode's error-node recovery quality needs its own fuzz target** (feeding
  deliberately-truncated/mid-edit inputs, distinct from the existing `parse` fuzz target's whole-file
  inputs) — worth deciding once M4B's implementation starts, not before.

Verification, in the order it becomes possible:

- **M4B:** the VS Code extension activates on `.mwl`, shows TextMate colour immediately and semantic-token
  colour once `mwl-lsp` responds; diagnostics/hover/go-to-definition/keyword-and-member-completion round
  trip through `mwl-lsp`; the AST panel renders `mwl ast --json`'s tree for the active file; typing an
  incomplete statement (unclosed brace, trailing `->`) does not stop completion from working on the
  well-formed code around it — the resilient-parse mode's core claim, tested directly.
- **M10:** every inspection/quick-fix/refactoring in *Decision § 3* round-trips as an LSP code action or
  rename request with no logic duplicated into the extension; format-on-save matches `mwl fmt --check`
  byte-for-byte; the Test Explorer runs `.mwlt` cases and shows coverage sourced from the Clover/lcov
  exporters; a captured profile opens correctly in speedscope; a breakpoint set in VS Code's UI hits in
  JIT-compiled code with correct variable values via the wired-up `mwl dap` adapter — with no
  MWL-authored debugger UI code, only the descriptor factory and schema contribution.
