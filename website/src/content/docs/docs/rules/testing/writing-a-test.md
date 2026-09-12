---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Writing a test"
description: "A test is a method with an attribute, running in its own isolate, asserting on two values of one type."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/testing/
  label: "Testing"
next:
  link: /docs/rules/testing/doubles-and-the-runner/
  label: "Doubles, determinism and the runner"
---

<p class="nv-section-lead">A test is a method with an attribute, running in its own isolate, asserting on two values of one type.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">10</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">7</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">3</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">9</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#test-attribute">A test is a method marked <code>#[Test]</code>, and its table is built while compiling</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#tests-never-reach-a-build"><code>nvs run</code> and <code>nvs build</code> lower no test, and <code>nvs check</code> type-checks every one</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#isolate-per-test">Every test runs in its own isolate and shares nothing but compiled code with its siblings</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#private-in-the-same-file">A test reaches <code>private</code> members of classes declared in its own file, and nowhere else</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#assertions-are-typed">An assertion names its subject first and compares two values of one type</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#failure-ledger">A failed assertion is a catchable <code>Throwable</code> and a ledger entry the test cannot erase</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#constructor-is-setup">The constructor is the test's setup, and there is no second before-hook</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#after-hook"><code>#[After]</code> is for state that outlives the isolate, and nothing else needs teardown</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#fixtures"><code>#[Fixture]</code> is built once per class and injected into a test by the type it returns</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#data-rows"><code>#[TestWith]</code> is matched against the parameters by name and by type, and each row is its own case</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="test-attribute">

## A test is a method marked `#[Test]`, and its table is built while compiling

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#test-attribute"><code>testing/test-attribute</code></a>
</div>

A test is an ordinary `public` method carrying `#[Test]`, on an ordinary class. There is no naming
convention, no base class to extend and no interface to implement: nothing about a test is inferred
from spelling.

The compiler collects every marked method into a table while it checks, keyed by class label, the
way the route table is built — so discovery costs nothing at startup, and four malformed shapes are
compile errors rather than tests that silently never run: two methods of one class sharing a name, a
`static` method, one returning anything but `void`, and one that is not `public`. The runner
constructs the class and calls the member, so each of those is one question about what shape a test
method has, asked once. Each row also carries the span of its method's **name** — where every refusal
about that test already points — because that is the one fact about a test nothing downstream can
recompute, and it is what locates a test in a report and in an editor's test tree.

`#[Test]`'s payload is an options bag — `skip`, `at`, `seed`, `db`, `server`, `retries`,
`because` — every field optional and every field checked at its own type. An option the roster does
not name is refused, one written twice is refused, and `skip: true` is a `bool` where a `string` is
declared.

The attribute is matched **nominally**, by resolved name: `#[Core\Test]` and a `use`d `#[Test]` are
one attribute, and a userland `class Test` is never it. The marker and the assertions are one class,
so a single `use Core\Test;` places both.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no test framework to install, no naming convention and no base class; a malformed test is a compile error rather than a case that silently never runs</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/writing-a-test/#isolate-per-test" title="Every test runs in its own isolate and shares nothing but compiled code with its siblings"><code>testing/isolate-per-test</code></a> <a href="/docs/rules/testing/writing-a-test/#fixtures" title="#[Fixture] is built once per class and injected into a test by the type it returns"><code>testing/fixtures</code></a> <a href="/docs/rules/testing/writing-a-test/#data-rows" title="#[TestWith] is matched against the parameters by name and by type, and each row is its own case"><code>testing/data-rows</code></a> <a href="/docs/rules/testing/doubles-and-the-runner/#runner-is-strict" title="A test that asserts nothing fails, a skip states a reason, and a retry is reported flaky rather than green"><code>testing/runner-is-strict</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0077.md">record 0077</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0046.md">record 0046</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0071.md">record 0071</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0011.md">record 0011</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0029.md">record 0029</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0172.md">record 0172</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-test-method-is-a-public-void-instance-method.nvst"><code>tests/conformance/reject/a-test-method-is-a-public-void-instance-method.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-test-attribute-payload-is-checked-against-its-option-shape.nvst"><code>tests/conformance/reject/a-test-attribute-payload-is-checked-against-its-option-shape.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-test-attribute-builds-a-table-the-runner-reports.nvst"><code>tests/conformance/lang/a-test-attribute-builds-a-table-the-runner-reports.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/testing.rs"><code>crates/nvs-types/tests/testing.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="tests-never-reach-a-build">

