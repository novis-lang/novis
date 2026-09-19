---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "What the toolchain is"
description: "No REPL, reflection and parsing built in, and the two claims the project will not make against Python."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/tooling/
  label: "Tooling"
next:
  link: /docs/rules/tooling/the-terminal/
  label: "The terminal and command-line programs"
---

<p class="nv-section-lead">No REPL, reflection and parsing built in, and the two claims the project will not make against Python.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">4</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">4</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">3</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#shebang-opens-code-mode">A file whose first two bytes are <code>#!</code> starts in code mode, and its first line is trivia</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#no-repl">There is no REPL and none is planned; <code>nvs run</code>, <code>nvs test</code> and <code>Core\Debug::dump</code> are the answer</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#reflection-and-source-parsing-are-core-features">Reflection and source parsing are built into <code>Core</code>, and <code>Core\Ast</code> is the compiler's own parser</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#python-claims">Against Python, Novis claims the tool that gets handed over; &quot;faster than Python&quot; and &quot;replaces Python&quot; are forbidden</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="shebang-opens-code-mode">

## A file whose first two bytes are `#!` starts in code mode, and its first line is trivia

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#shebang-opens-code-mode"><code>tooling/shebang-opens-code-mode</code></a>
</div>

```
#!/usr/bin/env nvs
Core\Cli::write("hello\n");
```

