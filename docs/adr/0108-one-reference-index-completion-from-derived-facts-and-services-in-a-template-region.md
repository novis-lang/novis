# ADR 0108 — One reference index answers five features, editor completion may only offer what the compiler already derived, and a template region gets services but no second formatter

- **Status:** Accepted
- **Date:** 2026-08-28
- **Scope:** the **M10** half of the VS Code catalog only — six additions that a review of an existing,
  mature PHP extension found missing from it, plus the boundary rule that decides which of that product's
  features Novis builds and which it structurally does not need. Covers: (1) the reference index and the five
  features that are projections of it; (2) what editor completion may draw on, which is the whole
  "framework awareness" question; (3) HTML/CSS/JavaScript services inside an inline-HTML region, and why
  they stop short of formatting; (4) code actions that *generate* rather than fix; (5) the DAP capability
  list the debugger UI is only as deep as; (6) `nvs check --json`, and the identifiers all of the above
  add. It does **not** touch M4B's frozen request set or contributions
  ([0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) §§ 1–6), `nvs fmt`'s style
  ([0039](0039-canonical-code-formatting.md)), the PhpStorm plugin ([0016](0016-ide-integration.md) § 3),
  or anything in the runtime.
- **Amends:** [0040](0040-vscode-deep-tooling-and-resilient-parsing.md) § 3 — its M10-gated catalog gains
  six entries, folded into that ADR's own body;
  [0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 6 — the frozen roster gains three settings,
  one command, one request of Novis's own and an `extensionKind`, added and never renamed under that
  section's own rule. [docs/plan/m10.md](../plan/m10.md) is rewritten to match.
- **Amended by:** none.

> **In short:** Novis's editor plan already matched a mature commercial PHP extension on everything a
> *language server* does. Everything it was missing sat in the layer above: **finding a symbol's other
> uses**, which five separate-looking features are all projections of; **completing a value the program
> itself defines**, which is what "framework support" actually means once you strip the guessing out of it;
> and **editing the template half of a file**, which for Novis is not a template language at all but inline
> HTML ([0082](0082-the-first-party-framework.md) — "the view layer is the language"), and today gets
> colour and nothing else. Six additions land at M10, and one rule decides the shape of the second:
> **`nvs-lsp` may complete a value only where the compiler already derives that value for another
> reason** — a route name because routing is resolved while compiling
> ([0077](0077-compile-time-routing.md)/[0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md)),
> a configuration directive because the runtime validates against that same registry
> ([0064](0064-configuration-file-format.md)/[0103](0103-configuration-is-a-tree-of-files.md)) — and never
> from a convention scan, an annotation dialect or a directory layout. That rule is what keeps framework
> knowledge out of the server while giving the feature anyway, and it is also why the largest single
> feature of the product reviewed — resolving a template engine's components and an ORM's columns — has no
> counterpart here to build: Novis has neither ([0082](0082-the-first-party-framework.md)). The embedded
> services get one hard edge: **services yes, formatting no**, because a second formatter inside a `.nvs`
> file is the exact thing [ADR 0039](0039-canonical-code-formatting.md) exists to prevent.

## Context

- [ADR 0040](0040-vscode-deep-tooling-and-resilient-parsing.md) set out to be the *full* v1 catalog for the
  VS Code client, and [ADR 0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) froze what M4B ships of
  it. Neither was written against a worked example of a finished product in the same niche.
- **The review that produced this ADR** read DEVSENSE's PHP Tools documentation set in full — its VS Code
  feature pages, the deeper Visual Studio feature set the same engine drives, its Zed integration, and two
  years of release notes — and diffed it against 0016/0040/0099 and the M4B/M10 milestones. The finding
  was one-sided in a useful way: on language-service fundamentals — completion, hover, definition, symbols,
  diagnostics, semantic tokens, folding, signature help, inlay hints, rename, extract, organize-imports,
  format-on-save, a Test Explorer with coverage — the plan was already equivalent or ahead, and in two
  places (`secret` redaction from server-computed ranges, [0101](0101-secret-is-redacted-in-the-editor-and-the-range-comes-from-the-server.md);
  phase-gated diagnostics and `.lspt` cases, [0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) §§ 3
  and 5) it has no counterpart there at all. Every real gap was in the layer above LSP's core requests.
- **`textDocument/references` appeared nowhere in the plan.** M10 commits to workspace-wide rename, which
  cannot be built without the same index, so this was an omission rather than a decision — and four further
  features that look unrelated (occurrence highlight, CodeLens, type hierarchy, dead-member dimming) are
  each one query against that one index.
- **Their differentiator is framework-aware IntelliSense**, and reading how it is built is the argument for
  Novis doing it differently rather than not at all: Eloquent columns are recovered by scanning migrations
  and factory `definition()` bodies, facades are followed through a runtime service container or, failing
  that, a `@mixin` annotation, Blade components are found by convention in default directories, and a
  vendor-specific `ide.json` exists to patch what none of that reaches. Each is a heuristic standing in for
  a fact the language does not make available. Novis makes those facts available: routing resolves while
  compiling, dependency injection is resolved while compiling
  ([0061](0061-compile-time-autoload-and-program-discovery.md)), there is no ORM and no runtime container
  ([0082](0082-the-first-party-framework.md)), and the view layer is the language rather than a second
  templating grammar.
- **Their template story is the one place they are unambiguously ahead of the plan.** PHP Tools enables VS
  Code's own HTML, CSS and JavaScript language services *inside* a `.php` file: Emmet, tag closing and
  renaming, the CSS colour picker, validation, hover. M4B gives an inline-HTML region a TextMate scope and
  nothing else. Since [ADR 0082](0082-the-first-party-framework.md) makes inline HTML the template engine,
  that region is not an edge case in Novis — it is where a web application's markup is written.
- **What is *not* worth copying is as informative**, and their pricing page is the evidence: the features
  behind the paywall are whole-workspace analysis, a per-rule configurable formatter, and completion-list
  re-ranking. Two of those three are things [ADR 0039](0039-canonical-code-formatting.md) and
  [ADR 0029](0029-identifier-casing-is-checked.md) already refuse on principle — a formatter with ~40
  rule settings across nine named code styles, and diagnostic severity configurable per file through
  `.editorconfig`, `settings.json` globs and in-source suppression tags, are the mature form of exactly the
  configurability those two ADRs closed the door on.

## Decision

### 1. One reference index, and the five features that are projections of it

M10 already builds a workspace index for symbol search and rename. **`textDocument/references` is that
index's read side, and it is named here because it was absent**, not deferred. Four further features are
each one query against it and none gets a walk of its own:

- **`textDocument/documentHighlight`** — the occurrences of the symbol under the cursor, within the file.
  [ADR 0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 6 kept this out of M4B because it needs
  resolution applied to *every* occurrence rather than to one, which is the same thing the index is; that
  reasoning places it here rather than nowhere.
- **CodeLens** above a declaration: reference count, and for a type its implementors, and for a method the
  declarations it overrides and the one it overrides. Gated on `nvs.codeLens.enable` (default `true`) — a
  lens is a request per visible declaration and a large file is where it is least welcome.
- **`textDocument/typeHierarchy`** — supertypes and subtypes of a class or interface, including
  [ADR 0043](0043-interface-default-methods-and-delegation-replace-traits.md)'s default methods and
  delegation, which is where a reader most needs to see the shape rather than reconstruct it.
- **Unused-member dimming.** A private member, constant or `use` with no reference anywhere in the index is
  reported as a diagnostic carrying LSP's `Unnecessary` tag, which VS Code renders as dimming rather than a
  squiggle. Its code is allocated from `nvs-diagnostics`'s registry when the slice lands
  ([conventions](../agent/conventions.md) § *A diagnostic* is the procedure). This one is only correct at
  workspace scope — a symbol unused in the open buffer is not unused — which is why it and § 6's
  `nvs.check.scope` land together, and why it is silent under the default scope rather than wrong.

**Call hierarchy is deliberately not in this list.** `textDocument/callHierarchy` is a different index —
edges between call sites, kept incrementally — not a query over the symbol index, and nothing else needs it.
It goes to *Revisiting* rather than into a milestone that would then carry it.

### 2. The editor completes a value only where the compiler already derived it

This is the whole of Novis's answer to "framework support", and it is a **closed** rule rather than a
starting point:

> `nvs-lsp` contains no framework-specific module, no annotation dialect, no convention scan and no
> directory-layout knowledge. It offers a value in a completion list only where the compiler already
> derives that value for another reason, and it reaches it through the same table that other reason uses.

What that admits at M10, each with the table it reads:

- **Route names and their parameters**, in the functions that take one — the route table is built while
  compiling ([0077](0077-compile-time-routing.md)) and
  [0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md) § *Decision*
  gives a route a name, a parameter list and a constraint on each. A misspelled route name is otherwise
  found at the moment a user clicks the link.
- **Configuration directives**, in `nvs.toml` and every file `[[include]]` pulls in — the directive set is
  closed ([0064](0064-configuration-file-format.md)) and the runtime already validates against it, so
  completion, hover with the directive's type and default, and "no such directive" are three readings of
  one registry. `[[include]]`'s `path` and `dir` complete as paths. This is the one place the extension
  answers for a document that is not `.nvs`, and it is scoped to the config tree the workspace actually
  has, never to every TOML file.
- **`#[Api]` attribute fields** ([0085](0085-openapi-is-generated-from-the-route-table.md)) and every
  other attribute's shape literal ([0046](0046-attributes-shape-literal-metadata.md)), which is a declared
  type like any other.
- **Enum cases, members off a resolved receiver, and in-scope variables** — already M4B's, listed here only
  because they are the same rule and not an exception to it.

What the rule refuses, and why each is a category Novis does not have rather than a feature it declines:

- **ORM column completion** has no subject: there is no ORM. Data access is
  [0067](0067-core-db.md)'s prepared statements over [0071](0071-derived-codecs.md)'s derived codecs, and a
  codec's fields are a class's declared properties, which completion already reaches as members.
- **Service-container and facade resolution** has no subject: wiring is constructor injection resolved
  while compiling ([0082](0082-the-first-party-framework.md)), so the thing a name resolves to is a
  declaration, and go-to-definition already goes there.
- **Template-component and view-name completion** has no subject: the view layer is the language
  ([0082](0082-the-first-party-framework.md)), so a view is ordinary code reached by ordinary navigation.
- **A vendor annotation dialect or an `ide.json`-style patch file** is refused outright. It would be a
  second description of a program's shape, unchecked against the first, which is the failure
  [ADR 0085](0085-openapi-is-generated-from-the-route-table.md) already refuses for API documents.
- **Anything requiring a network request.** The language server makes none — no registry lookup for a
  package name or version, no manifest metadata fetched while typing. A lockfile is on disk and may be
  read; a remote index may not be consulted. An editor that quietly talks to a third party while a
  developer types is the same class of surprise [ADR 0058](0058-outbound-request-policy.md) refuses for a
  running program, and the reviewed product does exactly this against a package registry.

### 3. A template region gets the editor's own services, and no second formatter

The extension forwards requests inside an inline-HTML region to VS Code's built-in HTML, CSS and
JavaScript language services, giving Emmet expansion, tag closing and renaming, the colour picker, hover
and validation in the half of a `.nvs` file that is markup.

- **The region list comes from the server**, as one request of Novis's own, `nvs/regions`, beside
  `nvs/redactions`. The lexer already knows where a mode ends; the client is not to re-derive it from a
  grammar, for the identical reason [ADR 0101](0101-secret-is-redacted-in-the-editor-and-the-range-comes-from-the-server.md) § 1
  gives for redaction ranges — the server knows, the client draws, and a client that guesses is a second
  implementation of the lexer.
- **This is not language logic in the client**, so [ADR 0016](0016-ide-integration.md) § 1 holds: the
  extension forwards a request to a service it did not write and holds no knowledge of HTML, CSS or Novis
  while doing it. The dependency allowlist test of
  [ADR 0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 6 is what keeps it that way.
- **Formatting is excluded, and this is the load-bearing half of the section.** The embedded services are
  not registered as formatting providers, and `editor.formatOnSave` in a `.nvs` file runs `nvs fmt` over
  the whole file and nothing else. Wiring VS Code's HTML formatter into the markup regions would put a
  second, configurable formatter inside a file whose formatter is unconfigurable by decision
  ([ADR 0039](0039-canonical-code-formatting.md)), and `nvs fmt --check` would then fail for a second
  reason, which § 9 of that ADR exists to prevent. `nvs fmt` treats an inline-HTML region as it treats any
  other span it does not reflow.
- **Off by one setting.** `nvs.template.services` (default `true`) disables the forwarding, because a user
  with their own HTML tooling has to be able to get out of the way of ours.

### 4. Three code actions that generate, bounded by what the type system already fixed

[ADR 0040](0040-vscode-deep-tooling-and-resilient-parsing.md) § 3's M10 list is entirely *fixes* — each
backed by a diagnostic. Three **generators** join it, each writing only what a declaration already
determines:

- **Implement missing members.** On a class that does not satisfy an interface or an abstract base, insert
  every missing method and property with the signature copied from the declaration, its `inout` markers
  ([0107](0107-by-reference-parameters-are-spelled-inout-at-both-ends.md)) and qualifiers intact, and a
  body that throws. This is the single most-used action in the product reviewed, and Novis's explicit types
  ([0007](0007-explicit-type-system.md)) make it mechanical where PHP's make it a guess.
- **Override a method.** A picker over the overridable members of the hierarchy, inserting the chosen
  signatures with a call to the base implementation where one exists.
- **Declare the function you just called.** On the diagnostic for an unresolved call, insert a declaration
  whose parameter types are the argument types at the call site and whose return type is the one the
  context requires — both already computed to produce the diagnostic.

**The bound is the rule, not the list**: a generator may write only text the type system has already fully
determined. It never invents a body, never names a parameter from a heuristic, and never picks between two
possible signatures. Anything needing a choice is a refactoring the user drives, not an action a light
bulb offers. **Getter/setter generation is refused under this bound and under
[ADR 0014](0014-property-observer.md)** — Novis has property hooks, so the pair of methods that action
exists to save typing has no reason to be written at all.

### 5. The debugger UI is only as deep as the adapter, so the adapter's capabilities are named here

[ADR 0040](0040-vscode-deep-tooling-and-resilient-parsing.md) § 3 wires `nvs dap` into VS Code's existing
debugger UI and stops. That is correct about the editor work and silent about the adapter, and the
distinction matters because **every item below renders in a UI that already exists and appears only if
`nvs dap` implements the corresponding capability**. Each is therefore M10 scope for `nvs dap`, not for the
extension:

- **Conditional breakpoints, hit counts and logpoints** — `supportsConditionalBreakpoints`,
  `supportsHitConditionalBreakpoints`, `supportsLogPoints`. A logpoint that does not stop the program is
  the debugging most users actually do.
- **Exception filters** — `exceptionBreakpointFilters`, so "break on uncaught" and "break on thrown" are
  separate switches. [ADR 0020](0020-error-escalation-ladder.md)'s single `Throwable` channel is what makes
  this two filters rather than the five categories PHP needs.
- **Stepping exclusions** — a `launch.json` glob list so stepping does not descend into package code, and
  a handled throw inside it does not stop the session.
- **Path mappings**, because the container case is the normal case: the file the adapter reports and the
  file in the editor differ whenever the program runs anywhere but the workspace root.
- **The value a function just returned**, surfaced in the variables pane after stepping out. It is one
  frame's worth of state the adapter has and the UI will render for free.
- **A `spawn`ed isolate is a DAP thread** ([0006](0006-isolated-script-execution.md)), which is the
  standard presentation and needs no protocol extension. The *tree* of isolates does — that stays in
  [ADR 0040](0040-vscode-deep-tooling-and-resilient-parsing.md) *Revisiting* where it already is.

### 6. `nvs check --json`, the check scope, and the identifiers all of this adds

- **`nvs check --json`** writes the same `Diagnostic` records the terminal renderer prints — code, spans,
  severity, help and `suggestions` — as one machine-readable document with a schema frozen the way
  [ADR 0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 7 freezes `nvs ast --json`'s. It is part
  of the CLI surface [ADR 0068](0068-dependency-currency-and-the-version-contract.md) versions. The text
  rendering stays the default and is what the Tasks `problemMatcher` reads; nothing about the terminal
  output changes. It exists for CI and for the agents that increasingly drive this compiler, including the
  one that maintains this repository.
- **`nvs.check.scope`** (`"open"` | `"workspace"`, default `"open"`) decides whether diagnostics are
  published for open documents and their `require`/`autoload` graph, or for every file the index holds.
  The default does not change what M4B does. `nvs.checkWorkspace` runs one workspace pass on demand
  without changing the setting, which is the cheap version of the same thing. **`nvs check` on the command
  line is untouched** — it has always analysed what it is given.
- **Frozen identifiers added** to
  [ADR 0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 6's roster, under that section's own rule
  that a name is added and never renamed: settings `nvs.check.scope`, `nvs.codeLens.enable`,
  `nvs.template.services`; command `nvs.checkWorkspace`; request `nvs/regions`.
- **`extensionKind: ["workspace"]`.** The extension spawns `nvs lsp`, which must be the binary next to the
  code — in a WSL distro, over SSH, or inside a devcontainer. Declaring it a workspace extension is one
  line in `package.json` and is the difference between working in every remote configuration and failing
  in all of them with a message about `nvs` not being on `PATH`.

## Consequences

**Positive**

- The five features of § 1 arrive as one piece of work rather than five, and the one of them that was
  simply missing — `references` — can no longer be discovered absent after rename ships against the same
  index.
- § 2 gives Novis the feature its competitor charges for, with a stronger guarantee: a route name completed
  from the compile-time route table is *correct*, where one recovered by scanning a conventions directory
  is a good guess. It also keeps `nvs-lsp` free of the per-framework modules that would otherwise arrive
  one framework at a time and never leave.
- § 3 closes the only place the review found Novis's plan plainly behind, and it closes it in the half of a
  `.nvs` file that [ADR 0082](0082-the-first-party-framework.md) makes the template engine — so the "view
  layer is the language" claim stops costing the user their HTML tooling.
- § 5 converts a one-line milestone commitment into a list that can be checked off, which is the difference
  between a debugger UI that opens and a debugger UI that works.

**Negative**

- **M10 grows again**, having already absorbed ADR 0040's deep half. Six additions, one of which (§ 3) is a
  request-forwarding layer with its own failure modes at region boundaries. The milestone's estimate is not
  assumed to be unchanged, for the second time.
- **§ 2's rule will be argued with.** The first user with a package whose route-like table is built at
  runtime will ask for completion the rule refuses, and the answer will be "make the table compile-time",
  which is a real cost to them and the correct answer for the language.
- **Workspace-scope diagnostics are the expensive setting**, and it is the one whose cost the plan has
  measured least. It is off by default for that reason, and § 1's dimming is silent under the default —
  a feature that appears only when a setting is changed is a feature some users will never find.
- **`nvs/regions` is a second request of Novis's own.** The protocol surface that ADR 0099 wanted to hold at
  one is now two, and each one is a thing a non-VS-Code client must implement to reach parity.

## Alternatives rejected

- **Build framework support the way the reviewed product does** — convention scans, an annotation dialect,
  a patch file for what neither reaches. Rejected under § 2: every one of those is a second description of
  a program's shape that nothing checks against the first, and Novis made the first description available on
  purpose.
- **Put a `Web`-framework module inside `nvs-lsp`.** Rejected: it would be the server's first special case,
  and the second would be a third-party package asking for the same treatment with no principle available
  to refuse it. § 2's rule is stated as a property of *where a value comes from* precisely so it answers
  both.
- **Wire the built-in HTML/CSS formatters into template regions.** Rejected under
  [ADR 0039](0039-canonical-code-formatting.md) § 9: two formatters in one file means `nvs fmt --check`
  fails for two reasons, and the second one is configurable.
- **Ship code snippets**, as the reviewed product does and as most language extensions do. Rejected: a
  snippet body is a second copy of a syntactic shape the grammar already owns, unchecked against it and
  silently stale after a grammar change. Keyword and member completion cover the same keystrokes with a
  description that cannot drift, and the shapes worth *teaching* live in `examples/` and in what `nvs new`
  produces ([0082](0082-the-first-party-framework.md)).
- **Package-name and version completion in the manifest, from a registry.** Rejected under § 2's last
  bullet: it needs a network request from the editor while the user types. A lockfile's resolved versions
  are on disk and may be shown; the index may not be queried.
- **Configurable diagnostic severity and suppression**, reached through `.editorconfig`, settings globs or
  in-source tags — the mature form of which the review documents in detail. Rejected: it is
  [ADR 0029](0029-identifier-casing-is-checked.md) § 3's closed door, and the review is evidence for that
  door rather than against it.
- **Getter/setter generation.** Rejected under § 4 — [ADR 0014](0014-property-observer.md)'s hooks mean the
  generated pair should not exist.

## Revisiting

- **`textDocument/callHierarchy`**, if the type hierarchy of § 1 proves to be the wrong half of the
  question in practice. It needs an edge index nothing else needs, which is why it is here and not in § 1.
- **An inferred-type probe** — an attribute or command that renders what the checker concluded for an
  arbitrary expression, the way a `@trace` annotation does in the reviewed product. Inlay hints cover
  assignments and returns ([0037](0037-var-local-type-inference.md)); the argument for a probe is the
  expression in the middle, and it is not strong enough yet to hold a name.
- **A toolchain picker in the status bar**, listing named `nvs` binaries the way `nvs.path` currently names
  one. Worth revisiting when [ADR 0068](0068-dependency-currency-and-the-version-contract.md)'s version
  contract starts putting two toolchains on one machine.
- **A wasm build of `nvs-lsp` for `vscode.dev`.** The server is synchronous with no async runtime
  ([0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) § 2) and answers every request from parse,
  resolve and typecheck without reaching codegen — so it is an ordinary Rust crate `rustc` can build for
  `wasm32-unknown-unknown`, and it never depended on [0025](0025-wasm-browser-target.md)'s retired Novis
  backend. The optionality is cheap to keep; nothing schedules it.
- **Marketplace publishing** stays exactly where [ADR 0016](0016-ide-integration.md) *Revisiting* left it —
  this ADR adds features, not a distribution channel.

## Verification

All at M10, in the order each becomes possible:

- A `.lspt` case per request added here — `references`, `documentHighlight`, `typeHierarchy` — resolves at
  a cursor and matches a frozen rendering, and
  [ADR 0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md)'s `every_request_answers_every_construct`
  matrix gains a row for each, so an unanswered construct fails by exit code rather than by inspection.
- **The five features of § 1 share one index**, checked structurally: `nvs-lsp` has exactly one symbol-index
  construction site, and `references`, `documentHighlight`, the CodeLens provider, `typeHierarchy` and the
  unused-member diagnostic all read it.
- **§ 2's rule is enforced by a test, not by review**: `nvs-lsp`'s completion sources are enumerated, and a
  test asserts each names a table the compiler builds for another reason. A completion source reading a
  directory layout, an annotation dialect or the network fails it.
- Route-name completion offers exactly the names
  [0102](0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md)'s table
  holds for a fixture application, and a directive completion in a `nvs.toml` offers exactly what
  [0064](0064-configuration-file-format.md)'s registry accepts — both asserted against the producing table
  rather than against a copied list.
- **The server makes no network request.** The `nvs-lsp` process is run against a fixture workspace with
  outbound sockets denied, and every request in the M4B and M10 sets answers.
- Inside an inline-HTML region, an Emmet expansion and a CSS colour decoration are produced, `nvs/regions`
  reports the region boundaries the lexer holds, and **the built-in formatters are not registered**:
  formatting a `.nvs` file with markup in it is byte-identical to `nvs fmt`, and `nvs fmt --check` passes
  on the result.
- Each generator of § 4 produces text that compiles: a class with the "implement missing members" action
  applied passes `nvs check` with only the inserted `throw`s reachable, and applying it twice is a no-op.
- `nvs dap` reports each capability of § 5 at `initialize`, and a fixture session exercises a conditional
  breakpoint, a logpoint, an exception filter, a stepping exclusion, a path mapping and a return value.
- `nvs check --json` over the corpus emits one record per diagnostic the text renderer prints, with the
  same codes and spans, and its schema is snapshot-tested the way `nvs ast --json`'s is.
- The extension declares `extensionKind: ["workspace"]`, asserted by the same contributions test that
  checks the frozen identifiers.
