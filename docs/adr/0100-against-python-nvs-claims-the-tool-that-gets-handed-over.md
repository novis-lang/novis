# ADR 0100 — Against Python, MWL claims the tool that gets handed over, not the script that gets thrown away

- **Status:** Accepted
- **Date:** 2026-08-26
- **Scope:** what MWL claims against Python, what it is forbidden to claim, and the three things that
  follow — a file whose first two bytes are `#!` starts in code mode, there is no REPL, and the userland
  benchmark suite grows Python as a third engine. Not in scope: any `Core` member, which
  [docs/spec/01-core-library.md](../spec/01-core-library.md) owns; the terminal surface and `#[Command]`
  argument parsing, which [0086](0086-core-cli-terminal-is-a-sink.md) owns; the single-file executable,
  which [0048](0048-portable-single-file-executables.md) owns; and the *primary* audience, which
  [0080](0080-the-audience-mwl-is-built-for.md) owns and which this ADR does not reorder.
- **Amends:** [0080](0080-the-audience-mwl-is-built-for.md) § 1 — its audience table is unchanged and still
  ranks first; § 1 below adds the second audience underneath it, and § 2 below extends that ADR's
  speech rule from PHP to Python, unchanged in shape.
  [0049](0049-single-open-tag-and-single-exit-keyword.md) § 2 — a shebang file enters code mode with no
  tag; "`<?mwl` is the only code-mode open tag" is untouched, because a shebang is not a tag.
  [0026](0026-performance-measurement-methodology.md) § 6's closing paragraph — the userland suite is three
  engines, not two, and its headline metric is still the instruction count.
  [benches/userland/README.md](../../benches/userland/README.md) — a case may carry a `.py` twin, and the
  fairness rule gains the one exception § 5 names.
- **Amended by:** none.

> **In short:** Python's grip on the quick script rests on the REPL, on being allowed to be wrong and still
> run, and on fifteen years of C-backed packages. MWL will take none of the three, **so it does not try**.
> The claim it makes instead is the one Python is worst at: **the tool that gets handed to somebody else** —
> one file with no interpreter, no virtualenv and no `pip install`
> ([0048](0048-portable-single-file-executables.md)); argument parsing, `--help`, completions, colour and
> prompts already in the binary ([0086](0086-core-cli-terminal-is-a-sink.md) § 6); no coloured `async`;
> and `shell=True`, a secret in a log line and an interpolated query each a **compile error**
> ([0044](0044-core-process-argv-only-no-shell.md),
> [0033](0033-secret-qualifier-for-confidential-values.md),
> [0024](0024-taint-tracking-for-injection-sinks.md)). That is
> [0080](0080-the-audience-mwl-is-built-for.md)'s existing purchase pointed at the command line, which is
> why this ADR adds no priority, reorders nothing, and schedules no milestone of its own. Three things
> follow and each is binding. **"Faster than Python", "replaces Python" and "Python without the GIL" are
> forbidden phrasings**, on 0080 § 3's rule and for its reason. **A file whose first two bytes are `#!`
> starts in code mode** — a CLI script has no inline HTML, so an opening tag is ceremony with nothing to
> justify it. **There is no REPL and none is planned**, stated here so the absence reads as a decision
> rather than an oversight. And because every sentence above is an argument where this project's own rule
> is that a performance claim is a number: **`tools/bench.py` grows Python as a third engine.**

## Context

- **Nothing in the docs said what MWL is to a Python developer**, while the CLI half of the project — 0086's
  terminal surface, 0048's single-file executable, 0044's `Core\Process`, 0093's service installer — was
  built as though the answer were obvious. It is not obvious, and the two plausible answers lead to very
  different work: "a faster Python" would justify a REPL, dynamic typing escapes and a numeric stack, none
  of which any accepted ADR wants.
- **Python is the incumbent 0080 already flagged as the fastest riser**, and it owns a category MWL's
  primary audience is standing in anyway: internal tooling, ops glue, the deploy script, the report
  generator, the one-off importer. A team that adopts MWL for 0080's reasons writes those too.
