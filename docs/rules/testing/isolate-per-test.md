Each test runs on an isolate of its own: it shares compiled code with its siblings and nothing
else — not a static, not a cache entry, not an open handle. A static one test increments reads its
declared initial value in the next, always, with no flag to change it
(`rule:statements/an-isolate-has-its-own-statics`).

This is not a hardening measure bolted onto a sequential runner; it is what makes a parallel one
possible at all, because an isolate is cheap enough to be the default rather than a per-case escape
hatch. Two consequences follow and are answered elsewhere: shared expensive setup needs its own
mechanism (`rule:testing/fixtures`), and execution order carries no meaning, so the report is in
declaration order regardless of what finished when (`rule:testing/runner-is-strict`).

Each test isolate spends its parent's budget, so a suite has the same enforceable ceiling on memory
and CPU that a request has, and a runaway test is terminated rather than left to consume the
machine.
