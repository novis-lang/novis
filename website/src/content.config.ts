import { defineCollection, z } from 'astro:content'
import { docsLoader } from '@astrojs/starlight/loaders'
import { docsSchema } from '@astrojs/starlight/schema'

export const collections = {
  docs: defineCollection({
    loader: docsLoader(),
    schema: docsSchema({
      extend: z.object({
        /** The features a handwritten page explains, as the proofs roster names them. `bun nv site` reads it. */
        covers: z.array(z.string()).optional(),
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
}