- **Three things hold Python's quick-script position, and MWL reaches none of them.** The REPL and the
  notebook, which are as much of the story as the language. Being allowed to be wrong and still run —
  precisely what [0022](0022-definite-property-initialization.md),
  [0024](0024-taint-tracking-for-injection-sinks.md) and
  [0090](0090-one-equality-operator-and-disjoint-types-do-not-compile.md) exist to prevent, correctly. And
  the numeric and ML stack, which is C and Fortran with a Python skin and is not an ecosystem gap that
  closes with time.
- **Python's worst area is the one MWL is strongest in.** Handing a Python program to a colleague means a
  virtualenv, an interpreter version, or PyInstaller. `mwl build --compile`
  ([0048](0048-portable-single-file-executables.md)) is one file. This is the wedge Go actually won from a
  standing start, and 0080 § 4 already names Go's deployment story as one of the two precedents for
  entering an occupied market at all.
- **At the command line, "no ecosystem" costs far less than it does on the web.** What a quick tool imports
  is an HTTP client, JSON, CSV, a regex engine, a URI parser, hashing, a path type and an argument parser.
  `Core` holds every one of those ([docs/spec/01-core-library.md](../spec/01-core-library.md) §§ 5-8, 12,
  16), and 0086 § 6 adds `#[Command]`/`#[Option]` with generated help and shell completions, which in
  Python is a third-party install. On the web the missing framework and driver ecosystem decide the
  question, which is why [0082](0082-the-first-party-framework.md) had to exist; at the command line the
  gap is already largely closed by [0051](0051-standard-library-tiers.md)'s Tier 0 roster.
- **The standing top findings of any Python tooling review are `subprocess(shell=True)`, a secret reaching
  a log line, and a query built by interpolation.** Each is a compile error here — 0044, 0033, 0024 — and
  none of the three can be retrofitted into Python for the reason 0080's *Context* gives about qualifiers.
  This is not a new claim; it is the existing one, applied to a different program shape.
- **MWL has never been measured against CPython.** [docs/perf/userland-gap.md](../perf/userland-gap.md)
  holds the suite's standing and is MWL against PHP only. Every performance sentence in this ADR would
  therefore be an assertion, and this project does not publish those
  ([0026](0026-performance-measurement-methodology.md) § 2).

## Investigation

- **Cold start is already measured; only the third engine is missing.** `tools/bench.py` reports `total`
  and `work` side by side and subtracts `00-baseline` — the empty program — to produce the second. `total`
  on `00-baseline` *is* the cold-start figure, and the harness's own header already says `total` answers
  "what does this script cost me at the command line" while `work` answers "how fast is the language". For a
  CLI claim the headline is `total`, and it needed no new measure — it needed a column.
- **The harness was already built for this.** Its header names the seam: `measure()` returns a dict per
  engine per case, `COLUMNS` says which keys print. What it hardcoded was the *number* of engines — two —
  in `evaluate`, in the baseline subtraction, in `explain` and in `write_json`. Turning that count into a
  list is a mechanical change with no new measurement concept, which is why this ADR can decide it rather
  than defer it.
- **Every scripting engine strips a leading `#!` line, and PHP does too** — `php script.php` with
  `#!/usr/bin/env php` on line 1 prints nothing extra, verified against the PHP 8.5.9 oracle this project
  already keeps. What PHP does *not* do is treat the shebang as entering code mode: the file still needs
  `<?php` on line 2. So the precedent for stripping is unanimous and the precedent for the second half is
  Python, Ruby, Perl, Node and `sh`, all of which are simply in code mode from byte 0 and have no tag to
  write.
- **A shebang line is unambiguous at byte 0 and nowhere else.** `#!` as the first two bytes of a file is
  not text any author means to emit, and no HTML, XML or template document begins with it. Away from byte 0
  it is ordinary text and stays ordinary text, so the rule needs no lookahead, no heuristic and no repair —
  which is what keeps it on the right side of [0095](0095-ambiguous-input-is-refused-never-repaired.md).
