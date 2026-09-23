- **An acceptance check can be red over a *doc* the last session's own wrap wrote, and the fix is a
  sentence in the file you are about to overwrite anyway.** `chain.py --check`, `rules.py --check`
  and `check-links.py` scan the whole tree, `docs/agent/handoff.md` included, so a handoff sentence
  can halt a DONE claim the session that wrote it reported green — the tell is a failure detail
  naming `docs/agent/*.md:NN` rather than a crate path. Fix it in the wrap's own `## handoff` or
  `## playbook:` section, never by hand: step 4 rewrites both files, so a hand edit is overwritten
  by the very call that is supposed to land it. [until: reviewed 2026-09-18]
