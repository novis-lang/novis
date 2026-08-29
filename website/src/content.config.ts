import { defineCollection, z } from 'astro:content'
import { docsLoader } from '@astrojs/starlight/loaders'
import { docsSchema } from '@astrojs/starlight/schema'
import { glob } from 'astro/loaders'

export const collections = {
  docs: defineCollection({
    loader: docsLoader(),
    schema: docsSchema({
      extend: z.object({
        /** Novis-specific page metadata, set by the sync tooling's stubs. */
        novis: z
          .object({
            kind: z.enum(['method', 'class']),
            /** Member or class id in src/data/core.json, e.g. "Str.length". */
            id: z.string(),
            /** True until a human has reviewed the generated draft prose. */
            draft: z.boolean().default(false),
          })
          .optional(),
      }),
    }),
  }),

  /**
   * The "Why Novis?" claims. One file per claim under src/content/claims/,
   * so adding a claim is adding a file — no page edit involved.
   */
  claims: defineCollection({
    loader: glob({ base: './src/content/claims', pattern: '**/*.md' }),
    schema: z.object({
      /** The claim, one bold sentence. */
      claim: z.string(),
      /** Grouping on the page. */
      category: z.enum(['security', 'performance', 'simplicity', 'tooling', 'compatibility', 'honesty']),
      /** What we compare against. */
      comparedTo: z.array(z.string()).default([]),
      /** Where the proof lives: an ADR number, a benchmark, a spec section. */
      proof: z.string().optional(),
      /** The honest cost of the decision, stated plainly. Required for real claims. */
      tradeoff: z.string().optional(),
      /** True while the claim is unreviewed. Draft claims are visibly marked. */
      draft: z.boolean().default(false),
      /** Sort weight inside its category (lower first). */
      weight: z.number().default(100),
    }),
  }),
}