- **A REPL is not a small feature here and never becomes one.** It needs a top-level scope that survives
  between inputs, redefinition of a class or member already compiled, and a printed representation of every
  value — three things that respectively fight
  [0011](0011-functions-and-constants-are-class-members.md) (everything is a class member, so there is no
  top level to bind into), [0042](0042-on-disk-artifact-cache-format.md) (an artifact is keyed by content
  hash, so redefinition is a cache invalidation problem, not an edit) and
  [0022](0022-definite-property-initialization.md). Each is answerable; none is answerable cheaply, and
  the answers would be visible in the language rather than confined to a tool.

## Decision

### 1. The second audience, and why it needs no new priority

Below [0080](0080-the-audience-mwl-is-built-for.md) § 1's table, and never above it, sits one more:

| The user | What makes MWL the answer rather than an option |
|---|---|
| A team writing internal and ops tooling that touches production credentials, customer data or a shell — today in Python, by default rather than by choice | `Core\Process` is argv-only ([0044](0044-core-process-argv-only-no-shell.md)); a `secret` cannot reach a log, a dump or a `Throwable` ([0033](0033-secret-qualifier-for-confidential-values.md)); an interpolated query does not compile ([0024](0024-taint-tracking-for-injection-sinks.md)); the finished tool is one file with no interpreter to install ([0048](0048-portable-single-file-executables.md)) |

This is the **same purchase** 0080 already describes, bought by a program with no HTTP request in it. It
justifies no new milestone, no reordering of [0004](0004-memory-for-simplicity.md)'s priorities, and no
`Core` member that would not have been built anyway. Its whole content is that CLI work already scheduled
is aimed at a named reader, and that §§ 2-5 below are the four places the aim was missing.

**It is second, and stays second.** If a slice would serve this audience at the cost of 0080 § 1's, it
loses; the greenfield multi-tenant platform is what makes an ecosystemless language viable at all, and this
audience does not have that property to the same degree — a team with Python tooling has working tools.

### 2. What may be said about Python, and what may not

[0080](0080-the-audience-mwl-is-built-for.md) § 3 governs how this project speaks about PHP. The identical
rule now governs Python, and for the identical reason: a claim a reader can test and find false costs more
than the adoption it buys.

- **Permitted:** that an MWL CLI program ships as one file with no interpreter, virtualenv or package
  install; that argument parsing, help, completions, colour, prompts and progress are in the binary; that
  any function may suspend so there is no `async` split through the library
  ([0072](0072-core-task-structured-concurrency.md)); that shelling out through a string, leaking a secret
  and interpolating a query are compile errors; and any **measured** figure from § 5's suite, quoted with
  its engine, mode and host, as [0026](0026-performance-measurement-methodology.md) already requires.
- **Forbidden in any document, error message, `--help` text or landing page:** "faster than Python",
  "replaces Python", "Python without the GIL", "a typed Python", or any phrasing implying a Python
  program, script or package runs, converts or ports. There is no `mwl convert` for Python and none is
  planned — [0089](0089-convert-is-one-rule-table-with-two-modes.md) is a PHP rule table and gains no
  second language.
- **Required, wherever the comparison is made at all:** that MWL has no REPL (§ 4), and no numeric or
  machine-learning stack and no route to one ([0052](0052-closed-doors.md) is why the FFI that would
  provide it does not exist).

### 3. A file whose first two bytes are `#!` starts in code mode

```
#!/usr/bin/env mwl
Core\Cli::write("hello\n");
```

- **The trigger is exact:** the bytes `#!` at offset 0. Line 1, up to and including its first `\n`, is
  **trivia** — it is not a token, it is not emitted, and it belongs to the trivia layer
  [0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md) already defines, so `mwl-fmt` preserves it and
  the LSP sees it as a comment.
- **The file then continues in code mode**, exactly as if `<?mwl` stood there. Nothing else changes: `?>`
  still switches to text mode and still writes literal bytes to standard output, and a later `<?mwl`
  reopens code mode, both per [0049](0049-single-open-tag-and-single-exit-keyword.md).
