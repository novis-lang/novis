- **Adding a bullet here can turn `python tools/chain.py --check` red, because the live goal's
  `[context] playbook` selector then matches two bullets.** A selector is a lead-in prefix
  (`Writing a test case > a tds`), so a bullet whose first words repeat an existing one makes it
  ambiguous, and the check names the *goal file* rather than the playbook the edit landed in. End
  the selector in `*` to take every match, in `docs/agent/loop-goal.toml` and in the
  `docs/agent/goals/<goal>.toml` copy both. [until: reviewed 2026-09-17]
