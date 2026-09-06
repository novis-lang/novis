---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Doc comments and `nvs meta`"
description: "A doc comment is exactly ///, and one JSON document is the machine-readable source every renderer consumes."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/tooling/the-formatter/
  label: "The formatter"
next:
  link: /docs/rules/tooling/nvs-convert/
  label: "Converting PHP"
---

<p class="nv-section-lead">A doc comment is exactly <code>///</code>, and one JSON document is the machine-readable source every renderer consumes.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">8</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">1</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">7</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">6</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#doc-comment-is-three-slashes">A doc comment is exactly <code>///</code>; <code>////</code> and longer runs are ordinary comments, and <code>#</code> never is one</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#doc-comment-attaches-to-the-next-declaration">A <code>///</code> run attaches to the declaration below it across no blank line, and a doc comment attached to nothing is a diagnostic</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#doc-comment-tags-are-see-and-example">A doc comment is Markdown plus <code>@see</code> and <code>@example</code>, each of which must resolve, and any other <code>@tag</code> is a diagnostic</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#strict-docs">Nothing requires a doc comment by default; <code>nvs check --strict-docs</code> reports a public member without one, and no autofix can satisfy it</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#nvs-doc-renders-and-decides-nothing"><code>nvs doc &lt;entry&gt;</code> writes one Markdown page per class from the JSON and has no source of truth of its own</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#one-json-several-renderers"><code>nvs meta --json</code> is the one machine-readable source of documentation, every renderer consumes it, and no renderer is authoritative for content</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#meta-json"><code>nvs meta --json</code> prints the whole <code>Core</code> registry as one JSON document, and a field with nothing written is an absent key rather than an empty one</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#meta-json-takes-a-program"><code>nvs meta --json &lt;entry&gt;</code> emits that program's own declarations beside the <code>Core</code> registry, in the registry's own shape</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li></ol>

<div class="nv-rule" id="doc-comment-is-three-slashes">

## A doc comment is exactly `///`; `////` and longer runs are ordinary comments, and `#` never is one

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#doc-comment-is-three-slashes"><code>tooling/doc-comment-is-three-slashes</code></a>
</div>

