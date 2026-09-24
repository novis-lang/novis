// A reference chapter's fields: `data/reference/<path>.json` for `docs/reference/<path>.md`. They are
// the chapter's front matter, which is rendered from here, and what the one-file reference and the
// help's `find` index it by.

import { defineRecord, s } from "../lib/schema.ts";

export const referenceChapter = defineRecord({
  name: "reference_chapter",
  dir: "reference",
  schema: s.object({
    /** The chapter's short name in the one-file reference, for a chapter that is one of its parts. */
    slug: s.optional(s.slug()),
    title: s.optional(s.string()),
    summary: s.string(),
    keywords: s.array(s.string()),
  }),
});
