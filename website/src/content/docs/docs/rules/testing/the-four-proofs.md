---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "The four proofs"
description: "A feature is finished when it has a test from both sides, three examples, a measured figure and an attack against it."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/testing/doubles-and-the-runner/
  label: "Doubles, determinism and the runner"
next:
  link: /docs/rules/testing/coverage-and-probes/
  label: "Coverage, tracing and the debug probes"
---

<p class="nv-section-lead">A feature is finished when it has a test from both sides, three examples, a measured figure and an attack against it.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">8</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">8</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">0</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#nvst-is-separate"><code>.nvst</code> proves the language; <code>#[Test]</code> is how a program written in Novis tests itself</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#one-slice-is-one-feature">One slice writes all four of a feature's proofs together</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#examples-live-in-the-repository"><code>docs/examples/</code> is authoritative and the website mirrors it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#hostile-case-contract">A hostile case passes when nothing came apart, and a compile diagnostic is never a pass</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#four-proofs">A feature is finished when it has a test from both sides, three examples, a measured figure and an attack</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#roster-is-derived">The roster of features is read from the registry and the reference chapters, never kept as a list</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#proof-attribution">An example, an attack and a bench are attributed by path; a test is attributed by a marker</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#a-failing-proof-is-fixed-or-recorded">A proof that fails is fixed or recorded as a known gap, and never weakened</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="nvst-is-separate">

## `.nvst` proves the language; `#[Test]` is how a program written in Novis tests itself

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#nvst-is-separate"><code>testing/nvst-is-separate</code></a>
</div>

`.nvst` is a whole-program, expected-stdout conformance case — a deliberate superset of PHP's
`.phpt`, so importing PHP's corpus stays mechanical — and it is how **Novis's own conformance to its
specification** is proven, the differential comparison against real PHP included.

`#[Test]` is how **a program written in Novis** tests itself.

The two formats answer different questions and are not unified, now or later. `nvs test` runs
both — a path of `.nvst` files, or a program's compiled test table — and reports each in the shape
that fits it.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/writing-a-test/#test-attribute" title="A test is a method marked #[Test], and its table is built while compiling"><code>testing/test-attribute</code></a> <a href="/docs/rules/testing/doubles-and-the-runner/#report-formats" title="One verdict, three renderings, a machine format owns stdout alone, and the JSON one locates every test it reports"><code>testing/report-formats</code></a> <a href="/docs/rules/testing/the-four-proofs/#four-proofs" title="A feature is finished when it has a test from both sides, three examples, a measured figure and an attack"><code>testing/four-proofs</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-test/src/lib.rs"><code>crates/nvs-test/src/lib.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/main.rs"><code>crates/nvs-cli/src/main.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="one-slice-is-one-feature">

## One slice writes all four of a feature's proofs together

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#one-slice-is-one-feature"><code>testing/one-slice-is-one-feature</code></a>
</div>

One slice takes one feature and writes **all four of its proofs together**. The expensive thing a
session buys is understanding what the feature does at its edges, and the test, the examples, the
bench and the attack all spend that same understanding; split across four sessions it is bought four
times, against a fixed per-session cost that does not shrink with the size of the work.

The generated work chain is one goal per group of features sharing an implementing file set, each
carrying a context manifest naming that file set and each gated by a command that exits non-zero.
Regenerating the chain is how it stays current: a group that owes nothing is left out, so a second
emission writes the chain that is *left*.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/the-four-proofs/#four-proofs" title="A feature is finished when it has a test from both sides, three examples, a measured figure and an attack"><code>testing/four-proofs</code></a> <a href="/docs/rules/testing/the-four-proofs/#a-failing-proof-is-fixed-or-recorded" title="A proof that fails is fixed or recorded as a known gap, and never weakened"><code>testing/a-failing-proof-is-fixed-or-recorded</code></a> <a href="/docs/rules/testing/the-four-proofs/#roster-is-derived" title="The roster of features is read from the registry and the reference chapters, never kept as a list"><code>testing/roster-is-derived</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0134.md">record 0134</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tools/dossier.py"><code>tools/dossier.py</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="examples-live-in-the-repository">

## `docs/examples/` is authoritative and the website mirrors it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#examples-live-in-the-repository"><code>testing/examples-live-in-the-repository</code></a>
</div>

`docs/examples/` is authoritative, and the website's example tree is a mirror rebuilt from it. The
site's own rule is unchanged — tool-owned files are regenerated, human-owned files are never
overwritten — and this simply makes the example tree one of the tool-owned ones.

