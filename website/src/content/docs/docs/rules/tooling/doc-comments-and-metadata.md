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

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">12</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">6</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">6</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">6</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#doc-comment-is-three-slashes">A doc comment is exactly <code>///</code>; <code>////</code> and longer runs are ordinary comments, and <code>#</code> never is one</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#doc-comment-attaches-to-the-next-declaration">A <code>///</code> run attaches to the declaration below it across no blank line, and a doc comment attached to nothing is a diagnostic</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#doc-comment-tags-are-see-and-example">A doc comment is Markdown plus <code>@see</code> and <code>@example</code>, each of which must resolve, and any other <code>@tag</code> is a diagnostic</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#strict-docs">Nothing requires a doc comment by default; <code>nvs check --strict-docs</code> reports a public member without one, and no autofix can satisfy it</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#nvs-doc-renders-and-decides-nothing"><code>nvs doc &lt;entry&gt;</code> writes one Markdown page per class from the JSON and has no source of truth of its own</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#one-json-several-renderers"><code>nvs meta --json</code> is the one machine-readable source of documentation, every renderer consumes it, and no renderer is authoritative for content</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#meta-json"><code>nvs meta --json</code> prints the whole <code>Core</code> registry as one JSON document, and a field with nothing written is an absent key rather than an empty one</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#meta-json-takes-a-program"><code>nvs meta --json &lt;entry&gt;</code> emits that program's own declarations beside the <code>Core</code> registry, in the registry's own shape</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#an-agent-asks-the-binary"><code>nvs agent</code> answers a coding agent from the registry in four commands, and the check loop is part of the surface</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#the-index-is-one-line-per-member"><code>nvs agent index</code> prints one derived line per registered member, carrying its signature and the capability it is gated on</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#a-primer-claim-is-executed">The primer is generated, and every refusal it states and every example it shows is proven against the compiler that ships with it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#an-adapter-carries-protocol-and-never-language"><code>nvs agent init</code> writes one pointer per harness, and no adapter ever states a language fact</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li></ol>

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

