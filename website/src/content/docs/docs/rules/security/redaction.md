---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Redaction in the editor"
description: "A secret is concealed by default, from ranges the server decides — and what concealment cannot cover is written down."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/security/secrets/
  label: "Secrets"
next:
  link: /docs/rules/security/extensions-and-qualifiers/
  label: "Qualifiers across an extension"
---

<p class="nv-section-lead">A secret is concealed by default, from ranges the server decides — and what concealment cannot cover is written down.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">5</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">0</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">5</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">0</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#reveal-is-explicit-and-window-local">A reveal is per range, window-local, and does not survive the editor closing</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#redaction-ranges-come-from-the-server">The editor conceals a <code>secret</code> value by default, and the ranges come from the language server rather than a client guess</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#redaction-covers-bytes-only">Only the bytes are concealed — a literal token or an interpolation slot, never an identifier or an annotation</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#redaction-reaches-the-tools-own-renderings">Every rendering the toolchain itself produces inherits the redaction, the AST dump included</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#redaction-does-not-reach">What the editor's concealment cannot cover is written down, because a redaction trusted past its reach is worse than none</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li></ol>

<div class="nv-rule" id="reveal-is-explicit-and-window-local">

## A reveal is per range, window-local, and does not survive the editor closing

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#reveal-is-explicit-and-window-local"><code>security/reveal-is-explicit-and-window-local</code></a>
</div>

A reveal is per range, window-local, and dropped when the editor for that document closes. It is not
written to workspace state and does not survive a reload.

The threat model is an unattended screen, so a reveal that outlives the moment it was needed is the
same as no redaction at all — and a user who revealed one credential to read it has not consented to
reveal every credential in the workspace for the rest of the week.

There is **no automatic reveal and no automatic re-conceal on a signal**, because there is no signal:
nothing reports that a window is being shared, recorded or projected. Anything that looked like one
would be a guess with a security failure attached, so redaction is unconditional by default and the
user is the only thing that turns it off.

**Not on disk.** There is no language server in the tree.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/redaction/#redaction-ranges-come-from-the-server" title="The editor conceals a secret value by default, and the ranges come from the language server rather than a client guess"><code>security/redaction-ranges-come-from-the-server</code></a> <a href="/docs/rules/security/redaction/#redaction-does-not-reach" title="What the editor's concealment cannot cover is written down, because a redaction trusted past its reach is worse than none"><code>security/redaction-does-not-reach</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0101.md">record 0101</a></dd></div></dl>

</div>

<div class="nv-rule" id="redaction-ranges-come-from-the-server">

## The editor conceals a `secret` value by default, and the ranges come from the language server rather than a client guess

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#redaction-ranges-come-from-the-server"><code>security/redaction-ranges-come-from-the-server</code></a>
</div>

A `secret` value is concealed in the editor by default. The language server answers a list of ranges
and their kinds; the client draws them and knows nothing about what a secret is, because the
alternative is the client guessing.

It does **not** ride the semantic-token channel, even though the qualifier already travels there. That
channel's contract is *names a theme styles*, and its correct degradation is to fall back to the
underlying token type — which a security default whose failure mode is *the value becomes visible*
cannot inherit. Two mechanisms, two contracts, and neither can silently disable the other.

The fail direction is named, because mid-edit is exactly when the type is unknown. A range whose
expression cannot be typed, but whose **binding's declared type carries `secret`**, is redacted
anyway; the client **holds its last answer** and never clears decorations on an error, a cancellation
or a restart. An empty answer means nothing to redact; a *missing* answer means nothing at all.

**Not on disk.** There is no language server in the tree.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/redaction/#redaction-covers-bytes-only" title="Only the bytes are concealed — a literal token or an interpolation slot, never an identifier or an annotation"><code>security/redaction-covers-bytes-only</code></a> <a href="/docs/rules/security/redaction/#reveal-is-explicit-and-window-local" title="A reveal is per range, window-local, and does not survive the editor closing"><code>security/reveal-is-explicit-and-window-local</code></a> <a href="/docs/rules/security/redaction/#redaction-does-not-reach" title="What the editor's concealment cannot cover is written down, because a redaction trusted past its reach is worse than none"><code>security/redaction-does-not-reach</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0101.md">record 0101</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0016.md">record 0016</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a></dd></div></dl>

</div>

<div class="nv-rule" id="redaction-covers-bytes-only">

## Only the bytes are concealed — a literal token or an interpolation slot, never an identifier or an annotation

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#redaction-covers-bytes-only"><code>security/redaction-covers-bytes-only</code></a>
</div>

