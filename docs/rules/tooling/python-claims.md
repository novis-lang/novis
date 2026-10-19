The claim Novis makes against Python is the one Python is worst at: **the tool that gets handed to
somebody else.** It is `rule:programs/audience`'s existing purchase pointed at the command line — a
second audience below the first and never above it — and it adds no priority, reorders nothing, and
schedules no milestone of its own.

Python holds the quick script through three things Novis takes none of: the REPL and the notebook,
being allowed to be wrong and still run, and a numeric stack that is C and Fortran under a Python
surface. So Novis does not compete there. Handing a Python program to a colleague means a virtualenv,
an interpreter version or a bundler; a Novis tool is one file. At the command line the missing
ecosystem also costs far less than on the web: what a quick tool imports — an HTTP client, JSON, CSV,
regex, a URI parser, hashing, a path type, an argument parser — is already in `Core`. And the
standing top findings of a Python tooling review, a shell command built from a string, a secret in a
log line and an interpolated query, are each a compile error here and cannot be retrofitted into
Python.

**Permitted:** that a Novis CLI program ships as one file with no interpreter, virtualenv or package
install; that argument parsing, help, completions, colour, prompts and progress are in the binary;
that any function may suspend, so there is no `async` split through the library; that shelling out
through a string (`rule:core-classes/process-is-argv-only`), leaking a secret
(`rule:security/secret-qualifier`) and interpolating a query (`rule:security/tainted-qualifier`) are
compile errors; and any **measured** figure, quoted with its engine, mode and host.

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

**Declined:**
- **Saying nothing about Python.** The command-line surface is built either way, and the two
  readings of it lead to different builds: a REPL and dynamic escapes under one, neither under the
  other. A question that changes what gets built is a decision.
- **Aiming at the prototype: a REPL, relaxed type rules for a script, an unchecked mode.** That niche
  is decided by the package index and the notebook, and Novis has neither. Loosening the rules to
  compete there spends the one durable difference, that being wrong does not compile, on a market it
  cannot win (`rule:tooling/no-repl`).