They live in the repository because the same sweep that tests a feature writes its examples, and a
sweep cannot write into a tree it is not allowed to touch.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/the-four-proofs/#four-proofs" title="A feature is finished when it has a test from both sides, three examples, a measured figure and an attack"><code>testing/four-proofs</code></a> <a href="/docs/rules/testing/the-four-proofs/#proof-attribution" title="An example, an attack and a bench are attributed by path; a test is attributed by a marker"><code>testing/proof-attribution</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0134.md">record 0134</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/examples/README.md"><code>docs/examples/README.md</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="hostile-case-contract">

## A hostile case passes when nothing came apart, and a compile diagnostic is never a pass

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#hostile-case-contract"><code>testing/hostile-case-contract</code></a>
</div>

A hostile case is judged by a contract, never by frozen output. Its assertion is that nothing came
apart: a program that throws, one a limit stops, one that runs out of memory and says so, and one
that simply works are all passes. A panic, an abort, a hang past its timeout, a crash-shaped exit
status and a definite leak under valgrind are not.

The one failure that would otherwise look like a pass is a **compile diagnostic**, and it is checked
for by name — an attack that does not compile was never delivered, and a typo would otherwise
survive every sweep for the rest of this repository's life. Where the refusal *is* the assertion — a
sink handed a tainted value, a capability used without being granted — the case says so, and
compiling cleanly is then what fails it.

Freezing the output instead is refused: every one of these programs is written to produce output
nobody can predict, and a suite whose expectations must be maintained is a suite that gets weakened
until it passes.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/the-four-proofs/#four-proofs" title="A feature is finished when it has a test from both sides, three examples, a measured figure and an attack"><code>testing/four-proofs</code></a> <a href="/docs/rules/testing/the-four-proofs/#a-failing-proof-is-fixed-or-recorded" title="A proof that fails is fixed or recorded as a known gap, and never weakened"><code>testing/a-failing-proof-is-fixed-or-recorded</code></a> <a href="/docs/rules/testing/coverage-and-probes/#capability-closure-test" title="Every member of a capability-bearing class declares a capability or declares none, and there is no allowlist"><code>testing/capability-closure-test</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0134.md">record 0134</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/hostile/README.md"><code>tests/hostile/README.md</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tools/dossier.py"><code>tools/dossier.py</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="four-proofs">

## A feature is finished when it has a test from both sides, three examples, a measured figure and an attack

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#four-proofs"><code>testing/four-proofs</code></a>
</div>

A feature is finished when four artefacts exist for it, not when it works.

**Tests** — its behaviour pinned from Novis *and* from Rust; a `Core` member owes at least one of
each. **Examples** — three small, self-contained, plainly-commented programs a reader learns from.
**Perf** — one measured figure, so a change can be re-measured against it. **Hostile** — one program
written to break it, which passes when the runtime is still standing.

Each tree's own README owns what a file in it *is*, and this rule restates none of them.

Not every kind of feature owes all four: an enum is not attacked and a directive is not benchmarked.
What each kind owes is **data**, overridable per feature, because a policy stated only in prose is a
policy nothing can check. A single feature excused from a single proof is a skip entry carrying
**the reason as its value**, so "this cannot be measured" and "nobody wrote one" never look the same
in the audit.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/the-four-proofs/#roster-is-derived" title="The roster of features is read from the registry and the reference chapters, never kept as a list"><code>testing/roster-is-derived</code></a> <a href="/docs/rules/testing/the-four-proofs/#proof-attribution" title="An example, an attack and a bench are attributed by path; a test is attributed by a marker"><code>testing/proof-attribution</code></a> <a href="/docs/rules/testing/measuring-performance/#member-perf-ledger" title="A member's figure is Novis against Novis, fingerprinted, and re-measured only when its implementing file moves"><code>testing/member-perf-ledger</code></a> <a href="/docs/rules/testing/the-four-proofs/#hostile-case-contract" title="A hostile case passes when nothing came apart, and a compile diagnostic is never a pass"><code>testing/hostile-case-contract</code></a> <a href="/docs/rules/testing/the-four-proofs/#one-slice-is-one-feature" title="One slice writes all four of a feature's proofs together"><code>testing/one-slice-is-one-feature</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0134.md">record 0134</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0117.md">record 0117</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tools/dossier.py"><code>tools/dossier.py</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tools/data/dossier-policy.toml"><code>tools/data/dossier-policy.toml</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="roster-is-derived">

