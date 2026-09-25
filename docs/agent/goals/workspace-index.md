---
milestone: M10
---
# Loop goal 38 — The language server grows past its first closed list

M4B shipped a server that answers a cursor. This goal makes it answer a *program*: one workspace
symbol index, the five features that are each a query against it, the two navigations and the
signature popup that need no index at all, and the completion arms the current rules already admit
but nobody wrote. When it is green, `nvs-lsp` answers everything
`rule:ide/every-feature-is-staged-behind-its-dependency` does not gate behind `nvs fmt`, `nvs dap`,
the profiler or `nvs ext` — none of which exist, and none of which this goal builds.

It is the server half on purpose. Goal `editor-surfaces` is the editor half, and it reads this one: a Test Explorer
and an inline-HTML region are client work over a CLI, while everything here is one crate and one
protocol. Splitting them is what keeps a stalled index from holding up a `problemMatcher`.

Goal `editor-install`'s whole acceptance list is this goal's floor, and it is never traded.

## Why here

`nvs fmt`, `nvs dap`, the profiler and `nvs ext` do not exist, so format-on-save, the debugger, the
profile view and the extension commands are not here and are not stubbed —
`rule:ide/every-feature-is-staged-behind-its-dependency` is the whole reason the milestone splits
this way rather than waiting for all four.

## Stage 0 — the catch-up

Two defects that are live in a shipped extension today, and both are in this crate.

