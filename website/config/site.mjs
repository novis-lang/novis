/**
 * The one place site-wide constants live. Everything else — astro.config.ts,
 * components, sync scripts — imports from here, so going live is editing this
 * file once. Plain .mjs so both Astro and the Node sync scripts can import it.
 *
 * ⚠ PLACEHOLDERS: none of these URLs is real yet. Swap them when the site
 * gets a home, a repository host and a community server.
 */

/** Canonical origin of the deployed site. Drives sitemap.xml, canonical URLs and Open Graph tags. */
export const SITE_URL = 'https://novis-lang.org'

/** Public repository. Drives the GitHub header icon, edit links and source cross-references. */
export const GITHUB_URL = 'https://github.com/novis-lang/novis'

/** Community server. Drives the Discord header icon. */
export const DISCORD_URL = 'https://discord.gg/novis-placeholder'

/** Branch the GitHub links point into. */
export const GITHUB_BRANCH = 'main'

/** Site title, used in the header and the `<title>` suffix. */
export const SITE_TITLE = 'Novis'

/** Default description for pages that state none. */
export const SITE_DESCRIPTION =
  "Novis — a programming language for the web. Secure by design, not by discipline: untrusted data is tracked and blocked from anywhere dangerous, secrets can't leak, and downloaded code does only what you allow. Fast, like you would expect."

/**
 * Deep link into the repository tree (for e.g. `docs/adr/0002-….md`).
 * @param {string} repoRelativePath
 * @returns {string}
 */
export function githubFile(repoRelativePath) {
  return `${GITHUB_URL}/blob/${GITHUB_BRANCH}/${repoRelativePath.replace(/^\/+/, '')}`
}
