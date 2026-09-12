---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Processes and files"
description: "Running another program is argv only — there is no shell string anywhere — and a temporary directory sweeps itself."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/core-classes/
  label: "The Core classes"
next:
  link: /docs/rules/core-classes/regex-html-and-introspection/
  label: "Regex, HTML and introspection"
---

<p class="nv-section-lead">Running another program is argv only — there is no shell string anywhere — and a temporary directory sweeps itself.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">10</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">8</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">2</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">8</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#cli-arguments"><code>Core\Cli::arguments</code> is how a program reads the words it was started with, at every depth</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#script-args"><code>Core\Script::args</code> is the value the current isolate was spawned with, and <code>null</code> where there was none</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#process-is-argv-only"><code>Core\Process</code> is the one way to run another program, and there is no shell string anywhere in it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#process-run"><code>run</code> waits by suspending the coroutine, and hands back the exit code with both captures as <code>bytes</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#process-options"><code>ProcessOptions</code> carries a working directory, a replaced environment and a timeout, and nothing else</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#process-spawn"><code>spawn</code> answers a handle whose reads and writes suspend, covering <code>proc_open</code> and <code>passthru</code> in one type</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#process-refuses-a-shell-target">A target only a second command-line parser could run is refused, on every platform</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#io-write-stream"><code>Core\IO::writeStream</code> is where every stream reaches disk, and a failed write removes its partial file</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#temporary-dir-sweep">A temporary directory lives under a Novis-owned root and is deleted when its script ends, and the sweep never throws</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#temporary-dir-orphan-sweep">The orphan sweep is keyed on the owner being alive, never on age, and runs in exactly two places</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="cli-arguments">

## `Core\Cli::arguments` is how a program reads the words it was started with, at every depth

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#cli-arguments"><code>core-classes/cli-arguments</code></a>
</div>

`Core\Cli::arguments()` answers the words the program was started with, as an
`array<tainted string>`. It replaces `$argv` and `$argc` in one place: a count is the array's length,
so there is nothing for two spellings to disagree about.

The elements are `tainted`, because a word typed at a shell is user-derived data like any other, and
a sink refuses one. A word that looks like syntax comes back as the value it is, unsplit and
uninterpreted.

