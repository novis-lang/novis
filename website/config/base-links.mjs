/**
 * Astro integration: rewrite root-absolute links in Markdown/MDX content to
 * include the site's `base` path (set via the ASTRO_BASE env var or the
 * `--base` CLI flag; see astro.config.ts).
 *
 * The content files link with root-absolute URLs (`/reference/...`). Astro only
 * prefixes `base` onto URLs it generates itself, so without this pass a
 * subfolder deploy breaks every content link. Rewriting at build time keeps
 * the sources deploy-agnostic. An integration rather than config-level
 * plugins because only the resolved config knows a base passed on the CLI,
 * and because the default Sätteri processor takes its plugins on the
 * processor object, not from `markdown.rehypePlugins` (mirroring how
 * Starlight registers its own transforms). Component-authored links use
 * withBase() from src/lib/base.ts instead — component output never passes
 * through the markdown pipeline.
 */
import { isSatteriProcessor } from '@astrojs/markdown-satteri'

export function baseLinks() {
  return {
    name: 'novis:base-links',
    hooks: {
      'astro:config:setup'({ config, updateConfig }) {
        const base = config.base.replace(/\/+$/, '')
        if (base === '') return
        const processor = config.markdown.processor
        if (processor && isSatteriProcessor(processor)) {
          processor.options.hastPlugins.push(satteriBaseLinks(base))
        } else {
          // The unified pipeline (`@astrojs/markdown-remark`) still reads the
          // classic plugin lists from the markdown config.
          updateConfig({ markdown: { rehypePlugins: [[rehypeBaseLinks, { base }]] } })
        }
      },
    },
  }
}

/** `base + value` for a root-absolute URL, undefined for anything else. */
function prefixed(base, value) {
  return typeof value === 'string' && value.startsWith('/') && !value.startsWith('//')
    ? base + value
    : undefined
}

/** href/src attributes with a root-absolute URL, inside raw HTML text. */
const rawUrlAttr = /(href|src)="(\/[^/"][^"]*|\/)"/g

/** Sätteri hast plugin: prefix `base` onto every root-absolute href/src. */
function satteriBaseLinks(base) {
  return {
    name: 'novis-base-links',
    element: {
      filter: ['a', 'img', 'source', 'video', 'audio'],
      visit(node, ctx) {
        for (const key of ['href', 'src']) {
          const value = prefixed(base, node.properties?.[key])
          if (value !== undefined) ctx.setProperty(node, key, value)
        }
      },
    },
    // Raw HTML embedded in the markdown (e.g. the ADR meta rows the sync
    // script writes) never reaches the element visitor.
    raw(node, ctx) {
      const value = node.value.replace(rawUrlAttr, (_, attr, url) => `${attr}="${base}${url}"`)
      if (value !== node.value) ctx.replaceNode(node, { type: 'raw', value })
    },
  }
}

/** Rehype plugin: the same rewrite for the unified pipeline. */
function rehypeBaseLinks({ base }) {
  const visit = (node) => {
    if (node.type === 'element' && node.properties) {
      for (const key of ['href', 'src']) {
        const value = prefixed(base, node.properties[key])
        if (value !== undefined) node.properties[key] = value
      }
    } else if (node.type === 'raw' && typeof node.value === 'string') {
      node.value = node.value.replace(rawUrlAttr, (_, attr, url) => `${attr}="${base}${url}"`)
    }
    if (node.children) node.children.forEach(visit)
  }
  return (tree) => visit(tree)
}