**The trigger is exact:** the bytes `#!` at offset 0. Line 1, up to and including its first `\n`, is
**trivia** — not a token, not emitted, preserved by the formatter and seen by an editor as a comment.
**The file then continues in code mode**, exactly as if `<?nvs` stood there. Nothing else changes:
`?>` still switches to text mode and writes literal bytes to standard output, and a later `<?nvs`
reopens code mode ([`statements/nvs-is-the-only-open-tag`](/docs/rules/statements/names-and-require/#nvs-is-the-only-open-tag "<?nvs is the only code-mode open tag; <?php is refused")).

**`#!` anywhere but offset 0 is ordinary text**, in either mode, with no lookahead and no special
case. A byte-order mark before it therefore defeats the shebang and the file has none — PHP's
long-standing behaviour too, left as-is rather than repaired, because inventing one rule for one
marker is how a parser acquires the heuristics [`errors/ambiguous-input-refused`](/docs/rules/errors/ambiguous-input/#ambiguous-input-refused "Ambiguous input is refused whole, never repaired") forbids.

**An `<?nvs` in a shebang file, before any `?>`, is `E0009`** — "this file opens with `#!` and is
already in code mode; remove the `<?nvs`" — rather than a lex error naming something the author did
not write. The reverse, a shebang file that never wanted code mode, is not a shape anyone writes and
gets no rule.

The line is trivia on every platform, so one file runs as `./app` on Unix and as `nvs app.nvs` on
Windows with no edit; Windows gains no kernel shebang support, and its distribution answer is the
single-file executable. This is one lexer branch at offset 0: no parser rule, HIR shape or runtime
behaviour changes.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A <code>#!</code> first line is stripped by PHP but the file still needs <code>&lt;?php</code> on line 2; here the shebang itself opens code mode, and a <code>&lt;?nvs</code> after it is E0009</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/statements/names-and-require/#nvs-is-the-only-open-tag" title="&lt;?nvs is the only code-mode open tag; &lt;?php is refused"><code>statements/nvs-is-the-only-open-tag</code></a> <a href="/docs/rules/statements/by-reference-and-exit/#exit-is-the-only-termination-keyword" title="exit is the only process-termination keyword; die is refused"><code>statements/exit-is-the-only-termination-keyword</code></a> <a href="/docs/rules/errors/ambiguous-input/#ambiguous-input-refused" title="Ambiguous input is refused whole, never repaired"><code>errors/ambiguous-input-refused</code></a> <a href="/docs/rules/packaging/single-file-builds/#nvs-build-compile-appends-the-program-to-a-copy-of-the-host" title="nvs build --compile appends a program to a copy of the nvs binary, and rebundling is running it again"><code>packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host</code></a> <a href="/docs/rules/ide/the-resilient-parse/#one-grammar-one-tree" title="The resilient parse is the one grammar's AST plus a trivia layer and an offset index, never a second tree"><code>ide/one-grammar-one-tree</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0100.md">record 0100</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0049.md">record 0049</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0095.md">record 0095</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-shebang-line-opens-code.nvst"><code>tests/conformance/core/a-shebang-line-opens-code.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="no-repl">

## There is no REPL and none is planned; `nvs run`, `nvs test` and `Core\Debug::dump` are the answer

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#no-repl"><code>tooling/no-repl</code></a>
</div>

`nvs` gains no `repl` subcommand and no interactive evaluator, and the subcommand roster is not
reopened for one. A REPL needs three things that are each a language question disguised as a tool: a
top-level scope that survives between inputs, where everything is a class member and there is no top
level to bind into ([`classes/no-free-functions-or-constants`](/docs/rules/classes/declaring-a-class/#no-free-functions-or-constants "No function and no const may be declared outside a class body")); redefinition of a class or member
already compiled, where an artifact is keyed by content hash and redefinition is a cache
invalidation, not an edit; and a printed representation of every value, against
[`classes/definite-property-initialization`](/docs/rules/classes/declaring-a-class/#definite-property-initialization "Every property a class declares is definitely assigned on every path out of every constructor"). Answering them would put a second, looser set of
rules beside the one every compiled program obeys — the shape
[`statements/nothing-gets-a-second-name`](/docs/rules/statements/names-and-require/#nothing-gets-a-second-name "A declaration is reachable under exactly the name it was declared with") rules against.

What exists instead, and what the documentation points at when the question is asked:

- **`nvs run file.nvs`** for a script, made cheap by the on-disk artifact cache — the second run of an
  unchanged file compiles nothing.
- **`nvs test`** ([`testing/test-attribute`](/docs/rules/testing/writing-a-test/#test-attribute "A test is a method marked #[Test], and its table is built while compiling")) for the "poke at it until it works" loop, which is
  what a REPL is used for most of the time and which leaves something behind afterwards.
- **`Core\Debug::dump`** ([`errors/debug-dump`](/docs/rules/errors/diagnostics-and-logging/#debug-dump "A dump goes to the log, and reaches a response body only in development")) for looking at a value.

This is a **decision, not a gap**, and [`tooling/python-claims`](/docs/rules/tooling/what-the-toolchain-is/#python-claims "Against Python, Novis claims the tool that gets handed over; faster than Python and replaces Python are forbidden") requires it to be said out loud
wherever Novis is compared to Python. It reopens only on evidence that "what is this value" costs a
build, and what would be built then is a debugger-shaped inspector over a paused isolate, not a
general evaluator.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>php -a</code>; the loop an interactive shell serves is <code>nvs run</code> over a cached artifact and <code>nvs test</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/what-the-toolchain-is/#python-claims" title="Against Python, Novis claims the tool that gets handed over; faster than Python and replaces Python are forbidden"><code>tooling/python-claims</code></a> <a href="/docs/rules/testing/feature-proofs/#nvst-is-separate" title=".nvst proves the language; #[Test] is how a program written in Novis tests itself"><code>testing/nvst-is-separate</code></a> <a href="/docs/rules/testing/writing-a-test/#test-attribute" title="A test is a method marked #[Test], and its table is built while compiling"><code>testing/test-attribute</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#debug-dump" title="A dump goes to the log, and reaches a response body only in development"><code>errors/debug-dump</code></a> <a href="/docs/rules/classes/declaring-a-class/#no-free-functions-or-constants" title="No function and no const may be declared outside a class body"><code>classes/no-free-functions-or-constants</code></a> <a href="/docs/rules/classes/declaring-a-class/#definite-property-initialization" title="Every property a class declares is definitely assigned on every path out of every constructor"><code>classes/definite-property-initialization</code></a> <a href="/docs/rules/statements/names-and-require/#nothing-gets-a-second-name" title="A declaration is reachable under exactly the name it was declared with"><code>statements/nothing-gets-a-second-name</code></a> <a href="/docs/rules/packaging/the-artifact-cache/#an-artifact-is-one-immutable-content-addressed-file" title="A compiled unit is one immutable file whose address is its content and its environment"><code>packaging/an-artifact-is-one-immutable-content-addressed-file</code></a> <a href="/docs/rules/packaging/running-as-a-service/#the-installer-is-a-sink" title="The service installer fails closed: a closed serve/run allowlist, no relative path, no argv without --config, no install whose output goes nowhere, no password on a command line"><code>packaging/the-installer-is-a-sink</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0100.md">record 0100</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0011.md">record 0011</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0042.md">record 0042</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0022.md">record 0022</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0015.md">record 0015</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0092.md">record 0092</a></dd></div></dl>

</div>

<div class="nv-rule" id="reflection-and-source-parsing-are-core-features">

## Reflection and source parsing are built into `Core`, and `Core\Ast` is the compiler's own parser

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#reflection-and-source-parsing-are-core-features"><code>tooling/reflection-and-source-parsing-are-core-features</code></a>
</div>

`Core\Reflect` and `Core\Ast` are built-in `Core` domain classes, present in every Novis program with
nothing to install: read-only structural introspection over a program's own classes, interfaces,
enums, functions, properties, constants, attributes and parameters on one side, and a parser for
Novis source text on the other. Neither is an extension a deployment might lack, because leaving
either to userland is what produces PHP's split — reflection native and mature, a real syntax tree
only from `nikic/php-parser` or a PECL extension whose grammar drifts from the engine's.

**There is one parser.** `Core\Ast::parse` calls the same lexer and parser the compiler runs, so a
construct that compiles parses identically at run time, a construct the compiler rejects is rejected
identically, and a linter, a codemod or a formatter is a program any Novis user can write rather than
a privilege of the toolchain's own Rust ([`core-classes/ast-is-inert`](/docs/rules/core-classes/regex-html-and-introspection/#ast-is-inert "Core\Ast runs the compiler's own parser and hands back typed, inert nodes")).

Two invariants keep both safe. A reflective call or write runs the visibility check and the property
observer ordinary code at that site would face, and there is no `setAccessible(true)`
([`security/reflection-enforces-visibility`](/docs/rules/security/closed-doors/#reflection-enforces-visibility "A reflective call faces exactly the visibility check ordinary code at that site would, and there is no setAccessible")). A parsed tree is inert typed data with no path back
into execution, because `eval` does not exist. Neither touches the filesystem, the network or another
process, so neither needs a capability grant ([`security/reflection-needs-no-capability`](/docs/rules/security/closed-doors/#reflection-needs-no-capability "Reflection and AST parsing are capability-free, because neither reaches the world outside the process")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>Core\Ast</code> is built in where PHP needs <code>nikic/php-parser</code> or the PECL <code>ast</code> extension, and there is no <code>setAccessible(true)</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/regex-html-and-introspection/#reflect" title="Core\Reflect is read-only structural introspection, and it is a first-class feature rather than an extension"><code>core-classes/reflect</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#ast-is-inert" title="Core\Ast runs the compiler's own parser and hands back typed, inert nodes"><code>core-classes/ast-is-inert</code></a> <a href="/docs/rules/security/closed-doors/#reflection-enforces-visibility" title="A reflective call faces exactly the visibility check ordinary code at that site would, and there is no setAccessible"><code>security/reflection-enforces-visibility</code></a> <a href="/docs/rules/security/closed-doors/#reflection-needs-no-capability" title="Reflection and AST parsing are capability-free, because neither reaches the world outside the process"><code>security/reflection-needs-no-capability</code></a> <a href="/docs/rules/enums/#reflection" title="Reflection describes an enum's shape and grants it nothing a class has"><code>enums/reflection</code></a> <a href="/docs/rules/ide/the-language-server/#every-feature-is-staged-behind-its-dependency" title="The VS Code client goes as deep as the editor allows, and each feature waits for the language or runtime piece it needs rather than shipping as a stub"><code>ide/every-feature-is-staged-behind-its-dependency</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0019.md">record 0019</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/core-ast-parse-answers-the-compilers-own-tree.nvst"><code>tests/conformance/core/core-ast-parse-answers-the-compilers-own-tree.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/core-ast-parse-stops-where-the-compiler-stops.nvst"><code>tests/conformance/core/core-ast-parse-stops-where-the-compiler-stops.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/reflect-forclass-reaches-a-description-by-name-and-absence-is-null.nvst"><code>tests/conformance/core/reflect-forclass-reaches-a-description-by-name-and-absence-is-null.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/reflect-call-runs-a-public-method-and-refuses-a-private-one.nvst"><code>tests/conformance/core/reflect-call-runs-a-public-method-and-refuses-a-private-one.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="python-claims">

## Against Python, Novis claims the tool that gets handed over; "faster than Python" and "replaces Python" are forbidden

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#python-claims"><code>tooling/python-claims</code></a>
</div>

The claim Novis makes against Python is the one Python is worst at: **the tool that gets handed to
somebody else.** It is [`programs/audience`](/docs/rules/programs/claims-and-priorities/#audience "Novis is built for web applications of every kind, with command-line tooling second")'s existing purchase pointed at the command line — a
second audience below the first and never above it — and it adds no priority, reorders nothing, and
schedules no milestone of its own.

**Permitted:** that a Novis CLI program ships as one file with no interpreter, virtualenv or package
install; that argument parsing, help, completions, colour, prompts and progress are in the binary;
that any function may suspend, so there is no `async` split through the library; that shelling out
through a string ([`core-classes/process-is-argv-only`](/docs/rules/core-classes/processes-and-files/#process-is-argv-only "Core\Process is the one way to run another program, and there is no shell string anywhere in it")), leaking a secret
([`security/secret-qualifier`](/docs/rules/security/secrets/#secret-qualifier "secret is a second, independent compile-time qualifier, written before tainted and in that order alone")) and interpolating a query ([`security/tainted-qualifier`](/docs/rules/security/tainted-data/#tainted-qualifier "tainted is a compile-time qualifier on string, bytes and a shape of them, spellable in any declaration and erased before codegen")) are
compile errors; and any **measured** figure from the userland suite
([`tooling/bench-engine-list-is-data`](/docs/rules/tooling/benchmarks-and-telemetry/#bench-engine-list-is-data "The benchmark engine list is data, total is the CLI headline, and a missing engine never turns a build red")), quoted with its engine, mode and host.

**Forbidden in any document, error message, `--help` text or landing page:** "faster than Python",
"replaces Python", "Python without the GIL", "a typed Python", or any phrasing implying a Python
program, script or package runs, converts or ports. There is no `nvs convert` for Python and none is
planned — the rule table is a PHP table and gains no second language. The rule and its reason are
[`programs/three-claims`](/docs/rules/programs/claims-and-priorities/#three-claims "Novis claims three things, and faster than PHP is not one of them")'s: a claim a reader can test and find false costs more than the
adoption it buys.

**Required wherever the comparison is made at all:** that Novis has no REPL ([`tooling/no-repl`](/docs/rules/tooling/what-the-toolchain-is/#no-repl "There is no REPL and none is planned; nvs run, nvs test and Core\Debug::dump are the answer")),
and no numeric or machine-learning stack and no route to one ([`security/no-ffi`](/docs/rules/security/closed-doors/#no-ffi "No userland mechanism loads native code into the process")).

No check enforces a forbidden phrasing and none should; the rule exists so a reviewer has something
to point at.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/programs/claims-and-priorities/#audience" title="Novis is built for web applications of every kind, with command-line tooling second"><code>programs/audience</code></a> <a href="/docs/rules/programs/claims-and-priorities/#three-claims" title="Novis claims three things, and faster than PHP is not one of them"><code>programs/three-claims</code></a> <a href="/docs/rules/programs/claims-and-priorities/#no-compatibility-promise" title="The PHP-shaped syntax is an on-ramp, and no document may promise compatibility"><code>programs/no-compatibility-promise</code></a> <a href="/docs/rules/tooling/what-the-toolchain-is/#no-repl" title="There is no REPL and none is planned; nvs run, nvs test and Core\Debug::dump are the answer"><code>tooling/no-repl</code></a> <a href="/docs/rules/tooling/benchmarks-and-telemetry/#bench-engine-list-is-data" title="The benchmark engine list is data, total is the CLI headline, and a missing engine never turns a build red"><code>tooling/bench-engine-list-is-data</code></a> <a href="/docs/rules/core-classes/processes-and-files/#process-is-argv-only" title="Core\Process is the one way to run another program, and there is no shell string anywhere in it"><code>core-classes/process-is-argv-only</code></a> <a href="/docs/rules/security/secrets/#secret-qualifier" title="secret is a second, independent compile-time qualifier, written before tainted and in that order alone"><code>security/secret-qualifier</code></a> <a href="/docs/rules/security/tainted-data/#tainted-qualifier" title="tainted is a compile-time qualifier on string, bytes and a shape of them, spellable in any declaration and erased before codegen"><code>security/tainted-qualifier</code></a> <a href="/docs/rules/security/closed-doors/#no-ffi" title="No userland mechanism loads native code into the process"><code>security/no-ffi</code></a> <a href="/docs/rules/packaging/single-file-builds/#nvs-build-compile-appends-the-program-to-a-copy-of-the-host" title="nvs build --compile appends a program to a copy of the nvs binary, and rebundling is running it again"><code>packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host</code></a> <a href="/docs/rules/tooling/the-terminal/#commands-are-compiled" title="#[Command] builds the command table while compiling, and a duplicate name or an unbindable option fails the build"><code>tooling/commands-are-compiled</code></a> <a href="/docs/rules/concurrency/tasks/#one-scheduler" title="Concurrency is the Core\Task roster over the runtime's own single scheduler"><code>concurrency/one-scheduler</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0100.md">record 0100</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0080.md">record 0080</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0044.md">record 0044</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0048.md">record 0048</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0052.md">record 0052</a></dd></div></dl>

</div>
