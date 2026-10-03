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
      }),
    }),
  }),
}
