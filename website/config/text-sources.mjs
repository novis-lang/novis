/**
 * Astro integration: in dev, load a bare Novis text file as an empty module.
 *
 * The pages read `.nvs`, `.out` and their siblings with `import.meta.glob(..., { query: '?raw' })`.
 * Vite's raw loader calls `addWatchFile`, which puts the bare file into the module graph as an
 * import of the `?raw` module. Astro's dev CSS collector (`ensureModulesLoaded` in
 * astro/dist/vite-plugin-css) then fetches every untransformed module it reaches, the bare file
 * included, so Vite's import analysis parses Novis source as JavaScript. Any `Feed::import($x)` in
 * an example reads as a dynamic `import()` and prints "cannot be analyzed by Vite". Nothing
 * imports these files except through `?raw`, so the bare module's code is never run, and an empty
 * module keeps it out of the parser while the file stays watched.
 */
const TEXT_SOURCE = /\.(?:nvs|cmd|out|err|tainted|replay|toml)$/

export function textSources() {
  return {
    name: 'novis:text-sources',
    hooks: {
      'astro:config:setup'({ updateConfig }) {
        updateConfig({
          vite: {
            plugins: [
              {
                name: 'novis:text-sources',
                apply: 'serve',
                enforce: 'pre',
                load: {
                  filter: { id: TEXT_SOURCE },
                  handler: () => 'export {}',
                },
              },
            ],
          },
        })
      },
    },
  }
}
