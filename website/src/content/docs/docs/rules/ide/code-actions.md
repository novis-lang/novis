---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Code actions and quick fixes"
description: "An action may only write text the compiler already determined — never a choice, never an alias, never a guess."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/ide/the-language-server/
  label: "The language server"
next:
  link: /docs/rules/ide/highlighting-and-completion/
  label: "Highlighting, completion and references"
---

<p class="nv-section-lead">An action may only write text the compiler already determined — never a choice, never an alias, never a guess.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">9</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">0</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">9</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">5</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#a-code-action-ships-only-a-fix-a-diagnostic-already-knows">A code action ships only where its replacement text is already in a diagnostic's suggestions — two at M4B, and no other</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#a-quick-fix-is-a-diagnostics-own-suggestion">An inspection is a code action backed by a diagnostic the checker emits, runs on the resilient tree, and is off by default under <code>source.fixAll.nvs</code> so it composes with format-on-save while <code>nvs fmt</code> stays layout-only</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-code-action-writes-only-what-is-already-determined">A code action that writes may write only text the type system has already fully determined, and never makes a choice</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#no-refactoring-introduces-an-alias">Rename, organize-imports and auto-import never write a <code>use</code> alias: an import inserts the fully-qualified name, and organize-imports only reorders and removes</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#narrow-an-annotation-to-its-literal">One editor action rewrites an <code>array&lt;mixed&gt;</code> annotation to the type of the array literal that initializes it, and rewrites nothing else</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#narrowest-means-narrowest-base-type">The synthesized element type is the canonical union of the elements' base types, recursed through nested literals and bounded at depth 32</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#the-action-answers-from-the-literal-or-not-at-all">The narrowing action answers from a literal in the same file, or it is not offered: no literal, a <code>mixed</code> element, or a no-op each hides it</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#no-compile-path-calls-the-synthesis">The synthesis is one function in <code>nvs-types</code> that no compile path calls, so an array literal stays checked against a target and <code>var</code> still refuses a bare one</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#narrowing-is-a-diff-never-a-save-time-fix">The narrowing action is never registered under <code>source.fixAll.nvs</code> and never chases the call sites it affects, because it breaks writes it cannot see</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li></ol>

<div class="nv-rule" id="a-code-action-ships-only-a-fix-a-diagnostic-already-knows">

## A code action ships only where its replacement text is already in a diagnostic's suggestions — two at M4B, and no other

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#a-code-action-ships-only-a-fix-a-diagnostic-already-knows"><code>ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows</code></a>
</div>

