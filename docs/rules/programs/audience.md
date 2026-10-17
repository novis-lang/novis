Novis is for web applications — not a segment, not a vertical, not a trust posture. A team choosing
Novis is choosing it to build the web application they were going to build anyway, and what they get
for it is `tainted` and `secret` checked while compiling rather than scanned afterwards; every
request, job and connection an isolate with an enforceable memory, CPU and time budget; a
dependency's authority declared and narrowed rather than ambient; `decimal`, durations written like `30s` and an
always-valid-UTF-8 `string`; and a reload that swaps a pointer instead of restarting a worker.

**Running code the operator did not write is where those properties pay most, and that is a benefit
rather than a definition.** No document may write as though the untrusted-code case were the point of
the language. An application with one owner, one team and no untrusted input at all is served by
exactly the same properties, because the mistakes they catch are overwhelmingly the application's own.

**Command-line tooling is a second audience, and it stays second.** A team writing internal and ops
tools that touch production credentials, customer data or a shell gets the same purchase from a
program with no HTTP request in it — argv-only process spawning, a `secret` that cannot reach a log,
an interpolated query that does not compile, and a finished tool that is one file with no interpreter
to install. It justifies no new milestone and no reordering; where a slice would serve it at the cost
of a web application's, it loses. The greenfield web application is what makes a language with no
ecosystem viable at all, and the second audience lacks that property: a team with Python tooling
already has working tools.

**The safety properties are how Novis is built, not who it is built for.** They are also why this does
not end where a typed, incompatible dialect of an existing language ended before: that one's value depended on a package
compatibility it could not hold, and Novis depends on that compatibility at no point, puts every
primitive a framework needs in `Core` so a framework is a package anyone can write
(`rule:programs/no-first-party-framework`), and differentiates on properties an incumbent cannot
acquire in a minor release.

**No audience is deliberately second among web teams.** A PHP team modernising and a greenfield
backend choosing between Go and Node are served by the same language and the same three claims. What
the first is refused is a compatibility promise (`rule:programs/no-compatibility-promise`), and what
the second is refused is a speed-only argument (`rule:programs/three-claims`). A language with no
ecosystem is chosen for a service being built rather than one already running: that bounds *when* a
team can pick Novis, not *which* teams it is for.

**Declined:**
- **PHP teams modernising as the audience.** The value on offer would depend on a migration Novis
  cannot perform, since such a team cannot bring its framework, its packages or most of its code. That
  is the position the incompatible typed dialect held, and lost.
- **A narrow first segment, such as multi-tenant and regulated platforms, ranking every slice.** It
  describes the qualifiers and the isolation as one vertical's requirement when they are the
  language's ordinary behaviour, and tells every other team the language is not for them. That
  segment is where the properties pay most, and it stays named as that.
- **No named audience at all.** It ranks slices silently by what is interesting rather than by what
  is blocking, and produces a language with no dependency story, the thing every user meets first
  (`rule:programs/slice-ranking`).