- **`#!` anywhere but offset 0 is ordinary text**, in text mode or in code mode alike, with no lookahead
  and no special case. A byte-order mark before it therefore defeats the shebang and the file has none —
  which is PHP's long-standing behaviour too, and is deliberately left as-is rather than repaired here,
  because nothing in `mwl-syntax` handles a BOM at all yet and inventing one rule for one marker is how a
  parser acquires the heuristics [0095](0095-ambiguous-input-is-refused-never-repaired.md) forbids.
- **An `<?mwl` in a shebang file, before any `?>`, is a diagnostic** rather than a lex error naming
  something the author did not write: `E0009`, "this file opens with `#!` and is already in code mode;
  remove the `<?mwl`". The reverse — a shebang file that never wanted code mode — is not a shape anyone
  writes and gets no rule.
- **The line is trivia on every platform**, so one file runs as `./app` on Unix and as `mwl app.mwl` on
  Windows with no edit. Windows has no kernel shebang support and this ADR does not add one; the Windows
  answer to distribution is 0048's `--compile`, which is also the better answer on Unix.

This slice is **lexer-only** — `mwl-syntax`, one branch at offset 0 — and is scheduled wherever M1's
remaining grammar items land. It changes no parser rule, no HIR shape and no runtime behaviour.

### 4. There is no REPL, and none is planned

`mwl` gains no `repl` subcommand and no interactive evaluator, and the roster in
[0093](0093-a-service-is-one-stored-argv-and-the-installer-is-a-sink.md) § 2 is not reopened for one.
The reason is *Investigation*'s last bullet: the three things a REPL needs are each a language question
disguised as a tool, and answering them would put a second, looser set of rules beside the one every
compiled program obeys — the exact shape [0015](0015-no-name-aliasing.md) rules against.

What exists instead, and what the documentation points at when the question is asked:

- **`mwl run file.mwl`** for a script, made cheap by [0042](0042-on-disk-artifact-cache-format.md)'s
  artifact cache — the second run of an unchanged file compiles nothing.
- **`mwl test`** ([0079](0079-testing-is-a-language-feature.md)) for the "poke at it until it works" loop,
  which is what a REPL is used for most of the time and which leaves something behind afterwards.
- **`Core\Debug::dump`** ([0092](0092-one-diagnostic-record-three-renderings.md)) for looking at a value.

This is a **decision, not a gap**, and § 2 requires it to be said out loud wherever MWL is compared to
Python — a reader who discovers it after installing has been misled by omission.

### 5. Python is a third engine in the userland suite, and the roster is open

[benches/userland/](../../benches/userland/) gains a `NN-slug.py` beside each existing pair, and
`tools/bench.py` runs whichever engines a case has twins for. **Bun is the fourth**, added under this
section's own *Revisiting* clause rather than by a new ADR, with its cases in TypeScript and a `.ts`
suffix; it is the strongest engine in the suite and is kept for exactly that reason.

- **The engine list is data, not a count.** `evaluate`, the `00-baseline` subtraction, the table, `explain`
  and the NDJSON record all iterate the list; `--engines mwl,php` narrows it. This is the change the
  harness's own "Adding a measure later" note anticipated, one axis over — and Bun landing as one entry
  plus a suffix is the evidence that it was the right shape.
- **`total` is the headline for a CLI claim and `work` for a language claim**, and the suite keeps refusing
  to pick one. Against Python the two answer genuinely different questions, and quoting either without
  saying which is the misuse § 2 forbids.
- **Byte-identical output is still the gate**, unchanged: a case whose three halves disagree reports `DIFF`
  and no time.
