B1 (errors) is the style every topic matches. This is its apply-file, abridged; the whole
rendered chapter is docs/rules/errors.md and the structure is docs/rules/errors.json.

## json: docs/rules/<topic>.json
{
  "topic": "errors", "title": "Errors", "order": 10,
  "rules": [
    {
      "id": "errors/on-limit",
      "title": "Tier 1 — a resource limit reaches the request that spent it",
      "status": "shipped",
      "because": ["0020"],
      "divergesFromPhp": "the handler is request-local rather than global, and runs on a reserve carved out before execution started",
      "seeAlso": ["errors/escalation-ladder", "errors/stack-depth"],
      "guardedBy": ["tests/conformance/core/fatal-on-limit-registers-a-handler-without-running-it.nvst"]
    }
  ]
}

## fragment: docs/rules/<topic>/<slug>.md
`Core\Fatal::onLimit(closure(LimitReport): void $handler): void` fires only for a resource-limit
`FATAL` — memory, CPU time, `max_output`, wall time, `max_script_depth` and call-stack depth. An
internal panic never reaches it (rule:errors/panics-bypass-user-code).

Registration is request-local, living beside the pending-error slot in `Ctx`, and dies with the
request like every other per-request slot. The handler runs on a **reserve carved out of the
request's own budget at request start** and unavailable to ordinary execution — otherwise a request
that exhausted its memory would have nothing left to report with.

## remap: 0020 § 1 -> errors/on-limit
## remap: 0020 § *Decision* -> errors/on-limit

WHAT TO COPY, AND WHAT THE PILOT LEARNED

- **A title is the rule as a sentence**, not a noun phrase: "A limit report is not a Throwable, and
  the type checker knows it", never "Limit reports". The ground-rules index is one line per rule and
  the title carries it.
- **A rule is one enforceable statement** — one thing you could write a `.nvst` case against. Mostly
  one per mapped anchor, but split where a section holds two: 0020 § 1 became `errors/on-limit` and
  `errors/stack-depth`, because the 8 MB native-stack bound is a different claim with its own guard.
  An anchor still remaps to exactly one rule, and two anchors may share a rule where a record's
  headline and its first section say the same thing.
- **The fragment states the rule, then why it is that rule.** Present tense, no "we decided", no
  reference to the ADR as a document — the reader is applying the rule, and the record is one click
  away in the trailer. Keep it to 60–200 words; four fragments over that in B1 all wanted splitting.
- **Cite another rule bare or in backticks — both render as one link.** `rules.py` reads either.
- **`status` is a claim about the tree, not about the design.** `shipped` means a reader can run it
  today. B1 marked `errors/renderings` and `errors/record-producers` `designed` because `nvs-render`
  has no HTML rendering and two of the five producers are not on the shared model — check the crate
  before you claim it.
- **`guardedBy` must be a path that exists**; `rules.py --check` refuses one that does not. Prefer a
  `.nvst` case, then a crate test. Empty is honest when nothing guards the rule yet.
- **`because` is every record the rule draws on**, not only the one that owns the anchor — it is what
  the reverse index at C1 is built from.

THE TWO TRAPS THAT COST B1 A ROLLBACK

- A record with no numbered sections is cited by section *name* (`ADR 0002 § *Measured cost*`), and
  a citation names a section *list* (`§§ 2-3`, `§§ 1, 3 and 4`). Both are ordinary `## remap:` keys.
  Run `grep -nE '§§|\[ADR [0-9]{4}\][^(]'` over your records' citation sites before you write the
  remap, and give every spelling you find a key — an unowned section leaves the whole citation
  untouched, which is silent.
- The apply-file is written to the scratchpad and never into the tree. `--apply` is the only thing
  that writes, it formats the Rust it rewrites, and it restores every byte if any of the six checks
  fails.