1. **`\` triggers a list this server cannot answer.**
   `crates/nvs-lsp/src/capabilities.rs:174` declares `-`, `>`, `:` and `\` as trigger characters
   while the comment above it says *"the three positions where a member list appears"*. There is no
   namespace arm, so typing `\` in `echo Core\` pops the keyword list —
   `abstract`, `autoload`, `break`, `class`. Stage 5 gives `\` its arm; until that lands the
   character promises what nothing delivers.
2. **A member list dies at the end of a block.** `crates/nvs-lsp/src/completion.rs`'s resilient
   parse finds a receiver when a statement follows the cursor and not when a `}` does:
   `$b->` above `var $n = 1;` answers `area`, `grow`, `size`, and the same `$b->` above `}` answers
   37 keywords. The `.lspt` corpus froze the first case — `crates/nvs-lsp/src/lib.rs:57`'s own
   example is *member completion survives an unclosed brace* — and never the second, which is why
   both halves were green. A case per closing delimiter is this stage's deliverable.

## Stage 1 — the floor

Goal `editor-install`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded for anything above it.

## Stage 2 — the keystone: one workspace symbol index

`rule:ide/five-features-are-one-reference-index` is the whole specification and it is already
written: **one construction site, five readers, and a structural test that says so.** Everything in
stage 3 is a query, so the index goes first and it is provable with no editor in the room.

1. **The declaration side.** Every name every file in the workspace declares, with its declaring
   span — the same set `Analysed::module` already resolves for one document's `require`/`autoload`
   graph, widened from that graph to the tree `nvs.check.scope` selects.
2. **The occurrence side.** Resolution applied to *every* occurrence rather than to the one under a
   cursor, which is the walk [ADR 0099](../../decisions/0099.md) § 3 named as the reason
   `documentHighlight` was not admitted at M4B. It is one walk producing both sides, not two.
3. **Invalidation.** A `didChange` invalidates the file that changed and the files whose resolution
   read it, and nothing else. `rule:ide/a-full-reanalysis-stays-under-a-bound` is the bound this
   must still hold when the index is warm.
4. **The scope.** `nvs.check.scope`'s `"open"` and `"workspace"` per
   `rule:ide/check-scope-defaults-to-the-workspace`, default `"open"`, plus `nvs.checkWorkspace`
   for one pass on demand. Both identifiers are already frozen in that rule's roster and are added,
   never renamed.

## Stage 3 — the five readers, and only five

Each is one query against stage 2 and none gets a walk of its own; the structural test is that
`nvs-lsp` has exactly one index construction site and these five read it.

1. **`textDocument/references`** — the index's read side.
2. **`textDocument/documentHighlight`** — the same query narrowed to the open file.
3. **CodeLens** — reference count, a type's implementors, and the methods a method overrides and is
   overridden by, behind `nvs.codeLens.enable` (default `true`), because a lens is a request per
   visible declaration.
4. **`textDocument/typeHierarchy`** — supertypes and subtypes, including
   `rule:classes/interface-default-methods` and `rule:classes/delegation-by-field`.
5. **Unused-member dimming** — a private member, constant or `use` with no reference anywhere in the
   index, carrying LSP's `Unnecessary` tag. Correct only at workspace scope, so it is **silent**
   under the default rather than wrong.

Call hierarchy is deliberately not here: it is a different index — call-site edges kept
incrementally — and nothing else reads it, so it is not built.

## Stage 4 — the three that need no index

1. **`textDocument/signatureHelp`** — the enclosing call from `SyntaxIndex.at`, the active parameter
   from the cursor's position among the arguments, and the signature itself from the row hover
   already renders: `nvs_stdlib::registry` for a `Core` member, the declared parameter list for a
   user method. No new analysis.
2. **`textDocument/typeDefinition`** and **`textDocument/implementation`** — the resolution
   `definition` already performs, pointed at the type a value holds and at the implementors of an
   interface rather than at the name under the cursor.
3. **`textDocument/declaration`** is *not* added. In Novis a declaration and a definition are the
   same span, so a second request answering identically is a request set that grew for nothing.

## Stage 5 — the completion arms the rules already admit

`rule:ide/completion-offers-only-what-the-compiler-derived` is the gate every arm here passes: each
offers a value only where the compiler already derives it for another reason, and reaches it through
the same table that reason uses.

1. **A namespace segment.** `Core\` offers the classes `nvs_stdlib::registry` holds and the
   namespace's own declarations from stage 2's index — the same registry `Core\Str::` already reads
   for members. This is what stage 0's trigger character has been promising.
2. **A bare class name in scope.** The imports in force plus the declarations the index holds, which
   is what makes `Att` reach `Attack`. It is admitted here and not at M4B for one reason: at M4B it
   would have been a workspace symbol search with no index under it.
3. **The PHP-name layer**, per `rule:php-migration/every-php-builtin-is-a-completion-candidate`:
   every PHP built-in in the differential oracle's own inventory is a candidate, answered from
   [docs/spec/02-php-migration.md](../../spec/02-php-migration.md)'s row for that name and gated on
   `nvs-stdlib`'s registry so an item inserts a member only where the compiler can resolve it. The
   other three shapes — dropped, unclassified, and destination-not-built — appear and insert
   nothing, per `rule:ide/three-of-four-item-shapes-insert-nothing`. One **build-time** join of
   those three sources generates the table and fails the build on a destination spelling that
   matches nothing. `nvs.completion.phpNames` (`all`/`resolved`/`off`, default `all`) turns it off.

## Stage 6 — inlay hints

The inferred type after a `var` declaration whose annotation is absent, and a parameter name at a
call site where the argument is a bare literal. Read straight off `Analysed::exprs`, which already
holds what the type phase recorded.

This stage **reverses a deferral rather than filling a gap**, and that is why it is last and why it
opens a record of its own: [ADR 0099](../../decisions/0099.md) § 3 held inlay hints back because
they *"want a settings story and encode idioms that are still moving through M5–M9"*. M8 and M9 have
not landed, so the reversal is bounded to the two shapes above — the two whose idiom is settled —
and the settings story is one enum, not a matrix.

## Standing decisions

- **The index is workspace-scoped from the first slice.** A graph-scoped index that four features
  read and the fifth cannot was considered and rejected: it makes `nvs.check.scope` a property of
  the index rather than of the query, and unused-member dimming — the one reader that is only
  correct at workspace scope — would have had to wait for a second construction site. One index,
  one construction site, the scope chosen per query.
- **Five readers, and a sixth is a decision.** `rule:ide/five-features-are-one-reference-index`
  names exactly five and excludes call hierarchy with a reason. A session that finds a sixth query
  useful writes it down in the handoff; it does not add it.
- **No feature here is stubbed ahead of its dependency.** `nvs fmt`, `nvs dap`, the profiler and
  `nvs ext` do not exist, so `textDocument/formatting`, the debugger, the profile view and the
  extension commands are not in this goal in any form — not behind a flag, not returning empty.
  `rule:ide/every-feature-is-staged-behind-its-dependency` is the reason and it is not reopened.
- **`nvs.toml` directive completion and route-name completion are out.** The first needs the
  extension to claim `.toml`, which collides with `rule:ide/the-extension-claims-nvs-only`; the
  second reads a route table that M7 is still moving. Both are a later goal's, after their
  respective questions are settled — not this one's under an assumption.
- **This goal opens one ADR**, covering stage 2's index shape, stage 4's request admissions against
  [ADR 0099](../../decisions/0099.md) § 3's test, stage 5's namespace and bare-name arms, and stage
  6's reversal of that same record's inlay-hint deferral. Its `changes:` block amends
  `rule:ide/the-request-set-is-closed` to name M10's additions beside M4B's nine.
- **Where ambiguity resolves:** a request whose data the index does not hold answers empty rather
  than erroring, exactly as `completion` already does for an unresolved receiver — LSP has no shape
  for "ask me again somewhere else".