M4B ships two code actions and only two: the casing fix ([`core-api/identifier-casing`](/docs/rules/core-api/naming-and-shape/#identifier-casing "An identifier's casing is fixed by its category and a mismatch is a hard compile error")) and the
legacy-cast fix `(int)$x` → `$x as int` ([`types/no-legacy-cast`](/docs/rules/types/unions-and-conversion/#no-legacy-cast "PHP's (T)expr cast does not parse, and the diagnostic names the as that replaces it")). They are admitted for one reason,
and it is not that they are useful: their replacement text is already computed, sitting in the
`Diagnostic::suggestions` field `nvs-diagnostics` has carried since M0. The provider is a translation from
`Suggestion` to `CodeAction` — a dozen lines and no new analysis.

The boundary is exactly that. A quick fix whose replacement a diagnostic already knows may ship; one that
would need the checker to compute something new is M10's. Both are registered under `source.fixAll.nvs`
so `editor.codeActionsOnSave` composes them with format-on-save when that arrives.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/naming-and-shape/#identifier-casing" title="An identifier's casing is fixed by its category and a mismatch is a hard compile error"><code>core-api/identifier-casing</code></a> <a href="/docs/rules/types/unions-and-conversion/#no-legacy-cast" title="PHP's (T)expr cast does not parse, and the diagnostic names the as that replaces it"><code>types/no-legacy-cast</code></a> <a href="/docs/rules/ide/the-language-server/#the-request-set-is-closed" title="M4B answers nine standard requests and exactly one of Novis's own, M10 adds eight more on the same test, and that test keeps the list from growing"><code>ide/the-request-set-is-closed</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-is-never-a-diagnostic" title="nvs fmt is a separate opt-in tool: no compiler command runs it, an unformatted file is never a diagnostic, and an editor composes it with quick fixes in the client"><code>tooling/fmt-is-never-a-diagnostic</code></a> <a href="/docs/rules/ide/code-actions/#narrow-an-annotation-to-its-literal" title="One editor action rewrites an array&lt;mixed&gt; annotation to the type of the array literal that initializes it, and rewrites nothing else"><code>ide/narrow-an-annotation-to-its-literal</code></a> <a href="/docs/rules/ide/the-language-server/#every-feature-is-staged-behind-its-dependency" title="The VS Code client goes as deep as the editor allows, and each feature waits for the language or runtime piece it needs rather than shipping as a stub"><code>ide/every-feature-is-staged-behind-its-dependency</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a></dd></div></dl>

</div>

<div class="nv-rule" id="a-quick-fix-is-a-diagnostics-own-suggestion">

## An inspection is a code action backed by a diagnostic the checker emits, runs on the resilient tree, and is off by default under `source.fixAll.nvs` so it composes with format-on-save while `nvs fmt` stays layout-only

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-quick-fix-is-a-diagnostics-own-suggestion"><code>ide/a-quick-fix-is-a-diagnostics-own-suggestion</code></a>
</div>

An inspection is an LSP code action, and every one is backed by a diagnostic the checker emits: a casing
violation offers the `camelCase`/`PascalCase` rename ([`core-api/identifier-casing`](/docs/rules/core-api/naming-and-shape/#identifier-casing "An identifier's casing is fixed by its category and a mismatch is a hard compile error")); a legacy
`(int)$x` offers `$x as int` ([`types/no-legacy-cast`](/docs/rules/types/unions-and-conversion/#no-legacy-cast "PHP's (T)expr cast does not parse, and the diagnostic names the as that replaces it")); a missing constructor property assignment
offers to add it ([`classes/definite-property-initialization`](/docs/rules/classes/declaring-a-class/#definite-property-initialization "Every property a class declares is definitely assigned on every path out of every constructor")); `include` and `require_once` offer
`require` ([`statements/require-is-the-only-inclusion-construct`](/docs/rules/statements/names-and-require/#require-is-the-only-inclusion-construct "require is the only file-inclusion construct, and it runs every time it is reached")); a `tainted` or `secret` value at
a refusing sink offers the laundering call the diagnostic already names
([`security/tainted-qualifier`](/docs/rules/security/tainted-data/#tainted-qualifier "tainted is a compile-time qualifier on string, bytes and a shape of them, spellable in any declaration and erased before codegen")), a mis-ordered `tainted secret string` included; and a `#[Route]`
missing its `path` offers the derived one ([`routing/a-quick-fix-writes-a-derived-path`](/docs/rules/routing/links-and-the-api-document/#a-quick-fix-writes-a-derived-path "A #[Route] missing its path gets a quick fix that offers a derived one, and nothing derives a path on its own")). Each is a
suggestion the developer applies deliberately.

They run against the resilient tree ([`ide/the-tree-survives-a-syntax-error`](/docs/rules/ide/the-resilient-parse/#the-tree-survives-a-syntax-error "The parser always returns a tree, a node it invented says so, and an offset maps to the innermost node even inside a malformed region")), not a successful
parse: a mis-ordered qualifier, a legacy cast and a `var $x` property are parse or declaration errors,
so a fix that fired only on a clean parse would never fire on the file that needs it.

They are off by default and composable with format-on-save. The extension registers them under
`source.fixAll.nvs`, which VS Code runs through `editor.codeActionsOnSave` independently of
`editor.formatOnSave`, so a developer who opts in gets layout and fixes on one keystroke while `nvs fmt`
itself stays layout-only and `nvs fmt --check` fails for exactly one reason. PhpStorm's Reformat Code
dialog, with its per-action checkboxes, is the same composition through a different client.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no inspection profile to tune; a fix exists only where a diagnostic already carries its replacement text, and running fixes on save is <code>editor.codeActionsOnSave</code>, never part of the formatter</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/the-resilient-parse/#the-tree-survives-a-syntax-error" title="The parser always returns a tree, a node it invented says so, and an offset maps to the innermost node even inside a malformed region"><code>ide/the-tree-survives-a-syntax-error</code></a> <a href="/docs/rules/ide/the-language-server/#the-first-server-answers-a-closed-list" title="The first nvs lsp answers six requests and two code actions, and nothing else until the workspace index exists"><code>ide/the-first-server-answers-a-closed-list</code></a> <a href="/docs/rules/routing/links-and-the-api-document/#a-quick-fix-writes-a-derived-path" title="A #[Route] missing its path gets a quick fix that offers a derived one, and nothing derives a path on its own"><code>routing/a-quick-fix-writes-a-derived-path</code></a> <a href="/docs/rules/core-api/naming-and-shape/#identifier-casing" title="An identifier's casing is fixed by its category and a mismatch is a hard compile error"><code>core-api/identifier-casing</code></a> <a href="/docs/rules/types/unions-and-conversion/#no-legacy-cast" title="PHP's (T)expr cast does not parse, and the diagnostic names the as that replaces it"><code>types/no-legacy-cast</code></a> <a href="/docs/rules/classes/declaring-a-class/#definite-property-initialization" title="Every property a class declares is definitely assigned on every path out of every constructor"><code>classes/definite-property-initialization</code></a> <a href="/docs/rules/statements/names-and-require/#require-is-the-only-inclusion-construct" title="require is the only file-inclusion construct, and it runs every time it is reached"><code>statements/require-is-the-only-inclusion-construct</code></a> <a href="/docs/rules/security/tainted-data/#tainted-qualifier" title="tainted is a compile-time qualifier on string, bytes and a shape of them, spellable in any declaration and erased before codegen"><code>security/tainted-qualifier</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-is-never-a-diagnostic" title="nvs fmt is a separate opt-in tool: no compiler command runs it, an unformatted file is never a diagnostic, and an editor composes it with quick fixes in the client"><code>tooling/fmt-is-never-a-diagnostic</code></a> <a href="/docs/rules/ide/the-language-server/#the-request-set-is-closed" title="M4B answers nine standard requests and exactly one of Novis's own, M10 adds eight more on the same test, and that test keeps the list from growing"><code>ide/the-request-set-is-closed</code></a> <a href="/docs/rules/ide/code-actions/#a-code-action-writes-only-what-is-already-determined" title="A code action that writes may write only text the type system has already fully determined, and never makes a choice"><code>ide/a-code-action-writes-only-what-is-already-determined</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0039.md">record 0039</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a></dd></div></dl>

</div>

<div class="nv-rule" id="a-code-action-writes-only-what-is-already-determined">

## A code action that writes may write only text the type system has already fully determined, and never makes a choice

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-code-action-writes-only-what-is-already-determined"><code>ide/a-code-action-writes-only-what-is-already-determined</code></a>
</div>

Four code actions **write** rather than fix, and one bound governs all of them: an action may write only
text the type system has already fully determined. It never invents a body, never names a parameter from a
heuristic, and never picks between two possible signatures. Anything needing a choice is a refactoring
the user drives, not an action a light bulb offers.

The four: **implement missing members** — on a class that does not satisfy an interface or abstract
base, insert every missing method and property with the signature copied from the declaration,
[`statements/inout-is-the-by-reference-spelling`](/docs/rules/statements/by-reference-and-exit/#inout-is-the-by-reference-spelling "A by-reference binding is written inout, in the modifier slot before the type") markers and qualifiers intact, and a body that
throws; **override a method** — a picker over the hierarchy's overridable members, inserting the chosen
signatures with a call to the base where one exists; **declare the function you just called** — on the
unresolved-call diagnostic, insert a declaration whose parameter types are the argument types at the call
and whose return type is the one the context requires, both already computed to produce the diagnostic;
and **narrow an `array<mixed>` to its literal's type**
([`ide/narrow-an-annotation-to-its-literal`](/docs/rules/ide/code-actions/#narrow-an-annotation-to-its-literal "One editor action rewrites an array<mixed> annotation to the type of the array literal that initializes it, and rewrites nothing else")), which writes an annotation rather than a declaration
under the same bound.

Applying a generator produces text that compiles, and applying it twice is a no-op.

**Getter/setter generation is refused** under this bound and under [`classes/property-observer`](/docs/rules/classes/properties/#property-observer "A class observes every read and write of its own properties by implementing PropertyObserver, never by declaring __get/__set"):
property hooks mean the pair of methods that action exists to save typing has no reason to be written.
Code snippets are refused for a neighbouring reason — a snippet body is a second copy of a syntactic shape
the grammar already owns, silently stale after a grammar change.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Implement-missing-members copies real signatures instead of guessing at them, and there is no getter/setter generator because property hooks make the pair pointless</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/code-actions/#narrowing-is-a-diff-never-a-save-time-fix" title="The narrowing action is never registered under source.fixAll.nvs and never chases the call sites it affects, because it breaks writes it cannot see"><code>ide/narrowing-is-a-diff-never-a-save-time-fix</code></a> <a href="/docs/rules/classes/properties/#property-observer" title="A class observes every read and write of its own properties by implementing PropertyObserver, never by declaring __get/__set"><code>classes/property-observer</code></a> <a href="/docs/rules/statements/by-reference-and-exit/#inout-is-the-by-reference-spelling" title="A by-reference binding is written inout, in the modifier slot before the type"><code>statements/inout-is-the-by-reference-spelling</code></a> <a href="/docs/rules/routing/links-and-the-api-document/#a-quick-fix-writes-a-derived-path" title="A #[Route] missing its path gets a quick fix that offers a derived one, and nothing derives a path on its own"><code>routing/a-quick-fix-writes-a-derived-path</code></a> <a href="/docs/rules/ide/the-language-server/#every-feature-is-staged-behind-its-dependency" title="The VS Code client goes as deep as the editor allows, and each feature waits for the language or runtime piece it needs rather than shipping as a stub"><code>ide/every-feature-is-staged-behind-its-dependency</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0108.md">record 0108</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0114.md">record 0114</a></dd></div></dl>

</div>

<div class="nv-rule" id="no-refactoring-introduces-an-alias">

## Rename, organize-imports and auto-import never write a `use` alias: an import inserts the fully-qualified name, and organize-imports only reorders and removes

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#no-refactoring-introduces-an-alias"><code>ide/no-refactoring-introduces-an-alias</code></a>
</div>

The refactorings are LSP requests — workspace-wide rename, extract to method or variable, and
organize-imports — and the deeper completion is signature help, cross-file symbol search, auto-import,
and inlay hints for `var`-inferred types ([`types/var-inference`](/docs/rules/types/declarations-and-numbers/#var-inference "var takes a local's type from its initializer and fixes it there for good")) and call-site parameter names.

None of them writes an alias. Organize-imports is restricted to reordering and removing unused `use`
statements; auto-import inserts the correct fully-qualified name and nothing else. An editor that
resolved a clash with `use Foo as Bar` would be inventing a spelling the language does not have
([`statements/nothing-gets-a-second-name`](/docs/rules/statements/names-and-require/#nothing-gets-a-second-name "A declaration is reachable under exactly the name it was declared with")), and a second name introduced by tooling is exactly as
much a second name as one typed by hand.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PhpStorm's auto-import may add <code>use Foo as Bar</code> to dodge a clash; here an alias is not a spelling the language has, so the editor cannot write one</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/statements/names-and-require/#nothing-gets-a-second-name" title="A declaration is reachable under exactly the name it was declared with"><code>statements/nothing-gets-a-second-name</code></a> <a href="/docs/rules/types/declarations-and-numbers/#var-inference" title="var takes a local's type from its initializer and fixes it there for good"><code>types/var-inference</code></a> <a href="/docs/rules/ide/the-language-server/#every-feature-is-staged-behind-its-dependency" title="The VS Code client goes as deep as the editor allows, and each feature waits for the language or runtime piece it needs rather than shipping as a stub"><code>ide/every-feature-is-staged-behind-its-dependency</code></a> <a href="/docs/rules/ide/highlighting-and-completion/#five-features-are-one-reference-index" title="Find-references, occurrence highlight, CodeLens, type hierarchy and unused-member dimming are five queries against one workspace index"><code>ide/five-features-are-one-reference-index</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a></dd></div></dl>

</div>

<div class="nv-rule" id="narrow-an-annotation-to-its-literal">

## One editor action rewrites an `array<mixed>` annotation to the type of the array literal that initializes it, and rewrites nothing else

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#narrow-an-annotation-to-its-literal"><code>ide/narrow-an-annotation-to-its-literal</code></a>
</div>

A deep nested literal gets annotated `array<mixed>` because writing `array<array<array<float>>>` by hand
is tedious, and the annotation then costs the program the typed path for the rest of its life: every
operation on a `mixed` resolves through the generic helper path, every element write becomes a runtime
check, and every read out of the structure needs an `as` at the far end. The type is derivable — the
checker already walks every element — so one editor action derives it.

The action is offered on the **type annotation** of a declaration whose initializer is an **array literal
in the same file**. It rewrites that annotation and nothing else — not the literal, not a use site, not
another line. It is invoked by the developer from the annotation; there is no diagnostic and no hint
behind it, because nothing is wrong, and discoverability rides on the annotation's own light bulb. It
never changes a value's kind: a union in the offer such as `array<string|int|array<string>>` is a signal
that the value is a record and wants an object shape, but converting the literal is not this action's
business, because it would turn copy-on-write value semantics into shared-reference ones.

What it writes is decided by [`ide/narrowest-means-narrowest-base-type`](/docs/rules/ide/code-actions/#narrowest-means-narrowest-base-type "The synthesized element type is the canonical union of the elements' base types, recursed through nested literals and bounded at depth 32"); when it declines, by
[`ide/the-action-answers-from-the-literal-or-not-at-all`](/docs/rules/ide/code-actions/#the-action-answers-from-the-literal-or-not-at-all "The narrowing action answers from a literal in the same file, or it is not offered: no literal, a mixed element, or a no-op each hides it"); and why it never runs on save, by
[`ide/narrowing-is-a-diff-never-a-save-time-fix`](/docs/rules/ide/code-actions/#narrowing-is-a-diff-never-a-save-time-fix "The narrowing action is never registered under source.fixAll.nvs and never chases the call sites it affects, because it breaks writes it cannot see"). It is a code action whose fix the checker has to
compute, which places it under [`ide/a-code-action-writes-only-what-is-already-determined`](/docs/rules/ide/code-actions/#a-code-action-writes-only-what-is-already-determined "A code action that writes may write only text the type system has already fully determined, and never makes a choice") and on
the far side of the boundary the first editor slice draws.

A round trip asserts it: offered on a literal-initialized annotation, absent on a `Core\Json::decode`
initializer, and the file it produces still checks clean.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>The language never infers an array's type from its literal; the narrowing is an editor edit a person reads in a diff and approves</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/code-actions/#narrowest-means-narrowest-base-type" title="The synthesized element type is the canonical union of the elements' base types, recursed through nested literals and bounded at depth 32"><code>ide/narrowest-means-narrowest-base-type</code></a> <a href="/docs/rules/ide/code-actions/#the-action-answers-from-the-literal-or-not-at-all" title="The narrowing action answers from a literal in the same file, or it is not offered: no literal, a mixed element, or a no-op each hides it"><code>ide/the-action-answers-from-the-literal-or-not-at-all</code></a> <a href="/docs/rules/ide/code-actions/#narrowing-is-a-diff-never-a-save-time-fix" title="The narrowing action is never registered under source.fixAll.nvs and never chases the call sites it affects, because it breaks writes it cannot see"><code>ide/narrowing-is-a-diff-never-a-save-time-fix</code></a> <a href="/docs/rules/ide/code-actions/#a-code-action-writes-only-what-is-already-determined" title="A code action that writes may write only text the type system has already fully determined, and never makes a choice"><code>ide/a-code-action-writes-only-what-is-already-determined</code></a> <a href="/docs/rules/types/arrays-and-property-keys/#arrays" title="An array is PHP's ordered hash with string keys and a declared element type"><code>types/arrays</code></a> <a href="/docs/rules/types/declarations-and-numbers/#var-inference" title="var takes a local's type from its initializer and fixes it there for good"><code>types/var-inference</code></a> <a href="/docs/rules/ide/the-resilient-parse/#one-grammar-one-tree" title="The resilient parse is the one grammar's AST plus a trivia layer and an offset index, never a second tree"><code>ide/one-grammar-one-tree</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0114.md">record 0114</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a></dd></div></dl>

</div>

<div class="nv-rule" id="narrowest-means-narrowest-base-type">

## The synthesized element type is the canonical union of the elements' base types, recursed through nested literals and bounded at depth 32

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#narrowest-means-narrowest-base-type"><code>ide/narrowest-means-narrowest-base-type</code></a>
</div>

The synthesized element type is the canonical union of the element types, **each widened to its base**:
the same widening [`types/literal-types`](/docs/rules/types/text-and-literal-types/#literal-types "A string or int literal is its own type, and a union of them is a closed set") already performs at a placement, and the same union
canonicalisation the type table uses everywhere. So `['retry' => 1, 'depth' => 3]` yields `array<int>`
and never `array<1|3>`, which would be narrower and would refuse the next write made to it. An enum case
widens to its enum by the same call.

- A nested literal recurses, which is where the whole value of the action is: `array<array<array<float>>>`
  from one keystroke.
- `[]` already has type `array<never>`, which satisfies every `array<T>`, so there is nothing to offer and
  the action does not appear.
- The walk inherits the depth-32 descriptor bound of [`types/arrays`](/docs/rules/types/arrays-and-property-keys/#arrays "An array is PHP's ordered hash with string keys and a declared element type"): past it the action declines
  rather than proposing a type the checker would then refuse.
- The action offers only a type that renders back to a spelling a developer could have written, because
  the edit it makes is source text a human reads in a diff.

The unit tests are the list: a homogeneous nest at three levels, a heterogeneous literal producing a
canonical union, an integer literal widening to `int`, an enum case widening to its enum, `[]` yielding no
offer, and a literal past depth 32 yielding no offer.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/code-actions/#narrow-an-annotation-to-its-literal" title="One editor action rewrites an array&lt;mixed&gt; annotation to the type of the array literal that initializes it, and rewrites nothing else"><code>ide/narrow-an-annotation-to-its-literal</code></a> <a href="/docs/rules/types/text-and-literal-types/#literal-types" title="A string or int literal is its own type, and a union of them is a closed set"><code>types/literal-types</code></a> <a href="/docs/rules/types/text-and-literal-types/#enum-case-type" title="An enum case used as a type is a narrowed subtype of its enum, never its backing integer"><code>types/enum-case-type</code></a> <a href="/docs/rules/types/unions-and-conversion/#unions-and-mixed" title="A union permits only what every member permits, and mixed is the one position checked nowhere"><code>types/unions-and-mixed</code></a> <a href="/docs/rules/types/arrays-and-property-keys/#arrays" title="An array is PHP's ordered hash with string keys and a declared element type"><code>types/arrays</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0114.md">record 0114</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0047.md">record 0047</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-action-answers-from-the-literal-or-not-at-all">

## The narrowing action answers from a literal in the same file, or it is not offered: no literal, a `mixed` element, or a no-op each hides it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#the-action-answers-from-the-literal-or-not-at-all"><code>ide/the-action-answers-from-the-literal-or-not-at-all</code></a>
</div>

Three refusals, and each is a soundness question rather than a difficulty one.

**No literal, no action.** `array<mixed> $body = Core\Json::decode($raw);` gets nothing. Neither does a
`Core\Request::query()`, a `Core\Script::args()` or any other value that arrives from outside — those are
`array<mixed>` on purpose ([`types/unions-and-mixed`](/docs/rules/types/unions-and-conversion/#unions-and-mixed "A union permits only what every member permits, and mixed is the one position checked nowhere"), [`statements/no-host-populated-variables`](/docs/rules/statements/where-state-lives/#no-host-populated-variables "No variable is ever populated by the host; every superglobal is a Core class member")).
The shape of one payload someone looked at does not bound the next request's, and narrowing there would
convert a runtime check into a compile-time assumption that is not true. This is priority 1 and 2, not
ergonomics, and a developer may not point the action at such a value on the grounds that they know their
data.

**A `mixed` element poisons the answer, and a no-op is not offered.** An element whose own type is
`mixed` — including a spread of one — makes the union `mixed`, so the synthesis yields `array<mixed>` and
the action stays hidden rather than proposing the annotation that is already written.

**Everything else is answerable**, and that is a property of the language rather than of the
implementation: because every binding site declares a type, an element that is a variable or a call has a
type the checker already holds. There is no PHP-style guess anywhere in this action.

Narrowing from writes made *after* the declaration is not offered: it needs the join over every write
reaching the declaration and a generator that picks between answers, which
[`ide/a-code-action-writes-only-what-is-already-determined`](/docs/rules/ide/code-actions/#a-code-action-writes-only-what-is-already-determined "A code action that writes may write only text the type system has already fully determined, and never makes a choice") refuses.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A value decoded from input stays <code>array&lt;mixed&gt;</code> on purpose and gets no offer, where a PHP editor would happily type it from one observed payload</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/code-actions/#narrow-an-annotation-to-its-literal" title="One editor action rewrites an array&lt;mixed&gt; annotation to the type of the array literal that initializes it, and rewrites nothing else"><code>ide/narrow-an-annotation-to-its-literal</code></a> <a href="/docs/rules/types/unions-and-conversion/#unions-and-mixed" title="A union permits only what every member permits, and mixed is the one position checked nowhere"><code>types/unions-and-mixed</code></a> <a href="/docs/rules/statements/where-state-lives/#no-host-populated-variables" title="No variable is ever populated by the host; every superglobal is a Core class member"><code>statements/no-host-populated-variables</code></a> <a href="/docs/rules/security/tainted-data/#tainted-sources" title="Every accessor that hands a program bytes from outside answers the tainted form, and the list of them is enumerable"><code>security/tainted-sources</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0114.md">record 0114</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a></dd></div></dl>

</div>

<div class="nv-rule" id="no-compile-path-calls-the-synthesis">

## The synthesis is one function in `nvs-types` that no compile path calls, so an array literal stays checked against a target and `var` still refuses a bare one

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#no-compile-path-calls-the-synthesis"><code>ide/no-compile-path-calls-the-synthesis</code></a>
</div>

The synthesis is a single function in `nvs-types` — a sibling of `check_array_literal`, not a change to
it — returning the type an array literal would have if it were synthesized bottom-up, or nothing where
[`ide/the-action-answers-from-the-literal-or-not-at-all`](/docs/rules/ide/code-actions/#the-action-answers-from-the-literal-or-not-at-all "The narrowing action answers from a literal in the same file, or it is not offered: no literal, a mixed element, or a no-op each hides it") declines. It lives in the checker's crate
rather than the editor's because putting it beside the editor would mean a second implementation of union
canonicalisation and literal widening outside the type system that owns both, and the two would drift.
The type table is the one home for what a type is; the editor asks it a question.

**No compile path calls it, and that is the more important half.** `check_array_literal` still returns
`array<mixed>` where it has no expectation. That single line is what keeps a literal checked against a
target rather than inferred ([`types/arrays`](/docs/rules/types/arrays-and-property-keys/#arrays "An array is PHP's ordered hash with string keys and a declared element type")), keeps `var $x = [1, 2];` refused
([`types/var-inference`](/docs/rules/types/declarations-and-numbers/#var-inference "var takes a local's type from its initializer and fixes it there for good")), and keeps every no-expectation position — an `echo` argument, a bare expression
statement — typing exactly as it does today. There is no new diagnostic, no new code, nothing in any
crate but `nvs-types`, no change to the runtime array descriptor and nothing per request. A guard test
holds that no compile path reaches the function, so "this changes no language behaviour" is checkable
rather than argued.

`var` still refusing a bare array literal is not a leftover. It refuses because of where the answer ends
up, not because the answer is unavailable: this action's output is text in the file, read in a diff and
approved by a person, while a `var` binding's inferred type is visible nowhere at all. A heterogeneous
literal producing `array<int|string>` is a fact worth showing someone; the same fact attached invisibly to
a binding is how a program acquires a type nobody chose. Same computation, opposite legibility.

The function and its consumer land together: a public function with no consumer has nothing to keep it
honest.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/declarations-and-numbers/#var-inference" title="var takes a local's type from its initializer and fixes it there for good"><code>types/var-inference</code></a> <a href="/docs/rules/types/arrays-and-property-keys/#arrays" title="An array is PHP's ordered hash with string keys and a declared element type"><code>types/arrays</code></a> <a href="/docs/rules/ide/code-actions/#narrow-an-annotation-to-its-literal" title="One editor action rewrites an array&lt;mixed&gt; annotation to the type of the array literal that initializes it, and rewrites nothing else"><code>ide/narrow-an-annotation-to-its-literal</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0114.md">record 0114</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0037.md">record 0037</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/var-refuses-a-bare-array-literal.nvst"><code>tests/conformance/lang/var-refuses-a-bare-array-literal.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="narrowing-is-a-diff-never-a-save-time-fix">

## The narrowing action is never registered under `source.fixAll.nvs` and never chases the call sites it affects, because it breaks writes it cannot see

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#narrowing-is-a-diff-never-a-save-time-fix"><code>ide/narrowing-is-a-diff-never-a-save-time-fix</code></a>
</div>

Narrowing a declaration is not a local edit, because [`types/arrays`](/docs/rules/types/arrays-and-property-keys/#arrays "An array is PHP's ordered hash with string keys and a declared element type")' element type is enforced on
every **write**: a binding that was an `array<mixed>` and is now an `array<array<int>>` refuses the
`$a[] = "x"` and the `$a = $wider;` that the wider annotation admitted, each of them a line the action
did not touch. Reading it is what stays safe — the narrowed array still satisfies every wider parameter
it reached before, that being the covariant direction — so what the action can break is exactly the
writes, and it cannot see them from the declaration. Two rules follow.

**The action is never registered under `source.fixAll.nvs`.** The casing fix and the legacy-cast fix
compose with format-on-save because their edit cannot break another line. This one can, so it is invoked,
previewed and approved like the generators beside it. A client test asserts it is absent from the
`source.fixAll.nvs` set, so a format-on-save never applies it.

**It does not chase the call sites it affects.** Repairing them means choosing between an `as` at the
call, a wider parameter and a narrower one — a refactoring with a decision in it, which
[`ide/a-code-action-writes-only-what-is-already-determined`](/docs/rules/ide/code-actions/#a-code-action-writes-only-what-is-already-determined "A code action that writes may write only text the type system has already fully determined, and never makes a choice") refuses to let a light bulb make.

The request path gets faster where the action is used, which is the point: a narrowed array leaves the
generic helper path for the typed one, and its element writes become compile errors instead of runtime
throws. Nothing is spent per request, and no memory beyond one more interned descriptor where the program
did not already have that type.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/code-actions/#narrow-an-annotation-to-its-literal" title="One editor action rewrites an array&lt;mixed&gt; annotation to the type of the array literal that initializes it, and rewrites nothing else"><code>ide/narrow-an-annotation-to-its-literal</code></a> <a href="/docs/rules/ide/code-actions/#a-code-action-writes-only-what-is-already-determined" title="A code action that writes may write only text the type system has already fully determined, and never makes a choice"><code>ide/a-code-action-writes-only-what-is-already-determined</code></a> <a href="/docs/rules/types/arrays-and-property-keys/#arrays" title="An array is PHP's ordered hash with string keys and a declared element type"><code>types/arrays</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-is-never-a-diagnostic" title="nvs fmt is a separate opt-in tool: no compiler command runs it, an unformatted file is never a diagnostic, and an editor composes it with quick fixes in the client"><code>tooling/fmt-is-never-a-diagnostic</code></a> <a href="/docs/rules/ide/the-language-server/#every-feature-is-staged-behind-its-dependency" title="The VS Code client goes as deep as the editor allows, and each feature waits for the language or runtime piece it needs rather than shipping as a stub"><code>ide/every-feature-is-staged-behind-its-dependency</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0114.md">record 0114</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0188.md">record 0188</a></dd></div></dl>

</div>
