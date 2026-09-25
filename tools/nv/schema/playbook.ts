// The playbook: a section is `data/playbook/<section>.json`, and each bullet in it is
// `data/playbook/<section>/<slug>.json`. A bullet is a trap: `lead` is the claim a reader matches against
// their symptom, `body` is why and what to do instead, and `until` is the condition that retires it.
//
// `until`'s kinds and what each tests are `tools/nv/cmd/playbook.ts`'s module doc.

import { defineRecord, s } from "../lib/schema.ts";

export const playbookSection = defineRecord({
  name: "playbook_section",
  dir: "playbook",
  schema: s.object({ title: s.string(), order: s.int() }),
  idOf: (sub) => (/^[a-z0-9-]+\.json$/.test(sub) ? sub.slice(0, -".json".length) : null),
});

export const playbookBullet = defineRecord({
  name: "playbook_bullet",
  dir: "playbook",
  schema: s.object({
    lead: s.string(),
    body: s.string(),
    /** The repo-rooted files the trap is about, which a goal's manifest selects it by. */
    files: s.array(s.string()),
    until: s.object({
      kind: s.enum("test", "exists", "gone", "rule"),
      arg: s.string(),
    }),
  }),
  idOf: (sub) => (/^[a-z0-9-]+\/[a-z0-9-]+\.json$/.test(sub) ? sub.slice(0, -".json".length) : null),
  checks: [
    {
      name: "a bullet is in a section",
      sql: `SELECT path, 'no section ' || substr(id, 1, instr(id, '/') - 1) AS detail FROM playbook_bullet
            WHERE substr(id, 1, instr(id, '/') - 1) NOT IN (SELECT id FROM playbook_section)`,
    },
    {
      name: "a bullet names a file",
      sql: `SELECT path, 'names no file' AS detail FROM playbook_bullet
            WHERE id NOT IN (SELECT owner FROM playbook_bullet__files)`,
    },
  ],
});