## The roster of features is read from the registry and the reference chapters, never kept as a list

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#roster-is-derived"><code>testing/roster-is-derived</code></a>
</div>

The roster of features is read from live sources and kept nowhere. The registry's own machine-
readable answer names every registered class, member, exception, enum, interface and directive, so a
member that is not implemented is owed no proofs and one that lands is owed them at once. Every `#`
heading of the language and tool reference chapters names one language or tool feature, and those
chapters already state every rule the shipped compiler has, which makes their headings a roster that
maintains itself.

A source that stops naming a feature stops owing proofs for it; one that starts naming a new feature
owes them on the next sweep. Nobody edits a list, so no list is ever stale — a hand-kept roster is
wrong within a day of an unattended run, and it under-reports silently, which is the failure mode
that looks like success.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/the-four-proofs/#four-proofs" title="A feature is finished when it has a test from both sides, three examples, a measured figure and an attack"><code>testing/four-proofs</code></a> <a href="/docs/rules/testing/the-four-proofs/#proof-attribution" title="An example, an attack and a bench are attributed by path; a test is attributed by a marker"><code>testing/proof-attribution</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0134.md">record 0134</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0117.md">record 0117</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tools/dossier.py"><code>tools/dossier.py</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="proof-attribution">

## An example, an attack and a bench are attributed by path; a test is attributed by a marker

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#proof-attribution"><code>testing/proof-attribution</code></a>
</div>

An example, an attack and a bench are attributed by **where they sit**: the trees share one relative
path per feature, so nothing has to be registered anywhere.

A test cannot work that way — a case lives where its suite wants it, and one case often pins several
features — so a test is attributed by a comment naming what it covers, written in a `.nvst` case's
file block or above a Rust test function. For a `Core` member the scan additionally credits a case
that plainly calls it by its written `::` spelling, which is what lets the cases written before this
rule count without being rewritten.

That written spelling is the only inference made. Crediting a bare `->method(` call to every class a
case happens to name is unsound rather than merely loose, and no tightening fixes it without a type
checker.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/the-four-proofs/#four-proofs" title="A feature is finished when it has a test from both sides, three examples, a measured figure and an attack"><code>testing/four-proofs</code></a> <a href="/docs/rules/testing/the-four-proofs/#roster-is-derived" title="The roster of features is read from the registry and the reference chapters, never kept as a list"><code>testing/roster-is-derived</code></a> <a href="/docs/rules/testing/the-four-proofs/#examples-live-in-the-repository" title="docs/examples/ is authoritative and the website mirrors it"><code>testing/examples-live-in-the-repository</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0134.md">record 0134</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tools/dossier.py"><code>tools/dossier.py</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-failing-proof-is-fixed-or-recorded">

## A proof that fails is fixed or recorded as a known gap, and never weakened

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#a-failing-proof-is-fixed-or-recorded"><code>testing/a-failing-proof-is-fixed-or-recorded</code></a>
</div>

An attack that breaks the member it was written against, and an example that disagrees with the
binary, are the program working. There are two answers and no third.

**Fix it**, in the slice that found it, with a case pinning the corrected behaviour. This is the
default, and most findings are small. **Record it**, when the fix is genuinely larger than a slice:
an entry in the owning crate's `# Known gaps` section, plus a marker on the proof naming that file.

A marked proof counts as a known gap rather than a failure, so a long unattended run is not stopped
by one bug it cannot fix. Two rules keep that from becoming a way to make anything green: the marker
must name a file that really carries such a section, so recording a bug means writing it where the
crate's own readers will find it; and **a marked proof that passes fails the sweep**, so removing
the marker is part of whatever fix eventually lands.

**Weakening the proof is not one of the two.** Softening the attack, re-blessing the example or
skipping the feature each turn a finding into a green check, which is the single outcome this rule
exists to prevent. A skip is for a proof that *cannot exist*, never for one that fails.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/the-four-proofs/#four-proofs" title="A feature is finished when it has a test from both sides, three examples, a measured figure and an attack"><code>testing/four-proofs</code></a> <a href="/docs/rules/testing/the-four-proofs/#hostile-case-contract" title="A hostile case passes when nothing came apart, and a compile diagnostic is never a pass"><code>testing/hostile-case-contract</code></a> <a href="/docs/rules/testing/the-four-proofs/#one-slice-is-one-feature" title="One slice writes all four of a feature's proofs together"><code>testing/one-slice-is-one-feature</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0134.md">record 0134</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tools/dossier.py"><code>tools/dossier.py</code></a></dd></div></dl>

</div>
