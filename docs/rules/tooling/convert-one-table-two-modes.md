Every rewrite `nvs convert` knows is a **rule** in one table, and every branch of a rule carries
exactly one tier. **E** — the converted construct behaves identically to the PHP one for every input
the converted program's type checker accepts. **D** — a mechanical Novis destination exists but the
behaviour may differ. **N** — no mechanical destination exists. The two modes are that tier read
through a filter, never a second code path:

| Tier | `--mode=equivalent` (default) | `--mode=runnable` |
|---|---|---|
| E | emitted as code | emitted as code |
| D | original commented out, the idiomatic Novis shape beside it | emitted as code with `TODO(convert:<id>)` naming the difference |
| N | original commented out, the idiomatic shape beside it | the same |

So `--mode=equivalent` output is a worklist that **will not run**, and its header says so.
`--mode=runnable` output usually runs, is explicitly not idiomatic Novis, and every site where it may
diverge is one `grep` away (`rule:tooling/convert-annotations-and-report`).

A rule has **ordered branches**, and a branch's tier may be predicated on a side condition the
converter decides by its own analysis. A condition it cannot decide is **false**: control falls to the
next branch and the weaker tier applies — fail-closed, the same direction as
`rule:security/sink-predicate`. A rule record is data with exactly these fields: `id` (a domain
letter plus four digits, never reused), `match`, `when`, `tier`, `rewrite`, `diverges` (one
sentence — the `TODO` text), `idiomatic` (what Novis wants instead — the comment the default mode
leaves), `dialect` and `proof`. `diverges` says what will break; `idiomatic` says what to write.