Within a request tree the answer is the same at every depth: it reflects how the **process** was
invoked, which is a process-wide fact rather than a per-isolate one, so a spawned isolate neither
fakes nor suppresses it the way it does for request state. A program reading arguments a *request*
supplied wants [`core-classes/script-args`](/docs/rules/core-classes/processes-and-files/#script-args "Core\Script::args is the value the current isolate was spawned with, and null where there was none") instead.

The record this rule comes from specified `args()`/`argc()` and a throw when called while serving
HTTP. The shipped member is `arguments()`, and inside a request it answers empty rather than throwing
— the launcher writes the command line and only the CLI entry point writes one.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>$argv</code> and <code>$argc</code> do not exist, the elements are <code>tainted</code>, and the answer is a process fact rather than a per-isolate one</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/processes-and-files/#script-args" title="Core\Script::args is the value the current isolate was spawned with, and null where there was none"><code>core-classes/script-args</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#session-is-started-explicitly" title="A session is opened by calling Core\Session::start, and there is no ambient session array"><code>core-classes/session-is-started-explicitly</code></a> <a href="/docs/rules/statements/where-state-lives/#no-host-populated-variables" title="No variable is ever populated by the host; every superglobal is a Core class member"><code>statements/no-host-populated-variables</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0012.md">record 0012</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0086.md">record 0086</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0118.md">record 0118</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/cli-arguments-answers-the-words-the-program-was-started-with.nvst"><code>tests/conformance/core/cli-arguments-answers-the-words-the-program-was-started-with.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/cli-arguments-is-empty-for-a-program-started-with-no-words.nvst"><code>tests/conformance/core/cli-arguments-is-empty-for-a-program-started-with-no-words.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/cli-arguments-elements-are-tainted-and-a-sink-refuses-one.nvst"><code>tests/conformance/core/cli-arguments-elements-are-tainted-and-a-sink-refuses-one.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/cli-arguments-hands-back-a-word-that-looks-like-syntax-as-the-value-it-is.nvst"><code>tests/conformance/core/cli-arguments-hands-back-a-word-that-looks-like-syntax-as-the-value-it-is.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="script-args">

## `Core\Script::args` is the value the current isolate was spawned with, and `null` where there was none

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#script-args"><code>core-classes/script-args</code></a>
</div>

`Core\Script::args(): mixed` is the deep-copied value the current isolate was spawned with, and
`null` where there was none — a child spawned without the option, and the root script, which nothing
spawned.

The type is `mixed` rather than `array<mixed>` because a spawn's argument accepts any value that can
cross the boundary, decided at run time, so nothing narrows the option at the call site. The answer
is `null` rather than an empty array because a program that wrote `args: []` said something a program
that wrote no option did not.

This is Novis's own superglobal being retired for the same reason PHP's were, and consistency is the
whole of the reason: an ambient, undeclared variable is the shape being closed, and one the project
introduced itself is no better for having been introduced deliberately. Each isolate's arguments are
its own.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Novis's own <code>$_ARGS</code> superglobal was retired for the same reason PHP's were — a spawned isolate reads its argument through a member</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/processes-and-files/#cli-arguments" title="Core\Cli::arguments is how a program reads the words it was started with, at every depth"><code>core-classes/cli-arguments</code></a> <a href="/docs/rules/statements/where-state-lives/#no-host-populated-variables" title="No variable is ever populated by the host; every superglobal is a Core class member"><code>statements/no-host-populated-variables</code></a> <a href="/docs/rules/statements/where-state-lives/#an-isolate-has-its-own-statics" title="An isolate's class statics are its own, and a child cannot reach its parent's"><code>statements/an-isolate-has-its-own-statics</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0012.md">record 0012</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0023.md">record 0023</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/script-args-are-read.nvst"><code>tests/conformance/core/script-args-are-read.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/script-args-are-each-isolates-own.nvst"><code>tests/conformance/core/script-args-are-each-isolates-own.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/script-args-tell-an-empty-array-from-no-option.nvst"><code>tests/conformance/core/script-args-tell-an-empty-array-from-no-option.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="process-is-argv-only">

## `Core\Process` is the one way to run another program, and there is no shell string anywhere in it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#process-is-argv-only"><code>core-classes/process-is-argv-only</code></a>
</div>

`Core\Process` is the one way to run another program, and it is argv-only. There is no shell-string
form anywhere in it and no flag that turns one on, so `exec`, `system`, `shell_exec`, `passthru`,
`popen`, `proc_open` and backticks all reach one of two members taking a path and an array of
arguments.

The path and every element of the argument array are plain `string`: a `tainted` value needs a
checked conversion or an explicit launderer first, exactly like any other sink. A name the program
did not choose is the whole of what a command injection is, and the array carries no nesting mark of
its own because `array<tainted string>` is simply not `array<string>`.

Running anything at all takes the deny-by-default `process.exec` capability, asked before the target
is looked at, so an ungranted program cannot even learn whether a binary exists.

What this costs is the one case where a shell genuinely was the feature — a pipeline, a glob, a
redirect. Those are written in Novis, or by spawning the shell explicitly and owning the quoting at
that call site.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>exec</code>, <code>system</code>, <code>shell_exec</code>, <code>passthru</code>, <code>popen</code>, <code>proc_open</code> and backticks are all gone, and no flag brings a command line back</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/processes-and-files/#process-run" title="run waits by suspending the coroutine, and hands back the exit code with both captures as bytes"><code>core-classes/process-run</code></a> <a href="/docs/rules/core-classes/processes-and-files/#process-refuses-a-shell-target" title="A target only a second command-line parser could run is refused, on every platform"><code>core-classes/process-refuses-a-shell-target</code></a> <a href="/docs/rules/core-classes/processes-and-files/#process-spawn" title="spawn answers a handle whose reads and writes suspend, covering proc_open and passthru in one type"><code>core-classes/process-spawn</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0044.md">record 0044</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0118.md">record 0118</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/there-is-no-shell-string-form-of-process-run.nvst"><code>tests/conformance/reject/there-is-no-shell-string-form-of-process-run.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/process-exec-is-deny-by-default.nvst"><code>tests/conformance/core/process-exec-is-deny-by-default.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/process-run-asks-for-the-capability-before-it-looks-at-the-target.nvst"><code>tests/conformance/core/process-run-asks-for-the-capability-before-it-looks-at-the-target.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="process-run">

## `run` waits by suspending the coroutine, and hands back the exit code with both captures as `bytes`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#process-run"><code>core-classes/process-run</code></a>
</div>

`Core\Process::run(string $path, array<string> $argv, ProcessOptions $options)` spawns the process,
waits for it to exit, and answers a result carrying the exit code and both captures. Waiting is an
ordinary suspension point on the runtime's stackful coroutines — the same mechanism that lets any
function perform I/O without being marked async — so a slow child ties up one coroutine's stack and
not the worker thread it started on, and other requests on the same core keep making progress.

**Captured stdout and stderr are `bytes`, never `string`.** An arbitrary child's output cannot be
assumed valid UTF-8, so a caller who knows it is text writes `as string`, which throws on invalid
input rather than mangling it into replacement characters. The two captures stay apart, a non-zero
exit keeps both, and a result answers the same thing every time it is asked.

What it spends, per call: the child's whole stdout and stderr, once each, held for as long as the
program holds the result, plus one object allocation — charged to the request that asked. That
capture is bounded by the request's existing `max_output` directive rather than by a cap of this
member's own: the same number the response ceiling is, read once per call, and a child that keeps
writing past it is killed while `run` throws.

**That refusal is catchable, and is not [`errors/on-limit`](/docs/rules/errors/the-escalation-ladder/#on-limit "Tier 1 — a resource limit reaches the request that spent it")'s `FATAL`.** Nothing reached the
response, so the request exceeded no limit — the member declined to hold more than the request is
allowed to produce, which is a refusal in [`security/denial-is-a-runtime-error`](/docs/rules/security/scopes-and-denial/#denial-is-a-runtime-error "A denied capability is a catchable RuntimeError naming the capability, never a fatal")'s shape. A caller
who ran a chattier child than it meant to can catch it and run a different one. `Core\IO::read` holds
a file to the same directive out of the same pair of methods, so the two answer a program alike.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Captured output is <code>bytes</code>, so text costs an <code>as string</code> that throws on invalid UTF-8 rather than mangling it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/processes-and-files/#process-is-argv-only" title="Core\Process is the one way to run another program, and there is no shell string anywhere in it"><code>core-classes/process-is-argv-only</code></a> <a href="/docs/rules/core-classes/processes-and-files/#process-options" title="ProcessOptions carries a working directory, a replaced environment and a timeout, and nothing else"><code>core-classes/process-options</code></a> <a href="/docs/rules/types/text-and-literal-types/#bytes" title="bytes is a primitive peer to string for data that carries no encoding"><code>types/bytes</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0044.md">record 0044</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0009.md">record 0009</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/process-a-completed-run-answers-the-status-the-child-chose.nvst"><code>tests/conformance/core/process-a-completed-run-answers-the-status-the-child-chose.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/process-a-completed-runs-two-streams-stay-apart.nvst"><code>tests/conformance/core/process-a-completed-runs-two-streams-stay-apart.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/process-a-non-zero-status-keeps-both-captures.nvst"><code>tests/conformance/core/process-a-non-zero-status-keeps-both-captures.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/process-a-capture-is-whole-not-a-pipes-worth.nvst"><code>tests/conformance/core/process-a-capture-is-whole-not-a-pipes-worth.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/process-a-child-that-writes-nothing-answers-two-empty-captures.nvst"><code>tests/conformance/core/process-a-child-that-writes-nothing-answers-two-empty-captures.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="process-options">

## `ProcessOptions` carries a working directory, a replaced environment and a timeout, and nothing else

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#process-options"><code>core-classes/process-options</code></a>
</div>

`ProcessOptions` carries three fields and no more: a working directory, an environment, and a
timeout.

`env`, when given, **replaces** the child's environment entirely rather than merging with the
parent's — explicit replacement is simpler to reason about than merge semantics. Every key and value
is plain `string`, so an API key held as a `secret` needs [`core-classes/secret-reveal`](/docs/rules/core-classes/regex-html-and-introspection/#secret-reveal "Core\Secret::reveal is the one named way out of secret, and it carries a written reason") first;
this is a new sink reusing an existing escape hatch, not a new mechanism.

`timeout` reuses the existing safepoint-driven cancellation — the same poll that already cancels a
request — rather than a bespoke process-only timer. On expiry the child is killed and the suspended
coroutine resumes into a throw naming the timeout.

**Not shipped.** `crates/nvs-stdlib/src/process.rs` registers `run` with a path and an argument array
and nothing else; there is no options type, so a child inherits the environment, runs in the calling
process's directory, and is bounded only by the request's own wall-clock deadline.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A supplied environment replaces the parent's rather than merging with it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/processes-and-files/#process-run" title="run waits by suspending the coroutine, and hands back the exit code with both captures as bytes"><code>core-classes/process-run</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#secret-reveal" title="Core\Secret::reveal is the one named way out of secret, and it carries a written reason"><code>core-classes/secret-reveal</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0044.md">record 0044</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a></dd></div></dl>

</div>

<div class="nv-rule" id="process-spawn">

## `spawn` answers a handle whose reads and writes suspend, covering `proc_open` and `passthru` in one type

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#process-spawn"><code>core-classes/process-spawn</code></a>
</div>

`Core\Process::spawn` takes the same path, argument array and options as `run` and answers a handle
instead of waiting: read stdout, read stderr, write stdin, wait, kill. Every read and write suspends
the calling coroutine exactly as `run`'s wait does, so streaming a child's output into a response
costs one coroutine and no worker thread.

One handle covers what PHP splits between `passthru` (stream straight through) and `proc_open` (full
pipe control), because the difference between them is which members a caller happens to use, not two
kinds of process.

**Not shipped.** `crates/nvs-stdlib/src/process.rs` registers `run` alone; there is no handle type,
so a program that needs to interleave with a child's output has no member to reach for.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/processes-and-files/#process-run" title="run waits by suspending the coroutine, and hands back the exit code with both captures as bytes"><code>core-classes/process-run</code></a> <a href="/docs/rules/core-classes/processes-and-files/#process-is-argv-only" title="Core\Process is the one way to run another program, and there is no shell string anywhere in it"><code>core-classes/process-is-argv-only</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0044.md">record 0044</a></dd></div></dl>

</div>

<div class="nv-rule" id="process-refuses-a-shell-target">

## A target only a second command-line parser could run is refused, on every platform

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#process-refuses-a-shell-target"><code>core-classes/process-refuses-a-shell-target</code></a>
</div>

A target the platform can only run by handing it to a second command-line parser — `.bat` and `.cmd`
to `cmd.exe`, `.ps1` to `powershell.exe` — is refused before spawning, with a diagnostic naming the
extension and the reason.

The check runs on **every platform build**, not only on Windows, even though the underlying risk (a
second parser re-reading an already-quoted argument) is real only there. Behaviour that silently
diverges by platform is the failure this refusal exists to prevent, and a case that passes on the
developer's machine and refuses in production is worth more than one that does the reverse.

Unix needs no equivalent. A shebang script is launched by `execve` reading the interpreter line and
invoking it in the same kernel call that receives the original, already-split argument vector — no
second program re-parses a command line, because there never was one. The asymmetry is the honest
shape of the underlying problem.

There is no convenience for the case where a batch file really is the target: a caller spawns
`cmd.exe` explicitly, through the same argv API, and takes the quoting risk visibly rather than
through a flag that looks as safe as every other call.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A <code>.bat</code>, <code>.cmd</code> or <code>.ps1</code> target does not run at all; a shebang script does, because <code>execve</code> reads it without re-parsing anything</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/processes-and-files/#process-is-argv-only" title="Core\Process is the one way to run another program, and there is no shell string anywhere in it"><code>core-classes/process-is-argv-only</code></a> <a href="/docs/rules/core-classes/processes-and-files/#process-run" title="run waits by suspending the coroutine, and hands back the exit code with both captures as bytes"><code>core-classes/process-run</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0044.md">record 0044</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/process-run-refuses-every-shell-target-and-only-those.nvst"><code>tests/conformance/core/process-run-refuses-every-shell-target-and-only-those.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="io-write-stream">

## `Core\IO::writeStream` is where every stream reaches disk, and a failed write removes its partial file

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#io-write-stream"><code>core-classes/io-write-stream</code></a>
</div>

`Core\IO::writeStream(string $path, Iterable<bytes> $src, {max?, overwrite?})` is where a stream
reaches disk, and every convenience that writes one delegates to it. Path first, because the subject
is parameter one and the plain write already reads that way.

It is an ordinary `Core\IO` member and therefore an ordinary **path sink** requiring `fs.write`, with
no exemption for arriving by way of an upload: a `tainted` filename reaching it is a compile error
exactly as it is everywhere else, and the containment check is the launderer.

Two rules are its own. **`overwrite` defaults to false**, because a destination chosen from a
client's claimed filename is the case this member exists to serve. **A write that fails mid-stream
removes the partial file**, because a truncated file the application believes it wrote is a worse
failure than an error — the caller learns from the throw, not from a later reader.

It is general on purpose: a request body, a decompressed archive, an outbound response body and an
upload part all reach disk through this one implementation, so the partial-write cleanup lives in one
place rather than in every call site that hand-wrote the loop.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>move_uploaded_file</code> and no temp file to move — a part is streamed to a path the program chose, and <code>overwrite</code> defaults to false</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/processes-and-files/#temporary-dir-sweep" title="A temporary directory lives under a Novis-owned root and is deleted when its script ends, and the sweep never throws"><code>core-classes/temporary-dir-sweep</code></a> <a href="/docs/rules/core-classes/processes-and-files/#process-run" title="run waits by suspending the coroutine, and hands back the exit code with both captures as bytes"><code>core-classes/process-run</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0105.md">record 0105</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/io-write-stream-lands-its-chunks-in-order-and-refuses-to-replace.nvst"><code>tests/conformance/core/io-write-stream-lands-its-chunks-in-order-and-refuses-to-replace.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/io-write-stream-asks-the-door-before-it-draws-a-chunk.nvst"><code>tests/conformance/core/io-write-stream-asks-the-door-before-it-draws-a-chunk.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/io-write-stream-holds-its-max-on-both-sides.nvst"><code>tests/conformance/core/io-write-stream-holds-its-max-on-both-sides.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="temporary-dir-sweep">

## A temporary directory lives under a Novis-owned root and is deleted when its script ends, and the sweep never throws

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#temporary-dir-sweep"><code>core-classes/temporary-dir-sweep</code></a>
</div>

`Core\IO::temporaryDir()` is the whole temporary-file surface: it hands out an **owned directory**
and the program names files inside it. There is no `temporaryFile`, because a program needing one
temporary file needs somewhere to put the second.

Every directory is created under one root the runtime owns — a configured path, else a private
subdirectory of the platform temporary directory. Exclusive ownership of that root is the entire
safety argument for the sweeps: the runtime never deletes anything it did not create, because nothing
else writes there. Sweeping a shared `/tmp`, with anyone's symlinks and anyone's names, is the
classic TOCTOU surface this forbids.

The runtime keeps a per-script list of the paths it handed out and deletes each surviving entry when
the script ends — after the exit queue on a CLI ending, after the after-response work on a request,
and off the request path, so a response never waits on a deletion. It covers normal end, `exit`, an
uncaught throw, and a request that died mid-flight.

**The sweep never throws and never alters a response.** A path already gone is the goal state reached
early. A deletion the OS refuses is one log line, and the directory waits for the next sweep. An
operator may set `[debug] keep_temporary` to keep everything *visibly* — each kept path is logged —
and there is no in-language setter, because a program that can exempt its own files can be made to
hoard them. The program's own `remove` and `removeDir` are unchanged and still throw: a deliberate
action's failure is the program's to hear about.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>tmpfile</code>, no <code>sys_get_temp_dir</code> writing and no <code>tempnam</code> — the runtime hands out a directory it owns and reclaims it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/processes-and-files/#temporary-dir-orphan-sweep" title="The orphan sweep is keyed on the owner being alive, never on age, and runs in exactly two places"><code>core-classes/temporary-dir-orphan-sweep</code></a> <a href="/docs/rules/core-classes/processes-and-files/#io-write-stream" title="Core\IO::writeStream is where every stream reaches disk, and a failed write removes its partial file"><code>core-classes/io-write-stream</code></a> <a href="/docs/rules/errors/how-an-error-travels/#propagation" title="An error propagates as a checked return, never by unwinding"><code>errors/propagation</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0131.md">record 0131</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0127.md">record 0127</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0072.md">record 0072</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0106.md">record 0106</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0078.md">record 0078</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0059.md">record 0059</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0004.md">record 0004</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/io-a-temporary-directory-is-made-not-named.nvst"><code>tests/conformance/core/io-a-temporary-directory-is-made-not-named.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/io-a-temporary-directory-is-a-granted-path-like-any-other.nvst"><code>tests/conformance/core/io-a-temporary-directory-is-a-granted-path-like-any-other.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/io-temporary-dir-is-refused-outright-with-nothing-granted.nvst"><code>tests/conformance/core/io-temporary-dir-is-refused-outright-with-nothing-granted.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="temporary-dir-orphan-sweep">

## The orphan sweep is keyed on the owner being alive, never on age, and runs in exactly two places

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#temporary-dir-orphan-sweep"><code>core-classes/temporary-dir-orphan-sweep</code></a>
</div>

A process killed outright ran no end-of-script sweep. Its leftovers are reclaimed by the orphan
sweep, which walks the owned root and deletes each entry **whose owning pid is not alive**.

It runs in exactly two places: once at server boot, before traffic, and whenever an operator runs the
cleanup command, which prints each path it removes and supports a dry run. It runs on no other
invocation — taxing every CLI start with a root walk to insure against a rare hard kill prices the
common case for the exceptional one. The deliberate consequence is that on a machine where the server
never runs and nobody runs the command, a hard-killed script's directory persists: bounded by crash
frequency, confined to one visible root, one command to clear.

The predicate is owner liveness, **never age**. An age rule is precisely what deletes a long-running
process's files out from under it; liveness cannot, because a live owner's entries are skipped no
matter how old. Every failure mode falls the safe way — a recycled pid makes a dead owner's entry
look alive and it leaks until a later sweep, never the reverse — so the sweep may under-delete and
can never over-delete. There is no force flag that overrides liveness.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/processes-and-files/#temporary-dir-sweep" title="A temporary directory lives under a Novis-owned root and is deleted when its script ends, and the sweep never throws"><code>core-classes/temporary-dir-sweep</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0131.md">record 0131</a></dd></div></dl>

</div>
