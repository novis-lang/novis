import { defineCollection, z } from 'astro:content';
import { docsLoader } from '@astrojs/starlight/loaders';
import { docsSchema } from '@astrojs/starlight/schema';

/**
 * Starlight's schema, extended with the three fields this project's tooling
 * needs. All three are optional, so a page written without thinking about them
 * still builds — the tooling then tells you what it could not check.
 *
 *   edit     Who may rewrite this page's prose.
 *              open    — tooling and agents may regenerate it freely
 *              review  — they may propose, a human merges (the default for
 *                        anything an agent has written and a human has read)
 *              locked  — nothing automated ever touches it again
 *            Regardless of this flag, `<!-- keep -->…<!-- /keep -->` regions
 *            inside the body are preserved verbatim by every tool.
 *
 *   sources  Repository files this page was written from, each stamped with the
 *            git blob hash it had at the last review: `adr/0006-…md@a1b2c3d`.
 *            `site.py check` reports a page whose source has moved on. This is
 *            what keeps one fact in one place while letting this site say it
 *            in different words.
 *
 *   reviewed ISO date a human last read the whole page. Informational; shown
 *            nowhere, used by `site.py check --stale` to sort the work queue.
 */
export const collections = {
  docs: defineCollection({
    loader: docsLoader(),
    schema: docsSchema({
      extend: z.object({
        edit: z.enum(['open', 'review', 'locked']).default('open'),
        sources: z.array(z.string()).default([]),
        reviewed: z.string().optional(),
      }),
    }),
  }),
};
