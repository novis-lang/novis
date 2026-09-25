- **Editing a rule fragment leaves the website's copy of it stale, and nothing local turns red.**
  `bun nv rules --render` rewrites `docs/rules/<topic>.md` and `nv verify` gates that, but the
  website's pages under `website/src/content/docs/docs/rules/` and `website/src/data/rules.json` are
  a second rendering, and only CI's `bun nv render --check` gates it. Run `bun nv render --website`
  after any fragment or `data/rules/` edit, and commit the pages with the rule.
  [until: exists tools/nv/cmd/verify.ts:WEBSITE_RENDERERS]
