/**
 * The site's `base` path (Astro's `base` option — set via the ASTRO_BASE env
 * var or the `--base` CLI flag; see astro.config.ts) prefixed onto a
 * root-absolute path. At a root deploy this is the identity function.
 *
 * Markdown/MDX-authored links are rewritten at build time by
 * config/base-links.mjs; this helper is for links authored in components and
 * libraries, whose output never passes through that rewrite.
 */
const base = import.meta.env.BASE_URL.replace(/\/+$/, '')

export function withBase(path: string): string {
  return `${base}${path.startsWith('/') ? '' : '/'}${path}`
}
