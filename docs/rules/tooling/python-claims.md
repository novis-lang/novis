The claim Novis makes against Python is the one Python is worst at: **the tool that gets handed to
somebody else.** It is `rule:programs/audience`'s existing purchase pointed at the command line — a
second audience below the first and never above it — and it adds no priority, reorders nothing, and
schedules no milestone of its own.

**Permitted:** that a Novis CLI program ships as one file with no interpreter, virtualenv or package
install; that argument parsing, help, completions, colour, prompts and progress are in the binary;
that any function may suspend, so there is no `async` split through the library; that shelling out
through a string (`rule:core-classes/process-is-argv-only`), leaking a secret
(`rule:security/secret-qualifier`) and interpolating a query (`rule:security/tainted-qualifier`) are
compile errors; and any **measured** figure from the userland suite
(`rule:tooling/bench-engine-list-is-data`), quoted with its engine, mode and host.

**Forbidden in any document, error message, `--help` text or landing page:** "faster than Python",
"replaces Python", "Python without the GIL", "a typed Python", or any phrasing implying a Python
program, script or package runs, converts or ports. No tool converts Python and none is planned. The
rule and its reason are
`rule:programs/three-claims`'s: a claim a reader can test and find false costs more than the
adoption it buys.

**Required wherever the comparison is made at all:** that Novis has no REPL (`rule:tooling/no-repl`),
and no numeric or machine-learning stack and no route to one (`rule:security/no-ffi`).

No check enforces a forbidden phrasing and none should; the rule exists so a reviewer has something
to point at.