A line comment opened by exactly three slashes is a doc comment, lexed as its own trivia kind. Four or
more slashes is an ordinary comment, as in Rust and for the same reason: a divider line of slashes is not
documentation. A `#` comment is never a doc comment whatever its length — `#[` already opens an attribute,
and a second doc spelling is what [`statements/nothing-gets-a-second-name`](/docs/rules/statements/names-and-require/#nothing-gets-a-second-name "A declaration is reachable under exactly the name it was declared with") refuses.

```php
/// The price in cents. Money is `decimal`, never `float`.
/// A negative amount throws; zero is allowed and is a no-op.
public function charge(uint $cents): void { … }

// An ordinary comment. Nothing reads it.
//// ─────────────────────────────  also ordinary
```

The comment documents the declaration it precedes ([`tooling/doc-comment-attaches-to-the-next-declaration`](/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-attaches-to-the-next-declaration "A /// run attaches to the declaration below it across no blank line, and a doc comment attached to nothing is a diagnostic")),
its body is Markdown plus two tags ([`tooling/doc-comment-tags-are-see-and-example`](/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-tags-are-see-and-example "A doc comment is Markdown plus @see and @example, each of which must resolve, and any other @tag is a diagnostic")), and it belongs
to *Novis* source only: a `Core` member is Rust and documents itself in the registry
([`core-api/reference-card`](/docs/rules/core-api/lifetimes-and-absences/#reference-card "An implemented Core member carries its reference card in the registry declaration beside its code")), so it never carries one. There is no module-level doc spelling either — a
file's declarations are its surface, and the class is the unit.

A trivium's classification never reaches the checker, so no program output moves; what a compile spends is
one variant test per line comment in the lexer, and nothing on any request path. A doc comment rendered
to HTML crosses the bidi boundary the lexer already checks for every comment span
([`security/bidi-boundaries`](/docs/rules/security/bidi-and-passwords/#bidi-boundaries "The lexer refuses an unterminated control and the sinks substitute it, and nobody writes a second predicate")), and reuses that check rather than growing a second one.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>/** */</code> docblock; the PHPDoc shape is an ordinary comment nothing reads</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-attaches-to-the-next-declaration" title="A /// run attaches to the declaration below it across no blank line, and a doc comment attached to nothing is a diagnostic"><code>tooling/doc-comment-attaches-to-the-next-declaration</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-tags-are-see-and-example" title="A doc comment is Markdown plus @see and @example, each of which must resolve, and any other @tag is a diagnostic"><code>tooling/doc-comment-tags-are-see-and-example</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#reference-card" title="An implemented Core member carries its reference card in the registry declaration beside its code"><code>core-api/reference-card</code></a> <a href="/docs/rules/security/bidi-and-passwords/#bidi-boundaries" title="The lexer refuses an unterminated control and the sinks substitute it, and nobody writes a second predicate"><code>security/bidi-boundaries</code></a> <a href="/docs/rules/statements/names-and-require/#nothing-gets-a-second-name" title="A declaration is reachable under exactly the name it was declared with"><code>statements/nothing-gets-a-second-name</code></a> <a href="/docs/rules/ide/the-resilient-parse/#one-grammar-one-tree" title="The resilient parse is the one grammar's AST plus a trivia layer and an offset index, never a second tree"><code>ide/one-grammar-one-tree</code></a> <a href="/docs/rules/ide/testing-the-editor/#an-lsp-answer-is-frozen-as-an-lspt-case" title="An LSP answer is frozen as a .lspt case — a document, a &lt;|&gt; cursor, a request and its rendering — run by nvs lsp-test printing N passed, M failed"><code>ide/an-lsp-answer-is-frozen-as-an-lspt-case</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0137.md">record 0137</a></dd></div></dl>

</div>

<div class="nv-rule" id="doc-comment-attaches-to-the-next-declaration">

## A `///` run attaches to the declaration below it across no blank line, and a doc comment attached to nothing is a diagnostic

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#doc-comment-attaches-to-the-next-declaration"><code>tooling/doc-comment-attaches-to-the-next-declaration</code></a>
</div>

A run of `///` lines is one doc comment, and it attaches to the next declaration. Consecutive `///` lines
with only whitespace between them are one comment; a `///` followed by a blank line and then a declaration
is not attached to it; and a `///` attached to nothing is a diagnostic — *this doc comment is attached to
nothing; a doc comment documents the declaration it precedes.*

Attachment is what separates documentation from a note-to-self. The alternative — any comment run above a
declaration is its documentation — needs no new syntax and would start working on every file already
written, including converted PHP; but then a note and a document are the same token, only the author knew
which was meant, and converted PHP arrives full of the first kind. Requiring the marker
([`tooling/doc-comment-is-three-slashes`](/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-is-three-slashes "A doc comment is exactly ///; //// and longer runs are ordinary comments, and # never is one")) and refusing an orphan keeps the two apart.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-is-three-slashes" title="A doc comment is exactly ///; //// and longer runs are ordinary comments, and # never is one"><code>tooling/doc-comment-is-three-slashes</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#strict-docs" title="Nothing requires a doc comment by default; nvs check --strict-docs reports a public member without one, and no autofix can satisfy it"><code>tooling/strict-docs</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0137.md">record 0137</a></dd></div></dl>

</div>

<div class="nv-rule" id="doc-comment-tags-are-see-and-example">

## A doc comment is Markdown plus `@see` and `@example`, each of which must resolve, and any other `@tag` is a diagnostic

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#doc-comment-tags-are-see-and-example"><code>tooling/doc-comment-tags-are-see-and-example</code></a>
</div>

The body of a doc comment is Markdown. Two tags may appear, each on its own line in a trailing block:

```php
/// The price in cents, never a float.
///
/// @see Core\Money::fromCents
/// @example examples/charge.nvs
```

`@see <member>` names a class, member, enum or constant, and must resolve or it is a diagnostic.
`@example <path>` names a file that must exist **and must be inside a directory the test corpus walks**,
so an example that stops compiling fails the build rather than rotting in a page. **Any other `@tag` at
the start of a line is a diagnostic**, and that one sentence is the entire difference between a closed set
and a convention: an unenforced set grows, and the growth is how PHPDoc arrived at documenting a signature
twice. An `@` anywhere else in the prose is just a character.

There is no `@param`, `@return`, `@throws`, `@var`, `@deprecated`, `@since` or `@internal`, because almost
nothing PHPDoc carried survives as prose. Parameter and return types are the signature, which cannot drift
from itself; a callable carries its own ([`types/callable-signature`](/docs/rules/types/closures/#callable-signature "A callable type may name its parameters, and must then name its return type")); what a `Core` member throws is
its registry card ([`core-api/reference-card`](/docs/rules/core-api/lifetimes-and-absences/#reference-card "An implemented Core member carries its reference card in the registry declaration beside its code")) and for user code is a sentence; every binding is
annotated, so `@var` has nothing to say; deprecation is an attribute ([`attributes/inert-metadata`](/docs/rules/attributes/#inert-metadata "An attribute is a shape literal on a declaration, and nothing is ever declared or instantiated for it"));
"this touches the filesystem" is a declared capability ([`security/capability-check-at-the-door`](/docs/rules/security/capabilities/#capability-check-at-the-door "The capability check lives inside the function that performs the effect, and that door is the only way out of the process")). A
parameter that needs explaining is named in a sentence — "the timeout is in **milliseconds**" — and the
diagnostic says so: *`@param` is not a documentation tag; name the parameter in a sentence instead — its
type is in the signature*, and likewise for `@throws` and `@returns`.

The two tags survive because each buys a *check* — a cross-reference that must resolve, an example that
must still compile — and a third has to meet the same standard, never merely improve a rendering.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>@param</code>, <code>@return</code>, <code>@throws</code>, <code>@var</code> or <code>@deprecated</code>; what PHPDoc tags carried is the signature, an attribute or a declared capability</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-is-three-slashes" title="A doc comment is exactly ///; //// and longer runs are ordinary comments, and # never is one"><code>tooling/doc-comment-is-three-slashes</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#meta-json-takes-a-program" title="nvs meta --json &lt;entry&gt; emits that program's own declarations beside the Core registry, in the registry's own shape"><code>tooling/meta-json-takes-a-program</code></a> <a href="/docs/rules/attributes/#inert-metadata" title="An attribute is a shape literal on a declaration, and nothing is ever declared or instantiated for it"><code>attributes/inert-metadata</code></a> <a href="/docs/rules/security/capabilities/#capability-check-at-the-door" title="The capability check lives inside the function that performs the effect, and that door is the only way out of the process"><code>security/capability-check-at-the-door</code></a> <a href="/docs/rules/types/closures/#callable-signature" title="A callable type may name its parameters, and must then name its return type"><code>types/callable-signature</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#reference-card" title="An implemented Core member carries its reference card in the registry declaration beside its code"><code>core-api/reference-card</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0137.md">record 0137</a></dd></div></dl>

</div>

<div class="nv-rule" id="strict-docs">

## Nothing requires a doc comment by default; `nvs check --strict-docs` reports a public member without one, and no autofix can satisfy it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#strict-docs"><code>tooling/strict-docs</code></a>
</div>

`nvs check` is silent about documentation. `nvs check --strict-docs` reports a **public** member with no
attached doc comment ([`tooling/doc-comment-attaches-to-the-next-declaration`](/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-attaches-to-the-next-declaration "A /// run attaches to the declaration below it across no blank line, and a doc comment attached to nothing is a diagnostic")); publishing a package
turns it on unconditionally. A private helper is never reported, and neither is an application, at any
setting, unless it asks.

Why this cannot become the failure mode it is modelled against: an editor that demands a docblock is
answered with a generated one, and a generated docblock is noise nobody reads and everybody deletes. Here
there is nothing to generate. With no `@param` and no `@return`
([`tooling/doc-comment-tags-are-see-and-example`](/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-tags-are-see-and-example "A doc comment is Markdown plus @see and @example, each of which must resolve, and any other @tag is a diagnostic")), a synthesized `///` would be empty, so no autofix is
possible and the only way to satisfy the check is to write a sentence. A diagnostic in every project was
rejected for the same reason: the pressure would be answered by `/// Charges the card.` above
`chargeTheCard()`, noise a human typed that will outlive the method's behaviour.

Until a package manager exists, `--strict-docs` is opt-in only and nothing fires it automatically. If
publishing turns out to want more than "a public member has a comment" — a minimum length, a required first
sentence — that is a lint's design and belongs with the publisher.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>An IDE cannot answer the check with a generated docblock, because there is no tag to generate</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-tags-are-see-and-example" title="A doc comment is Markdown plus @see and @example, each of which must resolve, and any other @tag is a diagnostic"><code>tooling/doc-comment-tags-are-see-and-example</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-attaches-to-the-next-declaration" title="A /// run attaches to the declaration below it across no blank line, and a doc comment attached to nothing is a diagnostic"><code>tooling/doc-comment-attaches-to-the-next-declaration</code></a> <a href="/docs/rules/packaging/packages/#a-package-is-its-digest" title="A package is an immutable source archive whose identity is its BLAKE3 digest"><code>packaging/a-package-is-its-digest</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0137.md">record 0137</a></dd></div></dl>

</div>

<div class="nv-rule" id="nvs-doc-renders-and-decides-nothing">

## `nvs doc <entry>` writes one Markdown page per class from the JSON and has no source of truth of its own

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#nvs-doc-renders-and-decides-nothing"><code>tooling/nvs-doc-renders-and-decides-nothing</code></a>
</div>

`nvs doc <entry>` writes one Markdown page per class from the JSON [`tooling/meta-json-takes-a-program`](/docs/rules/tooling/doc-comments-and-metadata/#meta-json-takes-a-program "nvs meta --json <entry> emits that program's own declarations beside the Core registry, in the registry's own shape")
emits. It ships in the binary because a user's project does not have this repository's `tools/reference.py`,
and it is deliberately the least interesting part of the design: a renderer with no source of truth of its
own ([`tooling/one-json-several-renderers`](/docs/rules/tooling/doc-comments-and-metadata/#one-json-several-renderers "nvs meta --json is the one machine-readable source of documentation, every renderer consumes it, and no renderer is authoritative for content")), so replacing it later costs nothing.

It renders text the lexer has already accepted, so it needs no bidi check of its own
([`security/bidi-boundaries`](/docs/rules/security/bidi-and-passwords/#bidi-boundaries "The lexer refuses an unterminated control and the sinks substitute it, and nobody writes a second predicate")). This repository does not itself need it — the one-file reference and the
website already cover every in-tree consumer — and it exists for a user's own project and for the package
ecosystem that does not exist yet, which is why it is kept cheap to replace.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>The renderer ships inside the binary rather than as a phpDocumentor-style tool a project installs</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/doc-comments-and-metadata/#one-json-several-renderers" title="nvs meta --json is the one machine-readable source of documentation, every renderer consumes it, and no renderer is authoritative for content"><code>tooling/one-json-several-renderers</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#meta-json-takes-a-program" title="nvs meta --json &lt;entry&gt; emits that program's own declarations beside the Core registry, in the registry's own shape"><code>tooling/meta-json-takes-a-program</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0137.md">record 0137</a></dd></div></dl>

</div>

<div class="nv-rule" id="one-json-several-renderers">

## `nvs meta --json` is the one machine-readable source of documentation, every renderer consumes it, and no renderer is authoritative for content

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#one-json-several-renderers"><code>tooling/one-json-several-renderers</code></a>
</div>

`nvs meta --json` ([`tooling/meta-json`](/docs/rules/tooling/doc-comments-and-metadata/#meta-json "nvs meta --json prints the whole Core registry as one JSON document, and a field with nothing written is an absent key rather than an empty one")) is the single machine-readable source of documentation, and
every renderer consumes it:

```
                      ┌─ tools/reference.py  → docs/novis.md
nvs meta --json ──────┼─ website sync:core    → core.json → MDX
                      └─ nvs doc              → Markdown pages
```

Nothing re-derives documentation from source, and no renderer is authoritative for content. The `Core`
half of the JSON is the registry's card ([`core-api/reference-card`](/docs/rules/core-api/lifetimes-and-absences/#reference-card "An implemented Core member carries its reference card in the registry declaration beside its code")); a program's own declarations
join the same document through one optional argument ([`tooling/meta-json-takes-a-program`](/docs/rules/tooling/doc-comments-and-metadata/#meta-json-takes-a-program "nvs meta --json <entry> emits that program's own declarations beside the Core registry, in the registry's own shape")) rather
than through a second pipeline beside it, and the renderer shipped in the binary
([`tooling/nvs-doc-renders-and-decides-nothing`](/docs/rules/tooling/doc-comments-and-metadata/#nvs-doc-renders-and-decides-nothing "nvs doc <entry> writes one Markdown page per class from the JSON and has no source of truth of its own")) is only that.

Two renderers already sit on the `Core` half — the one-file reference and the website's core data — and
neither reads a Rust file to find a description. A third source of truth for user declarations would be
exactly the duplication that shape exists to avoid.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PhpDocumentor re-derives documentation from source at every run; here nothing does, and the renderers share one JSON</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/doc-comments-and-metadata/#meta-json" title="nvs meta --json prints the whole Core registry as one JSON document, and a field with nothing written is an absent key rather than an empty one"><code>tooling/meta-json</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#meta-json-takes-a-program" title="nvs meta --json &lt;entry&gt; emits that program's own declarations beside the Core registry, in the registry's own shape"><code>tooling/meta-json-takes-a-program</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#nvs-doc-renders-and-decides-nothing" title="nvs doc &lt;entry&gt; writes one Markdown page per class from the JSON and has no source of truth of its own"><code>tooling/nvs-doc-renders-and-decides-nothing</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#reference-card" title="An implemented Core member carries its reference card in the registry declaration beside its code"><code>core-api/reference-card</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0137.md">record 0137</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0117.md">record 0117</a></dd></div></dl>

</div>

<div class="nv-rule" id="meta-json">

## `nvs meta --json` prints the whole `Core` registry as one JSON document, and a field with nothing written is an absent key rather than an empty one

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#meta-json"><code>tooling/meta-json</code></a>
</div>

`nvs meta --json` prints the `Core` registry as one JSON document: every class with its members and its
constants, and the enums beside them at top level — top level because the registry's roster is, an enum
having no owner class there. Each member's reference card ([`core-api/reference-card`](/docs/rules/core-api/lifetimes-and-absences/#reference-card "An implemented Core member carries its reference card in the registry declaration beside its code")) sits under a
`doc` key — `short`, `params` with each parameter's `name`, `desc` and shape keys, `return`, `errors` —
and each member also carries its signature half: `kind`, `signature` in the spec's own spelling, `params`
with types, qualifiers and defaults, `options`, `returns`; a class its `typeParams` and `constructor`, a
constant its `type` and `value`. Four rosters the compiler declares outside the registry sit beside
`classes` and `enums`: `exceptions`, `interfaces`, `attributes` and `directives`.

The omission rule is the same at every level: a row with nothing written has no `doc` key, a written card
carries only its non-empty fields, and no array is ever emitted empty. That is
[`core-api/field-wise-precedence`](/docs/rules/core-api/shape-parameters/#field-wise-precedence "Where the registry and the spec both describe an implemented member, precedence is field by field") made mechanical — an absent key is the one spelling of "not written"
a consumer can tell from "written, and empty" without learning a convention.

The command owns the contract, and `--json` is required so that a `meta` with nothing named cannot
succeed by printing nothing. A consumer ignores fields it does not know, so a field may be added but never
renamed or moved; a consumer on a toolchain without the subcommand treats it as "no registry docs yet",
never as an error. `docs/novis.md` and the website's core data are both built from this command alone.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>php --rf</code>, no reflection dump and no phpDocumentor pass over the standard library; one command from the binary is the reference's only input</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/lifetimes-and-absences/#reference-card" title="An implemented Core member carries its reference card in the registry declaration beside its code"><code>core-api/reference-card</code></a> <a href="/docs/rules/core-api/shape-parameters/#field-wise-precedence" title="Where the registry and the spec both describe an implemented member, precedence is field by field"><code>core-api/field-wise-precedence</code></a> <a href="/docs/rules/core-api/parameters-and-options/#parameters-are-callable-by-name" title="Every Core parameter is callable by the name the spec writes, and that name is compatibility surface"><code>core-api/parameters-are-callable-by-name</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#meta-json-takes-a-program" title="nvs meta --json &lt;entry&gt; emits that program's own declarations beside the Core registry, in the registry's own shape"><code>tooling/meta-json-takes-a-program</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#one-json-several-renderers" title="nvs meta --json is the one machine-readable source of documentation, every renderer consumes it, and no renderer is authoritative for content"><code>tooling/one-json-several-renderers</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0117.md">record 0117</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/tests/meta.rs"><code>crates/nvs-cli/tests/meta.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/meta.rs"><code>crates/nvs-cli/src/meta.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="meta-json-takes-a-program">

## `nvs meta --json <entry>` emits that program's own declarations beside the `Core` registry, in the registry's own shape

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#meta-json-takes-a-program"><code>tooling/meta-json-takes-a-program</code></a>
</div>

`nvs meta --json` with no argument prints the `Core` registry, a static compile-time table, exactly as
[`tooling/meta-json`](/docs/rules/tooling/doc-comments-and-metadata/#meta-json "nvs meta --json prints the whole Core registry as one JSON document, and a field with nothing written is an absent key rather than an empty one") states. With an entry path it prints the same registry plus that program's own
declarations:

```
nvs meta --json                 # the Core registry
nvs meta --json app.nvs         # the Core registry, plus that program's own declarations
```

The second form is **program-dependent** — it parses and resolves the program — which is a materially
different command wearing the same name, so it is stated here rather than discovered. A user declaration
is emitted in a shape mirroring the registry's own: name, signature, the doc comment's prose, its `@see`
list and its `@example` list ([`tooling/doc-comment-tags-are-see-and-example`](/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-tags-are-see-and-example "A doc comment is Markdown plus @see and @example, each of which must resolve, and any other @tag is a diagnostic")). The `Core` half of that
shape is the registry's and is unchanged by this; the user half is this rule's.

The no-argument form must emit byte-identical output before and after the argument exists: the seam is
one input added to one document, never a fork.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/doc-comments-and-metadata/#meta-json" title="nvs meta --json prints the whole Core registry as one JSON document, and a field with nothing written is an absent key rather than an empty one"><code>tooling/meta-json</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#one-json-several-renderers" title="nvs meta --json is the one machine-readable source of documentation, every renderer consumes it, and no renderer is authoritative for content"><code>tooling/one-json-several-renderers</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-tags-are-see-and-example" title="A doc comment is Markdown plus @see and @example, each of which must resolve, and any other @tag is a diagnostic"><code>tooling/doc-comment-tags-are-see-and-example</code></a> <a href="/docs/rules/ide/testing-the-editor/#an-lsp-answer-is-frozen-as-an-lspt-case" title="An LSP answer is frozen as a .lspt case — a document, a &lt;|&gt; cursor, a request and its rendering — run by nvs lsp-test printing N passed, M failed"><code>ide/an-lsp-answer-is-frozen-as-an-lspt-case</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0137.md">record 0137</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0117.md">record 0117</a></dd></div></dl>

</div>
