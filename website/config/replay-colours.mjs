/**
 * The colours of a replay's code, from the engine that colours a `<Snippet>`.
 *
 * `<Snippet>` colours code through Starlight's `<Code>` component, which is
 * Expressive Code with the grammar in `novis.tmLanguage.json` and the site's
 * light and dark themes. This module builds the same engine the way
 * `astro-expressive-code`'s own `<Code>` component does, from the integration's
 * virtual modules, so a replay and a snippet show the same colours. Those
 * modules exist only while Astro builds the site, so only
 * `src/components/Replay.astro` imports this file.
 *
 * `colourLines` gives each line of a text as runs `[text, style]`. A run's
 * style is the token's `style` attribute, `--0:<dark colour>;--1:<light
 * colour>` and the font style when the theme sets one, and is empty for text
 * that has no token colour. `replay-frames.mjs` turns the runs into the
 * frames' colours.
 */

let engine = null

/** The Expressive Code engine of the `<Code>` component, built once per build. */
function expressiveCode() {
  engine ??= (async () => {
    const { astroConfig, ecConfigFileOptions, ecIntegrationOptions = {} } = await import('virtual:astro-expressive-code/config')
    const { createAstroRenderer, mergeEcConfigOptions } = await import('virtual:astro-expressive-code/api')
    const { default: preprocess } = await import('virtual:astro-expressive-code/preprocess-config')
    const merged = mergeEcConfigOptions(ecIntegrationOptions, ecConfigFileOptions)
    const ecConfig = (await preprocess({ ecConfig: merged, astroConfig })) || merged
    return (await createAstroRenderer({ astroConfig, ecConfig })).ec
  })()
  return engine
}

const classes = (node) => [node.properties?.className ?? []].flat()

/** Each element under `node` whose class list has `name`, in document order. */
function withClass(node, name, found = []) {
  if (node.type === 'element' && classes(node).includes(name)) found.push(node)
  else for (const child of node.children ?? []) withClass(child, name, found)
  return found
}

/** The runs of text under `node`, each with the style of the nearest element above it that has one. */
function runs(node, style, out) {
  if (node.type === 'text') {
    if (out.length > 0 && out.at(-1)[1] === style) out.at(-1)[0] += node.value
    else out.push([node.value, style])
  } else {
    const own = node.properties?.style
    for (const child of node.children ?? []) runs(child, typeof own === 'string' ? own : style, out)
  }
  return out
}

/** Each line of `text`, coloured the way `<Snippet>` colours Novis code, as runs `[text, style]`. */
export async function colourLines(text) {
  const ec = await expressiveCode()
  const { renderedGroupAst } = await ec.render({ code: text, language: 'novis', meta: '' })
  // Expressive Code writes an empty line as a line break, so the line keeps its height.
  return withClass(renderedGroupAst, 'ec-line').map((line) => runs(withClass(line, 'code')[0] ?? line, '', []).filter(([t]) => t !== '\n'))
}