- **The fairness rule gains exactly one exception, of one kind.** `benches/userland/README.md` says a case
  is written in each language's own idiom rather than transliterated, and that stands — a Python case joins
  its parts and builds a list comprehension where the PHP one appends to a string and calls `array_map`,
  and a TypeScript case chains `.map().filter().reduce()`. The exception is **arithmetic that must agree**,
  because the byte-identical gate is a correctness requirement rather than a style choice: Python floors
  `%` where the other three truncate it, so two cases spell the truncated remainder out; and a `number` is
  a float64, so the two cases whose seeded LCG passes 2^53 run that one line in `BigInt`. Each is confined
  to the cases that need it and carries a comment saying why.
- **Neither Python nor Bun is a toolchain dependency.** `tools/` is already Python
  ([0065](0065-third-party-attribution-and-mwl-info.md) § *Consequences* is where that was accepted), so
  the machine that can run `bench.py` can already run its cases; Bun is an ordinary optional install, named
  in [docs/setup.md](../setup.md). A missing twin skips that engine with a warning and an uninstalled
  engine is narrowed away with `--engines`, so neither can turn a build, a test or a loop session red —
  nothing outside this suite reads any of it.

## Consequences

**Positive**

- The CLI work already scheduled — 0086, 0048, 0044, 0093 — acquires a reader it is aimed at, and the next
  argument about a CLI slice has a table to be settled against rather than an intuition.
- A CLI script is two lines of ceremony shorter and looks like every other language's script, which is the
  cheapest possible improvement to the first thing a new user writes.
- The absence of a REPL becomes a sentence in the documentation instead of a discovery after install.
- The project stops making an unmeasured comparison. After § 5 lands, "how does this compare to Python" has
  a number attached to a host, an engine mode and a commit.

**Negative**

- **A second way a file may begin.** This is the real cost of § 3 and it is a
  [0015](0015-no-name-aliasing.md)-shaped one: `<?mwl` and `#!` both start code, and a reader must
  now know two openings rather than one. Accepted because they are not two spellings of one concept — a
  shebang selects a *file kind* the operating system already requires the marker for, and a file with one
  is a program rather than a template. It buys usability at the command line and costs one sentence of
  language surface; nothing else in [0004](0004-memory-for-simplicity.md)'s ordering moves, since there is
  no runtime, latency or memory consequence at all.
- **A default `bench.py` run gets slower**, because CPython is far behind both other engines on the loops
  these cases are made of and pays that on every rep. `--engines mwl,php` is the escape and the existing
  pattern filters still apply.
- **Twenty-one more files to keep in step.** The byte-identical gate is what stops them from rotting
  silently, which is the same protection the `.php` twins already have and the reason `--check` is cheap.
- **A second audience is a second thing to say no to.** § 1's closing paragraph exists because the failure
  mode is real: a language that serves two audiences equally serves the first one worse, and 0080's
  audience is the one that makes the project viable.

**Neutral**

- No `Core` member is added, removed or changed, and no milestone gains scope. § 3 is one lexer branch and
  § 5 is a tools-and-benches change; §§ 1, 2 and 4 are what the project *says*, which is exactly the shape
  0080 established.

## Alternatives rejected

- **Say nothing about Python.** The status quo, and the cheapest option. Rejected because the CLI surface
  is being built either way and the two candidate answers lead to different builds — a REPL and dynamic
  escapes under one reading, neither under the other. A question that changes what gets built is a
  decision, not a marketing matter.
- **Aim at the prototype instead: add a REPL, relax the type rules for a script, allow an unchecked mode.**
  Rejected on the same evidence 0080 § 4 already assembles. Deno led with a permissions model, was
  admired, and stalled until it shipped npm compatibility; the prototype niche is decided by the package
  index and the notebook, and MWL will have neither. Loosening the rules to compete there would spend the
  one durable asymmetry — that being wrong does not compile — for a market it cannot win anyway.
- **Keep `<?mwl` required after a shebang, exactly as PHP requires `<?php`.** The most conservative option
  and a defensible one: one file opening, one rule. Rejected because the tag's whole justification is
  distinguishing code from surrounding template text, and a shebang file has no template text by
  construction — so it is ceremony charged to every CLI script for a disambiguation that cannot arise. PHP
  pays it for backward compatibility with files that predate its CLI SAPI; MWL has no such files.
