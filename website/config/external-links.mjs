/**
 * The one home of the external-link rule: a link that leaves the site opens
 * in a new tab and carries `rel="noopener noreferrer nofollow"`. `noopener`
 * keeps the opened page off our `window`, `noreferrer` keeps our URL out of
 * its logs, `nofollow` says we vouch for nothing we link to. The project's
 * own pages elsewhere — the repository and the Discord server, OWN_LINKS
 * below — are the exception: they open in a new tab too, but carry no `rel`
 * of ours, since we do vouch for them and want them followed.
 *
 * Links reach the built HTML by two roads, and the rule has to meet both:
 *
 * - Markdown/MDX content passes through the markdown pipeline, where the
 *   `externalLinks()` integration below decorates every `<a>` — the ones
 *   markdown made and the ones embedded as raw HTML (the ADR meta rows the
 *   sync script writes). It is shaped like base-links.mjs, and for the same
 *   reasons: the default Sätteri processor takes its plugins on the processor
 *   object, and the unified pipeline still reads `markdown.rehypePlugins`.
 * - Component output never passes through the pipeline, so a component
 *   spreads `externalLinkAttrs(href)` onto the anchor it writes. The Starlight
 *   component that links out (SocialIcons) is overridden in src/components/
 *   to do the same. A JSX `<a>` written in an MDX page is a
 *   component too — the visitor sees only markdown-made elements and raw
 *   HTML — so it spreads the same helper (sponsoring.mdx does).
 *
 * "External" is any absolute `http(s)` or protocol-relative URL whose origin
 * is not SITE_URL's. `mailto:` and `tel:` are not websites and stay as they
 * are; the site's own absolute URLs stay in-tab.
 */
import { isSatteriProcessor } from '@astrojs/markdown-satteri'
import { SITE_URL, GITHUB_URL, DISCORD_URL } from './site.mjs'

/** The `rel` every external link carries, except one of our own. */
export const EXTERNAL_REL = 'noopener noreferrer nofollow'

/** Prefixes of the project's own pages off the site: new tab, no `rel`. */
export const OWN_LINKS = [GITHUB_URL, DISCORD_URL]

const siteOrigin = new URL(SITE_URL).origin

/**
 * Whether `href` is one of the project's own pages elsewhere — the
 * repository, anything under it (a blob link), the Discord invite.
 * @param {string} href
 * @returns {boolean}
 */
function isOwn(href) {
  return OWN_LINKS.some((own) => href === own || href.startsWith(own + '/') || href.startsWith(own + '?'))
}

/**
 * Whether `href` points at another website. A `URL` counts as its string.
 * @param {unknown} href
 * @returns {boolean}
 */
export function isExternal(href) {
  if (href instanceof URL) href = href.href
  if (typeof href !== 'string' || !/^(?:https?:)?\/\//i.test(href)) return false
  try {
    return new URL(href, SITE_URL).origin !== siteOrigin
  } catch {
    return false
  }
}

/**
 * The attributes a component spreads onto an anchor: `{}` for a link that
 * stays on the site, `target` alone for one of our own pages elsewhere,
 * `target` and `rel` for one that leaves the project. An existing `rel`
 * (Starlight's social icons carry `rel="me"`) is kept, and extended where
 * the rule adds tokens.
 * @param {unknown} href
 * @param {string} [rel]
 * @returns {{ target?: string; rel?: string }}
 */
export function externalLinkAttrs(href, rel) {
  if (!isExternal(href)) return rel ? { rel } : {}
  if (isOwn(String(href))) return rel ? { target: '_blank', rel } : { target: '_blank' }
  return { target: '_blank', rel: mergeRel(rel) }
}

/** `EXTERNAL_REL` plus whatever tokens `rel` already carried, without repeats. */
function mergeRel(rel) {
  const tokens = new Set(EXTERNAL_REL.split(' '))
  for (const token of String(rel ?? '').split(/\s+/)) if (token) tokens.add(token)
  return [...tokens].join(' ')
}

/** Astro integration: apply the rule to every anchor markdown content emits. */
export function externalLinks() {
  return {
    name: 'novis:external-links',
    hooks: {
      'astro:config:setup'({ config, updateConfig }) {
        const processor = config.markdown.processor
        if (processor && isSatteriProcessor(processor)) {
          processor.options.hastPlugins.push(satteriExternalLinks())
        } else {
          updateConfig({ markdown: { rehypePlugins: [rehypeExternalLinks] } })
        }
      },
    },
  }
}

/** Sätteri hast plugin: `target` and `rel` on every external `<a>`. */
function satteriExternalLinks() {
  return {
    name: 'novis-external-links',
    element: {
      filter: ['a'],
      visit(node, ctx) {
        const attrs = externalLinkAttrs(node.properties?.href, relOf(node.properties?.rel))
        if (!attrs.target) return
        ctx.setProperty(node, 'target', attrs.target)
        if (attrs.rel) ctx.setProperty(node, 'rel', attrs.rel)
      },
    },
    raw(node, ctx) {
      const value = decorateRawAnchors(node.value)
      if (value !== node.value) ctx.replaceNode(node, { type: 'raw', value })
    },
  }
}

/** Rehype plugin: the same rule for the unified pipeline. */
function rehypeExternalLinks() {
  const visit = (node) => {
    if (node.type === 'element' && node.tagName === 'a' && node.properties) {
      const attrs = externalLinkAttrs(node.properties.href, relOf(node.properties.rel))
      if (attrs.target) {
        node.properties.target = attrs.target
        if (attrs.rel) node.properties.rel = attrs.rel
      }
    } else if (node.type === 'raw' && typeof node.value === 'string') {
      node.value = decorateRawAnchors(node.value)
    }
    if (node.children) node.children.forEach(visit)
  }
  return (tree) => visit(tree)
}

/** hast stores `rel` as a token list; components hand over a string. */
function relOf(value) {
  return Array.isArray(value) ? value.join(' ') : value
}

/** An opening `<a>` tag, with its attributes captured. */
const rawAnchor = /<a\s+([^>]*?)\s*\/?>/gi
const rawAttr = (name) => new RegExp(`\\b${name}\\s*=\\s*"([^"]*)"`, 'i')

/**
 * The rule applied to anchors inside raw HTML, which never reach an element
 * visitor. Attributes already on the tag are kept: `target` wins if present,
 * `rel` is extended where the rule adds tokens.
 * @param {string} html
 * @returns {string}
 */
export function decorateRawAnchors(html) {
  if (!html.includes('<a')) return html
  return html.replace(rawAnchor, (tag, attrs) => {
    const href = rawAttr('href').exec(attrs)?.[1]
    if (!isExternal(href)) return tag
    const target = rawAttr('target').test(attrs) ? '' : ' target="_blank"'
    if (isOwn(href)) return `<a ${attrs}${target}>`
    let rest = attrs
    const rel = rawAttr('rel').exec(attrs)?.[1]
    if (rel !== undefined) rest = rest.replace(rawAttr('rel'), ' ').replace(/\s+/g, ' ').trim()
    return `<a ${rest}${target} rel="${mergeRel(rel)}">`
  })
}
