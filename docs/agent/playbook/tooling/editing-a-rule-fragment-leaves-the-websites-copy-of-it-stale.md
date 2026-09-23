- **Editing a rule fragment leaves the website's copy of it stale, and nothing turns red.**
  `python tools/rules.py --render` rewrites `docs/rules/<topic>.md` and `verify.py` gates that,
  but `website/src/content/docs/docs/rules/` is a second rendering no Python tool touches — it was
  already a goal behind at this session's HEAD. Run `node scripts/sync-rules.mjs` from `website/`
  after any fragment or `<topic>.json` edit, and expect the diff to carry whatever the sessions
  before you left behind as well as your own rule.
  [until: exists tools/verify.py:sync-rules]
