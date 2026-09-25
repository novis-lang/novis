# The hostile tree — one file per feature, written to break it

Every feature Novis ships owes an attack
([ADR 0134](../../docs/decisions/0134.md)), and this is where they live. A
case here is not a test of what the feature *does*. It is the program somebody writes when they are
trying to make the runtime come apart, and it passes when the runtime is still standing afterwards.

`bun nv proofs --run` runs them with the examples, and `--valgrind` runs them under valgrind on a
Linux leg. This file owns what a case **is**; the tool owns how it is judged.

## Where a case goes

`tests/hostile/<the feature's path>/NN-slug.nvs`, the same path the example and bench trees use:
`tests/hostile/core/Str/length/01-unbounded-input.nvs`. As many files per feature as the feature
deserves; one is what is owed. `bun nv proofs --id '<feature>'` prints the directory.

## The contract — what makes a case pass

**There is no expected output, and adding one would be a mistake.** Freezing the output makes it a
fixture, and what a hostile case asserts is not what it printed. It asserts that *nothing came
apart*:

| The runtime may | The runtime may not |
|---|---|
| run the program to completion | panic — `panicked at`, an abort, `RUST_BACKTRACE` |
| throw, and let it go uncaught | exit with a crash-shaped status (a signal, `0xC0000005`) |
| stop it at a limit, cleanly | still be running when the timeout expires |
| run out of memory *and say so* | leak definitely, under `--valgrind` |
| take as long as its timeout allows | report an internal error |

So a program that throws, one a limit stops and one that simply works are all passes. That is the
point: **a hostile case is about the blast radius, not the verdict.**

**The exception is a compile diagnostic**, which is a failure — because an attack that does not
compile was never delivered, and a typo would otherwise survive every sweep for the rest of this
repository's life. Where the refusal *is* the assertion — a sink handed a tainted value, a
capability used without being granted, a `secret` asked to print itself — say so:

    // hostile: expect-refusal

and then compiling cleanly is what fails the case instead.

**The second exception is a program that ends before its last line**, for the same reason. A case
is usually several numbered attacks in one file, and the program is one process: an uncaught throw
or a limit in step 2 means steps 3 to 5 were never delivered, and a runtime that survived step 2
has said nothing about them. So:

- **Catch everything that can be caught**, and say in the comment that it is caught so the attack
  after it still runs. A `RecursionError`, a `LogicError`, a `ParseError` — each goes in a `try`.
- **What cannot be caught goes last.** A memory limit and `exit` end the program whatever is
  wrapped around them, so the step that reaches one is the file's final step, and the case says so:

      // hostile: ends-early

- A feature that deserves two endings gets two files. `02-slug.nvs` costs nothing.

A non-zero exit status without the marker fails the case as *ended before its last line*. With the
marker, running to the last line is what fails it — the ending it declares did not happen, and the
marker comes off or the attack is made to land.

## What to write

Write what an attacker writes. The shapes that have found things in languages like this one:

- **Unbounded input** — a string, an array, a nesting depth, a repeat count sized to the memory of
  the machine rather than to the example. Then the same thing one element under a documented limit,
  and one over.
- **Deep recursion and deep nesting** — a structure deep enough to reach the engine's own recursion
  bound, which exists and is guarded (`benches/abi-probe/tests/invariants.rs`).
- **Boundaries by one** — an offset at the length, a negative length, an empty receiver, a range
  that ends before it starts, the largest `int` and the value after it.
- **The refcount edges** — a cycle, a value held while the thing under it is replaced, a closure
  capturing what created it, an `inout` outliving its frame. These are where a leak actually lives,
  which is why `--valgrind` exists.
- **Encoding that is not text** — invalid UTF-8, a lone surrogate, a combining mark alone, a
  grapheme split down the middle, a NUL in a path.
- **The thing the feature promises not to do** — a sink handed a tainted value, a secret asked to
  print itself, a capability used without being granted. A refusal is the pass; a silent success is
  the finding.
- **A type pushed past what the checker proved** — a `rule:types/narrowing` narrowing that should not
  hold, a union reached through the wrong arm, a generic instantiated against its variance, a
  `rule:types/erased-member-access` receiver widened and then read as its old shape. Compiled code
  loads a field without re-reading its tag, because `rule:types/declaration` settles the static type,
  so a checker that is wrong here is a memory-safety question rather than a wrong answer.

Every case opens with an `// Attack:` comment saying what it tries and what should happen instead,
and each numbered step gets one line saying what that step tries. **These comments are read by
people looking the feature up, not only by us**, so they are written in plain words a beginner
follows: [`AGENTS.md`](../../AGENTS.md) § *Text an end user reads* is the rule, and it holds here
unchanged. The list above is for choosing the attack; its vocabulary —
refcount, shard, variance — does not go into the comment.

## When a case actually breaks something

That is the point of the tree, and there are two honest answers: **fix it**, or **record it**. Record
it with a gap record under `data/gaps/<crate>/`, naming its owner, and a marker on the case:

    // proof: gap nvs-stdlib/a-slug-naming-what-breaks

The sweep then counts the case as `known-gap` instead of a failure, so a long unattended run is not
stopped by a bug too big for the session that found it, and `bun nv proofs --gaps` keeps
the list in front of anyone who asks. **A marked case that passes fails the sweep**, so the marker
comes off with the fix.

**Softening the attack until it survives is not one of the two answers.** A case that has been
weakened to go green is worse than no case: it reports that a thing was tried and held.

## The directives a case may carry

    // hostile: timeout-ms 4000     how long it may run before it counts as unbounded (default 10s)
    // hostile: expect-refusal      the compiler saying no is this case's assertion
    // hostile: ends-early          the last step ends the program, and that ending is the attack
    // proof: gap <gap id>          it breaks something; the gap record says what, not this file
    // requires: unimplemented      skip: the feature does not run yet
    // requires: unix               skip on Windows: it opens a Unix-domain socket

A case runs with nothing on its standard input. A sibling `<name>.in` is sent to it instead, byte
for byte, so an attack on a member that reads input can deliver bytes no `.nvs` file can spell —
[docs/examples/README.md](../../docs/examples/README.md) § *A file for standard input* is the rule.

A case needing longer than a few seconds is usually measuring the machine rather than the runtime —
raise the ceiling deliberately, and say in the comment why this one earns it.

## Why these are not in `tests/conformance/`

A `.nvst` case freezes stdout byte for byte, and every one of these is written to produce output
nobody can predict — an allocation failure message, a limit's diagnostic, whatever a 400 MB string
does. A suite whose expected output has to be maintained is a suite that gets weakened until it
passes. These are judged by a contract instead, and the contract is above.