**Both tags repeat, and every one of them is checked.** A declaration carries as many `@see` and
`@example` lines as it has cross-references and examples, in whatever order they are written, and the
order they are written is the order they are read back: `nvs meta --json` emits `see` and `example` as
lists ([`tooling/meta-json-takes-a-program`](/docs/rules/tooling/doc-comments-and-metadata/#meta-json-takes-a-program "nvs meta --json <entry> emits that program's own declarations beside the Core registry, in the registry's own shape")) and `nvs doc` renders each list on one line. A second
tag is not a weaker one — a `@see` that does not resolve is the same diagnostic whether it is the first
line of the block or the last.

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
<span class="nv-rule-status" data-status="shipped">Shipped</span>
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

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/doc-comments-and-metadata/#one-json-several-renderers" title="nvs meta --json is the one machine-readable source of documentation, every renderer consumes it, and no renderer is authoritative for content"><code>tooling/one-json-several-renderers</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#meta-json-takes-a-program" title="nvs meta --json &lt;entry&gt; emits that program's own declarations beside the Core registry, in the registry's own shape"><code>tooling/meta-json-takes-a-program</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0137.md">record 0137</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/tests/doc.rs"><code>crates/nvs-cli/tests/doc.rs</code></a></dd></div></dl>

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
                      ├─ website sync:core    → core.json → MDX
nvs meta --json ──────┼─ nvs doc              → Markdown pages
                      └─ nvs agent            → the primer, the index, one card
```

Nothing re-derives documentation from source, and no renderer is authoritative for content. The `Core`
half of the JSON is the registry's card ([`core-api/reference-card`](/docs/rules/core-api/lifetimes-and-absences/#reference-card "An implemented Core member carries its reference card in the registry declaration beside its code")); a program's own declarations
join the same document through one optional argument ([`tooling/meta-json-takes-a-program`](/docs/rules/tooling/doc-comments-and-metadata/#meta-json-takes-a-program "nvs meta --json <entry> emits that program's own declarations beside the Core registry, in the registry's own shape")) rather
than through a second pipeline beside it, and the renderer shipped in the binary
([`tooling/nvs-doc-renders-and-decides-nothing`](/docs/rules/tooling/doc-comments-and-metadata/#nvs-doc-renders-and-decides-nothing "nvs doc <entry> writes one Markdown page per class from the JSON and has no source of truth of its own")) is only that.

Two renderers already sit on the `Core` half — the one-file reference and the website's core data — and
neither reads a Rust file to find a description. A third source of truth for user declarations would be
exactly the duplication that shape exists to avoid.

The fourth arm answers a coding agent rather than a reader ([`tooling/an-agent-asks-the-binary`](/docs/rules/tooling/doc-comments-and-metadata/#an-agent-asks-the-binary "nvs agent answers a coding agent from the registry in four commands, and the check loop is part of the surface")),
and it is on this diagram for the reason the others are: it renders at the call and holds nothing, so
the binary that compiles a program is the binary that documents it.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PhpDocumentor re-derives documentation from source at every run; here nothing does, and the renderers share one JSON</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/doc-comments-and-metadata/#meta-json" title="nvs meta --json prints the whole Core registry as one JSON document, and a field with nothing written is an absent key rather than an empty one"><code>tooling/meta-json</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#meta-json-takes-a-program" title="nvs meta --json &lt;entry&gt; emits that program's own declarations beside the Core registry, in the registry's own shape"><code>tooling/meta-json-takes-a-program</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#nvs-doc-renders-and-decides-nothing" title="nvs doc &lt;entry&gt; writes one Markdown page per class from the JSON and has no source of truth of its own"><code>tooling/nvs-doc-renders-and-decides-nothing</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#reference-card" title="An implemented Core member carries its reference card in the registry declaration beside its code"><code>core-api/reference-card</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0137.md">record 0137</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0117.md">record 0117</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0167.md">record 0167</a></dd></div></dl>

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

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/lifetimes-and-absences/#reference-card" title="An implemented Core member carries its reference card in the registry declaration beside its code"><code>core-api/reference-card</code></a> <a href="/docs/rules/core-api/shape-parameters/#field-wise-precedence" title="Where the registry and the spec both describe an implemented member, precedence is field by field"><code>core-api/field-wise-precedence</code></a> <a href="/docs/rules/core-api/parameters-and-options/#parameters-are-callable-by-name" title="Every Core parameter is callable by the name the spec writes, and that name is compatibility surface"><code>core-api/parameters-are-callable-by-name</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#meta-json-takes-a-program" title="nvs meta --json &lt;entry&gt; emits that program's own declarations beside the Core registry, in the registry's own shape"><code>tooling/meta-json-takes-a-program</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#one-json-several-renderers" title="nvs meta --json is the one machine-readable source of documentation, every renderer consumes it, and no renderer is authoritative for content"><code>tooling/one-json-several-renderers</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0117.md">record 0117</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0167.md">record 0167</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/tests/meta.rs"><code>crates/nvs-cli/tests/meta.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/meta.rs"><code>crates/nvs-cli/src/meta.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="meta-json-takes-a-program">

## `nvs meta --json <entry>` emits that program's own declarations beside the `Core` registry, in the registry's own shape

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
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

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/doc-comments-and-metadata/#meta-json" title="nvs meta --json prints the whole Core registry as one JSON document, and a field with nothing written is an absent key rather than an empty one"><code>tooling/meta-json</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#one-json-several-renderers" title="nvs meta --json is the one machine-readable source of documentation, every renderer consumes it, and no renderer is authoritative for content"><code>tooling/one-json-several-renderers</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-tags-are-see-and-example" title="A doc comment is Markdown plus @see and @example, each of which must resolve, and any other @tag is a diagnostic"><code>tooling/doc-comment-tags-are-see-and-example</code></a> <a href="/docs/rules/ide/testing-the-editor/#an-lsp-answer-is-frozen-as-an-lspt-case" title="An LSP answer is frozen as a .lspt case — a document, a &lt;|&gt; cursor, a request and its rendering — run by nvs lsp-test printing N passed, M failed"><code>ide/an-lsp-answer-is-frozen-as-an-lspt-case</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0137.md">record 0137</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0117.md">record 0117</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/tests/meta.rs"><code>crates/nvs-cli/tests/meta.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="an-agent-asks-the-binary">

## `nvs agent` answers a coding agent from the registry in four commands, and the check loop is part of the surface

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#an-agent-asks-the-binary"><code>tooling/an-agent-asks-the-binary</code></a>
</div>

`nvs agent` is the surface a coding agent reads the language through, and it is four commands that
answer from the registry the binary already carries: `primer` prints the short document that makes an
agent productive, `index` prints one line per member, `find <query>` prints the index lines matching a
query, and `show <symbol>` prints one member's card. Nothing is written to disk and nothing is cached,
so the binary that compiles a program is the binary that answers for it and an answer can never
describe a version that is not installed.

All four render `nvs meta --json`'s document and decide nothing ([`tooling/one-json-several-renderers`](/docs/rules/tooling/doc-comments-and-metadata/#one-json-several-renderers "nvs meta --json is the one machine-readable source of documentation, every renderer consumes it, and no renderer is authoritative for content")),
which makes them a fourth consumer rather than a fifth source of truth. `find` is a command rather than
an instruction to grep the index, because a namespaced name loses its backslash to the shell before
`grep` sees it and the empty result that follows is indistinguishable from a name the language does not
have.

The protocol is three calls and a check: read `primer` once, `find` a name, `show` its card, then
`nvs check`. A diagnostic is the cheapest documentation the system has — it is read only by the agent
that got something wrong — so the check loop is part of the surface rather than an alternative to it.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/doc-comments-and-metadata/#one-json-several-renderers" title="nvs meta --json is the one machine-readable source of documentation, every renderer consumes it, and no renderer is authoritative for content"><code>tooling/one-json-several-renderers</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#meta-json" title="nvs meta --json prints the whole Core registry as one JSON document, and a field with nothing written is an absent key rather than an empty one"><code>tooling/meta-json</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#the-index-is-one-line-per-member" title="nvs agent index prints one derived line per registered member, carrying its signature and the capability it is gated on"><code>tooling/the-index-is-one-line-per-member</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#a-primer-claim-is-executed" title="The primer is generated, and every refusal it states and every example it shows is proven against the compiler that ships with it"><code>tooling/a-primer-claim-is-executed</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#an-adapter-carries-protocol-and-never-language" title="nvs agent init writes one pointer per harness, and no adapter ever states a language fact"><code>tooling/an-adapter-carries-protocol-and-never-language</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0167.md">record 0167</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/tests/agent.rs"><code>crates/nvs-cli/tests/agent.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="the-index-is-one-line-per-member">

## `nvs agent index` prints one derived line per registered member, carrying its signature and the capability it is gated on

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#the-index-is-one-line-per-member"><code>tooling/the-index-is-one-line-per-member</code></a>
</div>

`nvs agent index` prints exactly one line for every member the registry holds, and one for every enum,
exception and attribute beside them. It is derived at the call and kept nowhere
([`testing/roster-is-derived`](/docs/rules/testing/feature-proofs/#roster-is-derived "The roster of features is read from the registry and the reference chapters, never kept as a list")'s shape), so a member that lands owes its line at once and no list is
ever stale.

Completeness is the property the command exists to have. An agent that greps a complete list learns
something from an empty result — that the name it guessed does not exist — and learns nothing at all
from an empty result over a list that merely happens not to mention it. That is why the index is
enumerated from the registry rather than written, and why the guard is a member-for-member
correspondence rather than a count.

A line carries the member's signature in the spec's own spelling and the capability the call is gated
on, written after it in brackets: `Core\IO::read(string $path): string  [fs.read]`. The capability is
joined from [`security/capability-declaration-is-one-table`](/docs/rules/security/capabilities/#capability-declaration-is-one-table "What each Core member needs is declared once in one table, and nothing at run time reads it")'s one table at render time, never
copied onto a member row — that table's own rule refuses a per-member field, and this reads it rather
than reshaping it. So an agent learns the gate from the name of the thing it is about to call, which is
where every arm of `0167`'s investigation was stopped.

A line carries no behaviour: what `header: true` does to a row is the card's answer, which is what
`show` is for.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/doc-comments-and-metadata/#an-agent-asks-the-binary" title="nvs agent answers a coding agent from the registry in four commands, and the check loop is part of the surface"><code>tooling/an-agent-asks-the-binary</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#meta-json" title="nvs meta --json prints the whole Core registry as one JSON document, and a field with nothing written is an absent key rather than an empty one"><code>tooling/meta-json</code></a> <a href="/docs/rules/testing/feature-proofs/#roster-is-derived" title="The roster of features is read from the registry and the reference chapters, never kept as a list"><code>testing/roster-is-derived</code></a> <a href="/docs/rules/security/capabilities/#capability-check-at-the-door" title="The capability check lives inside the function that performs the effect, and that door is the only way out of the process"><code>security/capability-check-at-the-door</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0167.md">record 0167</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/tests/agent.rs"><code>crates/nvs-cli/tests/agent.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-primer-claim-is-executed">

## The primer is generated, and every refusal it states and every example it shows is proven against the compiler that ships with it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#a-primer-claim-is-executed"><code>tooling/a-primer-claim-is-executed</code></a>
</div>

Every claim the primer makes is proven against the compiler that ships with it: each example in it is
run and must print what the primer says it prints, and each refusal it states must name a diagnostic
code that compiler declares. `python tools/reference.py --primer --check` is that proof, and it is the
harness `docs/novis.md`'s examples already run under, applied to the one document that is read by
someone who has nothing else.

A refusal is checked as its code rather than as a program because its PHP cell is a fragment — a
`list($a) = $b`, an untyped `as $each` — that no `nvs check` can be handed. The `E0xxx` beside it is
the executable half: it either names a constant in the diagnostic registry or it names nothing, and a
refusal the compiler cannot raise is the one lie a document generated from marked sections can still
tell.

The primer is generated — from marked sections of the reference chapters, those chapters' front matter,
and the registry — so it cannot drift from the language, and a section that stops being true stops being
rendered rather than becoming a lie. What it carries is fixed by what an agent gets wrong without it:
the lookup protocol, one complete worked program with every shape annotated, the capability model and
the smallest `nvs.toml` that grants a file read, the refusal table, and the chapter map.

The refusal table is the highest-value part and the reason the order puts it late rather than first: a
model's prior for a language that reads like PHP is confident and wrong, so what Novis refuses and what
to write instead is worth more per byte than what Novis has. Its budget is a low four figures of tokens,
and it is met by what the primer selects — never by trimming what a selected section says.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/doc-comments-and-metadata/#an-agent-asks-the-binary" title="nvs agent answers a coding agent from the registry in four commands, and the check loop is part of the surface"><code>tooling/an-agent-asks-the-binary</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#the-index-is-one-line-per-member" title="nvs agent index prints one derived line per registered member, carrying its signature and the capability it is gated on"><code>tooling/the-index-is-one-line-per-member</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0167.md">record 0167</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/tests/agent.rs"><code>crates/nvs-cli/tests/agent.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tools/reference.py"><code>tools/reference.py</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="an-adapter-carries-protocol-and-never-language">

## `nvs agent init` writes one pointer per harness, and no adapter ever states a language fact

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#an-adapter-carries-protocol-and-never-language"><code>tooling/an-adapter-carries-protocol-and-never-language</code></a>
</div>

`nvs agent init` installs the surface into a project by writing one pointer per harness — an `AGENTS.md`
stanza, which is harness-neutral, and beside it an adapter for each harness that is present, such as a
Claude Code skill at `.claude/skills/novis/SKILL.md`. Each names the four `nvs agent` commands and the
`nvs check` loop.

**No adapter states a language fact.** Not a member signature, not a refusal, not a type. A language
fact written into an adapter is a copy that goes stale the day the member changes, and every copy is
read by an agent that has no way to know it is old — which is the failure the whole surface is arranged
to avoid. An adapter says where to ask; the binary answers.

That is what keeps the adapter list open. Another harness is another short pointer file, adding one
decides nothing and reopens nothing, and none of them can disagree with the language, because none of
them says anything about it.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/doc-comments-and-metadata/#an-agent-asks-the-binary" title="nvs agent answers a coding agent from the registry in four commands, and the check loop is part of the surface"><code>tooling/an-agent-asks-the-binary</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#a-primer-claim-is-executed" title="The primer is generated, and every refusal it states and every example it shows is proven against the compiler that ships with it"><code>tooling/a-primer-claim-is-executed</code></a> <a href="/docs/rules/ide/the-vs-code-extension/#the-extension-guides-an-install-and-never-bundles-one" title="The extension ships no nvs binary: a missing one is a guided install the user starts, verified against the release's own SHA256SUMS"><code>ide/the-extension-guides-an-install-and-never-bundles-one</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0167.md">record 0167</a></dd></div></dl>

</div>