A range is answered when it is a string, heredoc or `bytes` literal token whose static type carries
`secret`, or an interpolation slot inside one whose interpolated expression does. Nothing else.

**Never an identifier** — a variable or property name is a name, not a secret, and concealing it hides
no bytes while making the file unreadable for the developer whose editor it is. **Never a type
annotation** — `secret string` is the declaration doing its job, and it is how a reader knows the
concealment below it is deliberate rather than a rendering fault. **Never a whole line, statement or
block**, which line-granular folding structurally cannot express anyway.

The concealment is a decoration over the range with the character cells kept, so the cursor, the
selection and every edit still address the real text. It is a rendering, not an edit: the buffer is
the file on disk, byte for byte.

**Not on disk.** There is no language server in the tree.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/redaction/#redaction-ranges-come-from-the-server" title="The editor conceals a secret value by default, and the ranges come from the language server rather than a client guess"><code>security/redaction-ranges-come-from-the-server</code></a> <a href="/docs/rules/security/secrets/#secret-qualifier" title="secret is a second, independent compile-time qualifier, written before tainted and in that order alone"><code>security/secret-qualifier</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0101.md">record 0101</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a></dd></div></dl>

</div>

<div class="nv-rule" id="redaction-reaches-the-tools-own-renderings">

## Every rendering the toolchain itself produces inherits the redaction, the AST dump included

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#redaction-reaches-the-tools-own-renderings"><code>security/redaction-reaches-the-tools-own-renderings</code></a>
</div>

Every rendering the toolchain itself produces inherits the redaction, so no surface prints the
plaintext the editor is concealing. The diagnostic record already carries it as a node kind, which is
what makes plaintext, JSON and HTML renderings agree from one place
([`errors/record-transformations`](/docs/rules/errors/diagnostics-and-logging/#record-transformations "Redaction, control bytes, bidi and elision are decided in the record")).

The AST dump is the surface that would otherwise disagree: a node's own scalar fields include a string
literal's text, so a panel reading it would render a secret the buffer behind it is blurring. A
literal node whose static type carries `secret` emits the same fixed placeholder a dumped property
gets, **in the JSON itself** rather than in the viewer, so the command-line output and the panel
cannot diverge.

The cost is that a frozen dump schema now has a type-dependent field value, and a reader can no longer
assume a literal node's text is the source text.

**Not on disk.** There is no language server, and the dump emits no such placeholder.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/secrets/#secret-sinks-refuse" title="Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one"><code>security/secret-sinks-refuse</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#record-transformations" title="Redaction, control bytes, bidi and elision are decided in the record"><code>errors/record-transformations</code></a> <a href="/docs/rules/security/redaction/#redaction-ranges-come-from-the-server" title="The editor conceals a secret value by default, and the ranges come from the language server rather than a client guess"><code>security/redaction-ranges-come-from-the-server</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0101.md">record 0101</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0092.md">record 0092</a></dd></div></dl>

</div>

<div class="nv-rule" id="redaction-does-not-reach">

## What the editor's concealment cannot cover is written down, because a redaction trusted past its reach is worse than none

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#redaction-does-not-reach"><code>security/redaction-does-not-reach</code></a>
</div>

The concealment is cosmetic, and what it cannot cover is written down and shipped in the tool's own
documentation, because a redaction trusted past its reach is worse than none.

Workspace search results and quick-open previews render matching lines outside any editor, so no
decoration applies. Diff and version-control views can be decorated, but there is no type information
for the "before" side, so the old value of an edited secret is visible in the review of that edit. The
minimap renders from the buffer. Any other extension's hover, lens or webview reads the document text
directly. A copy of a concealed range copies the plaintext, and nothing may intercept the clipboard.
And the file itself is on disk, in the working tree, and in the history the moment it is committed —
concealing a hardcoded credential does not make it less hardcoded.

This list is part of the decision, not commentary on it.

**Not on disk.** There is no language server in the tree.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/redaction/#redaction-ranges-come-from-the-server" title="The editor conceals a secret value by default, and the ranges come from the language server rather than a client guess"><code>security/redaction-ranges-come-from-the-server</code></a> <a href="/docs/rules/security/redaction/#reveal-is-explicit-and-window-local" title="A reveal is per range, window-local, and does not survive the editor closing"><code>security/reveal-is-explicit-and-window-local</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0101.md">record 0101</a></dd></div></dl>

</div>