## `nvs run` and `nvs build` lower no test, and `nvs check` type-checks every one

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#tests-never-reach-a-build"><code>testing/tests-never-reach-a-build</code></a>
</div>

A `#[Test]` method may be declared in any file — beside the class it tests, or in a separate tree.
What makes a method a test is the attribute, never the path.

`nvs test` compiles and runs them. `nvs run` and `nvs build` do not lower them at all: a test
method, its fixtures, its data rows, its assertion messages and its doubles are absent from a built
artifact. Production pays nothing for them, and no test surface is reachable at run time.

`nvs check` does type-check test code. That is the one place tests and non-tests are treated alike,
and deliberately so: a test cannot rot silently while the code around it changes.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Test code is absent from a built artifact by construction, rather than excluded by whatever the deployment happened to copy</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/writing-a-test/#test-attribute" title="A test is a method marked #[Test], and its table is built while compiling"><code>testing/test-attribute</code></a> <a href="/docs/rules/testing/doubles-and-the-runner/#determinism-declared-on-the-test" title="at: and seed: fix the clock and the generator of the test's own isolate"><code>testing/determinism-declared-on-the-test</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a></dd></div></dl>

</div>

<div class="nv-rule" id="isolate-per-test">

## Every test runs in its own isolate and shares nothing but compiled code with its siblings

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#isolate-per-test"><code>testing/isolate-per-test</code></a>
</div>

