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
        /** The classes an example app page shows, each a class name or a namespace (`Novis\\Pdf`). Every
         * Core class page it matches lists the page under "Example apps": src/lib/example-apps.ts. */
        shows: z.array(z.string()).optional(),
      }),
    }),
  }),
}
