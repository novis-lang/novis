- **`rules.py --check` reads `docs/agent/handoff.md`, so a `rule:` token invented in a *handoff* turns
  a floor check red for the next session rather than for the one that wrote it.** Session 0006's
  `## Next group` cited `rule:http-server/containment-does-not-end-at-the-helper` with its tail cut
  off, and the driver's earliest-stage failure became "the rulebook validates", with findings
  pointing at the handoff rather than at any rule. Paste a `rule:` token from `brief.py --where
  <topic>` rather than shortening one to the words you remember — **including in the bullet you write
  about the mistake**, since this file is scanned too and quoting the broken token here keeps the
  check red. [until: reviewed 2026-09-10]