Each test runs on an isolate of its own: it shares compiled code with its siblings and nothing
else — not a static, not a cache entry, not an open handle. A static one test increments reads its
declared initial value in the next, always, with no flag to change it
([`statements/an-isolate-has-its-own-statics`](/docs/rules/statements/where-state-lives/#an-isolate-has-its-own-statics "An isolate's class statics are its own, and a child cannot reach its parent's")).

This is not a hardening measure bolted onto a sequential runner; it is what makes a parallel one
possible at all, because an isolate is cheap enough to be the default rather than a per-case escape
hatch. Two consequences follow and are answered elsewhere: shared expensive setup needs its own
mechanism ([`testing/fixtures`](/docs/rules/testing/writing-a-test/#fixtures "#[Fixture] is built once per class and injected into a test by the type it returns")), and execution order carries no meaning, so the report is in
declaration order regardless of what finished when ([`testing/runner-is-strict`](/docs/rules/testing/doubles-and-the-runner/#runner-is-strict "A test that asserts nothing fails, a skip states a reason, and a retry is reported flaky rather than green")).

Each test isolate spends its parent's budget, so a suite has the same enforceable ceiling on memory
and CPU that a request has, and a runaway test is terminated rather than left to consume the
machine.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Total isolation is the default rather than a per-case escape hatch; a static one test wrote is never visible to the next</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/writing-a-test/#test-attribute" title="A test is a method marked #[Test], and its table is built while compiling"><code>testing/test-attribute</code></a> <a href="/docs/rules/testing/writing-a-test/#fixtures" title="#[Fixture] is built once per class and injected into a test by the type it returns"><code>testing/fixtures</code></a> <a href="/docs/rules/testing/doubles-and-the-runner/#runner-is-strict" title="A test that asserts nothing fails, a skip states a reason, and a retry is reported flaky rather than green"><code>testing/runner-is-strict</code></a> <a href="/docs/rules/statements/where-state-lives/#an-isolate-has-its-own-statics" title="An isolate's class statics are its own, and a child cannot reach its parent's"><code>statements/an-isolate-has-its-own-statics</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0004.md">record 0004</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/runner.rs"><code>crates/nvs-cli/src/runner.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="private-in-the-same-file">

## A test reaches `private` members of classes declared in its own file, and nowhere else

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#private-in-the-same-file"><code>testing/private-in-the-same-file</code></a>
</div>

A test method may reach `private` and `protected` members of classes declared **in the same file**.
A test in a separate file is held to the public contract.

This is the narrowest rule that avoids the failure it exists to prevent: a `public` method that
exists only because a test needed to reach it. White-box testing stays possible where the author has
already chosen to put the test next to the code; everything at a distance tests behaviour.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no reflection to reach a private member with, so white-box testing is bought by co-location and by nothing else</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/writing-a-test/#test-attribute" title="A test is a method marked #[Test], and its table is built while compiling"><code>testing/test-attribute</code></a> <a href="/docs/rules/testing/writing-a-test/#tests-never-reach-a-build" title="nvs run and nvs build lower no test, and nvs check type-checks every one"><code>testing/tests-never-reach-a-build</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a></dd></div></dl>

</div>

<div class="nv-rule" id="assertions-are-typed">

## An assertion names its subject first and compares two values of one type

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#assertions-are-typed"><code>testing/assertions-are-typed</code></a>
</div>

The assertion surface is static members on `Core\Test`, each taking its **subject first**:
`assertEquals($actual, $expected)`, with one trailing options shape, nothing mutating and failure
throwing. Both operands of an equality member are one type variable, so comparing an `int` against a
`string` does not compile. Because reversing the two is the commonest mistake in the ecosystem this
language is migrated from, every failure report labels the sides by name rather than by position, so
a reversed call still reads correctly.

Three equality members, and which one was asked for is visible at the call site. `assertSame` is
identity. `assertEquals` compares values — scalars natively, objects only through `Comparable`.
`assertEqualsDeep` is an explicit structural walk of properties, arrays and shapes, with a diff.
`assertEquals` on an object with no `compareTo` names `assertEqualsDeep` rather than falling back to
a property walk: a comparison that quietly changes meaning when a class gains a field is a bug found
years later.

The roster is wide on purpose. Ranking the right member by the subject's type at the call site is
the language server's job, and not a reason to reshape the API into a chain.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>The argument order is <code>(actual, expected)</code>, the reverse of PHPUnit's, and comparing an <code>int</code> against a <code>string</code> does not compile</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/writing-a-test/#failure-ledger" title="A failed assertion is a catchable Throwable and a ledger entry the test cannot erase"><code>testing/failure-ledger</code></a> <a href="/docs/rules/testing/doubles-and-the-runner/#runner-is-strict" title="A test that asserts nothing fails, a skip states a reason, and a retry is reported flaky rather than green"><code>testing/runner-is-strict</code></a> <a href="/docs/rules/testing/doubles-and-the-runner/#inline-snapshots" title="A snapshot is a literal in the test body, and the updater rewrites that literal and nothing else"><code>testing/inline-snapshots</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0013.md">record 0013</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0090.md">record 0090</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/an-assertion-compares-its-subject-against-its-expectation.nvst"><code>tests/conformance/core/an-assertion-compares-its-subject-against-its-expectation.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-predicate-assertion-judges-the-one-subject-its-type-admits.nvst"><code>tests/conformance/core/a-predicate-assertion-judges-the-one-subject-its-type-admits.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/an-assertion-over-a-callable-judges-what-came-back.nvst"><code>tests/conformance/core/an-assertion-over-a-callable-judges-what-came-back.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/test-assert-equals-refuses-an-object-with-no-compare-to.nvst"><code>tests/conformance/core/test-assert-equals-refuses-an-object-with-no-compare-to.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/test-assert-same-and-assert-equals-deep-part-only-at-a-copy.nvst"><code>tests/conformance/core/test-assert-same-and-assert-equals-deep-part-only-at-a-copy.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="failure-ledger">

## A failed assertion is a catchable `Throwable` and a ledger entry the test cannot erase

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#failure-ledger"><code>testing/failure-ledger</code></a>
</div>

`Core\Test\Failure` is an ordinary `Throwable`, and catching one is not merely allowed but
necessary: a composite assertion, a retry wrapper, a soft-assert block and any test *of* an
assertion all need to intercept one.

Every assertion **also** records its outcome into a per-test ledger the test's own code cannot
reach, and the runner reads the ledger rather than the exception state. A body that swallowed its
own failure with `catch (Throwable)` has still failed. There is deliberately no member that reads
the ledger back — that would hand the test the eraser this rule takes away — and the single throw
site records the entry *before* it hands the throw back, so no edge exists on which the throw
happened and the record did not.

`Core\Test::expectFailure(callable)` is the one greppable spelling for a failure consumed on
purpose: it runs the body, requires that it fail, and removes that entry. It decides from the ledger
rather than from what came back, so a body that caught its own failure has still failed; a body in
which nothing failed is itself a failure; and the passing assertions inside a discharged body stay,
because they really ran.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A <code>catch (Throwable)</code> around a failing assertion cannot turn a red test green; the runner reads its own ledger rather than the exception state</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/writing-a-test/#assertions-are-typed" title="An assertion names its subject first and compares two values of one type"><code>testing/assertions-are-typed</code></a> <a href="/docs/rules/testing/doubles-and-the-runner/#runner-is-strict" title="A test that asserts nothing fails, a skip states a reason, and a retry is reported flaky rather than green"><code>testing/runner-is-strict</code></a> <a href="/docs/rules/errors/how-an-error-travels/#throwable-hierarchy" title="A limit report is not a Throwable, and the type checker knows it"><code>errors/throwable-hierarchy</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0020.md">record 0020</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-failed-assertion-is-caught-by-name.nvst"><code>tests/conformance/core/a-failed-assertion-is-caught-by-name.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-failure-consumed-on-purpose-is-spelled-expect-failure.nvst"><code>tests/conformance/core/a-failure-consumed-on-purpose-is-spelled-expect-failure.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/test-expect-failure-discharges-a-failed-assertion-and-refuses-a-passing-body.nvst"><code>tests/conformance/core/test-expect-failure-discharges-a-failed-assertion-and-refuses-a-passing-body.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/test-three-assertions-agree-on-what-reaches-the-ledger.nvst"><code>tests/conformance/core/test-three-assertions-agree-on-what-reaches-the-ledger.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/test-assert-throws-consumes-the-throw-but-not-the-ledger-entry.nvst"><code>tests/conformance/core/test-assert-throws-consumes-the-throw-but-not-the-ledger-entry.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="constructor-is-setup">

## The constructor is the test's setup, and there is no second before-hook

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#constructor-is-setup"><code>testing/constructor-is-setup</code></a>
</div>

The constructor is the test's setup, and a stronger one than a `setUp` method can be: definite
property initialization guarantees every property is assigned before any test body runs. There is no
`#[Before]`, and no all-cases pair either — across isolates those would either lie or need fixtures,
which is the honest spelling of what they were for.

A test class's constructor therefore declares no parameters. One that does is reported as that test
failing rather than pretended past: the runner constructs the class with no arguments, and a test
method's own parameters are filled by [`testing/fixtures`](/docs/rules/testing/writing-a-test/#fixtures "#[Fixture] is built once per class and injected into a test by the type it returns") and [`testing/data-rows`](/docs/rules/testing/writing-a-test/#data-rows "#[TestWith] is matched against the parameters by name and by type, and each row is its own case") instead.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>setUp</code>, <code>setUpBeforeClass</code> or <code>tearDownAfterClass</code>; the constructor is the only one, and every property is assigned before any test body runs</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/writing-a-test/#after-hook" title="#[After] is for state that outlives the isolate, and nothing else needs teardown"><code>testing/after-hook</code></a> <a href="/docs/rules/testing/writing-a-test/#fixtures" title="#[Fixture] is built once per class and injected into a test by the type it returns"><code>testing/fixtures</code></a> <a href="/docs/rules/testing/writing-a-test/#isolate-per-test" title="Every test runs in its own isolate and shares nothing but compiled code with its siblings"><code>testing/isolate-per-test</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0022.md">record 0022</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-runtime/src/dispatch.rs"><code>crates/nvs-runtime/src/dispatch.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="after-hook">

## `#[After]` is for state that outlives the isolate, and nothing else needs teardown

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#after-hook"><code>testing/after-hook</code></a>
</div>

`#[After]` runs after a test and exists only for residue that outlives the isolate — a file, a row,
a remote object. Teardown is otherwise unnecessary here: the isolate dies and takes everything
inside it with it, which is what leaves this marker one narrow job rather than a general hook.

Every such residue is capability-bearing, so there is nothing `#[After]` can usefully do until a
test run can hold a capability.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/writing-a-test/#constructor-is-setup" title="The constructor is the test's setup, and there is no second before-hook"><code>testing/constructor-is-setup</code></a> <a href="/docs/rules/testing/writing-a-test/#isolate-per-test" title="Every test runs in its own isolate and shares nothing but compiled code with its siblings"><code>testing/isolate-per-test</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a></dd></div></dl>

</div>

<div class="nv-rule" id="fixtures">

## `#[Fixture]` is built once per class and injected into a test by the type it returns

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#fixtures"><code>testing/fixtures</code></a>
</div>

`#[Fixture]` marks a `public static` method, and what it supplies is its declared return type. It is
built **once per class**, in the parent, and reaches a test by that test declaring a parameter of
the type it returns. Resolution is by interned type and happens while compiling, so an unsatisfiable
parameter is a diagnostic naming the type it asked for rather than a null at run time. A fixture may
itself declare fixture parameters; a cycle is a compile error naming the whole chain.

Two fixtures of one class returning one type is a duplicate declaration, because resolution is by
type and two answers to one parameter is exactly the ambiguity that refusal prevents. Four
declarations cannot supply a value and are refused where they are written: one that is not `static`,
one that is not `public`, one returning `void`, and a method carrying both this marker and `#[Test]`.

Only a fixture that a test which will actually run asks for is built — a skipped test's fixture is
setup nobody wanted. Resolution runs once the whole class is collected rather than as the walk
descends, because a test may be written above the fixture that supplies it.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Shared setup is resolved by declared type while compiling, so an unsatisfiable parameter is a diagnostic rather than a null at run time</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/writing-a-test/#data-rows" title="#[TestWith] is matched against the parameters by name and by type, and each row is its own case"><code>testing/data-rows</code></a> <a href="/docs/rules/testing/writing-a-test/#constructor-is-setup" title="The constructor is the test's setup, and there is no second before-hook"><code>testing/constructor-is-setup</code></a> <a href="/docs/rules/testing/writing-a-test/#isolate-per-test" title="Every test runs in its own isolate and shares nothing but compiled code with its siblings"><code>testing/isolate-per-test</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0023.md">record 0023</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-fixture-is-built-once-and-injected-by-type.nvst"><code>tests/conformance/lang/a-fixture-is-built-once-and-injected-by-type.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-fixture-supplies-one-type-from-a-public-static-method.nvst"><code>tests/conformance/reject/a-fixture-supplies-one-type-from-a-public-static-method.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-test-parameter-is-supplied-by-a-fixture-or-refused.nvst"><code>tests/conformance/reject/a-test-parameter-is-supplied-by-a-fixture-or-refused.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/testing.rs"><code>crates/nvs-types/tests/testing.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="data-rows">

## `#[TestWith]` is matched against the parameters by name and by type, and each row is its own case

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#data-rows"><code>testing/data-rows</code></a>
</div>

`#[TestWith(...)]` carries a shape literal that the checker matches against the method's parameters
**by name and by type**. It is the one test marker that may repeat on a declaration, and each row is
its own separately reported case, labelled `method#N` in the order the rows are written — so a row
that fails is reported alone rather than stopping the rows after it, and a skip is stated per row,
a row being a test.

Every way a row and a parameter list fail to line up is one refusal, because the fix is the same
every time: write the row against the parameter list. A field naming no parameter, a value that is
not a literal of that parameter's declared type, a row omitting a field its siblings supply, and the
marker written on a method that is no test at all.

Where a row and a fixture could both answer one parameter, the **row** wins: it was written against
this method's own parameter list, while a fixture answers every method of the class at once. Each
row's values are folded to constants in parameter order rather than in written order, because that
is the order the call is made in, and are released when that call returns.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A data provider is a checked attribute payload rather than an <code>array&lt;array&lt;mixed&gt;&gt;</code> a method returns, so a row that does not fit the signature is refused where it is written</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/testing/writing-a-test/#fixtures" title="#[Fixture] is built once per class and injected into a test by the type it returns"><code>testing/fixtures</code></a> <a href="/docs/rules/testing/writing-a-test/#test-attribute" title="A test is a method marked #[Test], and its table is built while compiling"><code>testing/test-attribute</code></a> <a href="/docs/rules/testing/doubles-and-the-runner/#report-formats" title="One verdict, three renderings, a machine format owns stdout alone, and the JSON one locates every test it reports"><code>testing/report-formats</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0046.md">record 0046</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0071.md">record 0071</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-data-row-is-its-own-reported-case.nvst"><code>tests/conformance/lang/a-data-row-is-its-own-reported-case.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-data-row-is-matched-against-the-parameters-by-name-and-by-type.nvst"><code>tests/conformance/reject/a-data-row-is-matched-against-the-parameters-by-name-and-by-type.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/testing.rs"><code>crates/nvs-types/tests/testing.rs</code></a></dd></div></dl>

</div>