- **A second extension — `.mwls` for a script, code mode from byte 0.** Rejected: the shebang is already
  the unambiguous marker and it has to be in the file anyway, so an extension adds a second thing to get
  right and splits tooling, editor associations and `require` resolution for no gain.
- **Make every file code mode by default and require an opt-in for template mode.** The cleanest design
  looked at purely from the command line, and rejected on scope rather than on merit: it overturns
  [0021](0021-single-file-inclusion-construct.md), inline HTML, `<?=`, and the on-ramp claim
  0080 § 3 permits, in order to save one line in a CLI script. If the template half of the language is ever
  reconsidered on its own terms, this is the alternative to reopen.
- **Add a Python column by transliterating the PHP cases.** Rejected for the reason
  `benches/userland/README.md` already gives about the `.mwl`/`.php` pair: a transliteration measures how
  badly one language writes another's idiom. A Python case joins a list where PHP concatenates and writes a
  comprehension where PHP calls `array_map`, and the one arithmetic exception § 5 names is stated as an
  exception precisely so it is not read as licence for more.
- **Benchmark against PyPy as well, or instead.** Rejected: the Python a tool is actually run under is
  CPython, and a second Python engine doubles the maintenance of the twins to sharpen a comparison nobody
  in § 1's audience experiences. `--engines` makes adding one later a flag rather than a redesign — as Bun
  then demonstrated.
- **Leave Bun out, on the grounds that it is not what § 1's audience is switching from.** True, and
  rejected anyway: Bun is the closest thing in the suite to what MWL is trying to be — a fast JIT behind a
  scripting surface with the batteries in the binary — so it is the engine whose numbers actually test
  this project's own claims. Measuring only engines MWL beats is how a benchmark suite stops being
  evidence.

## Revisiting

- **§ 4 reopens if `mwl test`'s loop turns out not to substitute for a REPL in practice** — the concrete
  trigger is a measured session or a user report where the answer to "what is this value" costs a build.
  What would be built then is *not* a general evaluator: it is a debugger-shaped inspector over a paused
  isolate, which [0006](0006-isolated-script-execution.md) already makes cheap and which raises none of
  *Investigation*'s three questions.
- **§ 1 reopens if the CLI audience ever produces adoption the primary one does not.** That would be
  evidence about which greenfield story is real, and 0080 § 1's ordering is a hypothesis, not a
  commitment.
- **§ 5's engine list reopens the moment a comparison is asked for that the suite cannot answer.** It has
  already reopened once, for Bun, and cost one `Engine` entry and a suffix — which is the clause working
  as intended. A second Python implementation, Node or Go would go the same way. The list being data is
  what makes each of those a flag rather than this ADR again.
- **Bun's numbers are the sharpest standing argument against § 2's permitted claims**, and are recorded in
  [docs/perf/userland-gap.md](../perf/userland-gap.md) rather than softened. A mature JIT beats MWL on the
  median userland case today; what MWL still wins there is cold start, which is the CLI headline § 5
  names. If that stops being true, § 1's second audience loses its best evidence and this ADR is worth
  re-reading whole.

## Verification

- **§ 3** is a lexer slice with its cases in `crates/mwl-syntax`: a shebang file whose first statement
  parses; a shebang file containing `?>` followed by text and a reopening `<?mwl`; `#!` at offset 1 and
  after a leading newline, both lexing as ordinary text; `E0009` on a `<?mwl` in a shebang file; and the
  shebang line surviving a `mwl-fmt` round trip as trivia, which is the [0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md)
  half.
- **§ 5** is verified by the suite itself: `python tools/bench.py --check` passes with three engines, which
  is the byte-identical gate over every case, and `--engines mwl,php` reproduces the two-engine table
  unchanged.
- **§§ 1, 2 and 4** are rules about documents and are verified by reading, exactly as
  [0080](0080-the-audience-mwl-is-built-for.md) § 3 is. No check enforces a forbidden phrasing and none
  should; the rule exists so a reviewer has something to point at.
