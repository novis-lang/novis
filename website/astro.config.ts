import { defineConfig } from 'astro/config'
import starlight from '@astrojs/starlight'

// No `site` is set yet: nothing deploys this. Add it when the site gets a home,
// together with whatever workflow puts it there.
export default defineConfig({
  integrations: [
    starlight({
      title: 'Novis',
      description: 'A JIT-compiled, memory-safe language for web servers and the command line.',
      sidebar: [],
      customCss: ['./src/styles/custom.css'],
    }),
  ],
})
