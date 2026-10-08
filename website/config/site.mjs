/**
 * The one place site-wide constants live. Everything else — astro.config.ts,
 * components, sync scripts — imports from here, so going live is editing this
 * file once. Plain .mjs so both Astro and the Node sync scripts can import it.
 *
 * The repository and the Discord server are real. ⚠ SITE_URL is still the
 * intended domain rather than a deployed one — swap it when the site goes live.
 */

/** Canonical origin of the deployed site. Drives sitemap.xml, canonical URLs and Open Graph tags. */
export const SITE_URL = 'https://novis-lang.org'

/** Public repository. Drives the GitHub header icon and source cross-references. */
export const GITHUB_URL = 'https://github.com/novis-lang/novis'

/** Community server. Drives the Discord header icon. */
export const DISCORD_URL = 'https://discord.gg/8ftMjPeH8h'

/**
 * The contact address the legal notice and the privacy policy print. The page
 * HTML never carries it in plain text: src/components/ProtectedEmail.astro
 * builds it in the browser.
 */
export const CONTACT_EMAIL = 'novis-lang@proton.me'

/** Branch the GitHub links point into. */
export const GITHUB_BRANCH = 'main'

/** Site title, used in the header and the `<title>` suffix. */
export const SITE_TITLE = 'Novis'

/** Default description for pages that state none. */
export const SITE_DESCRIPTION =
  "Novis — a programming language for the web. Secure by design, not by discipline: untrusted data is tracked and blocked from anywhere dangerous, secrets can't leak, and downloaded code does only what you allow. Fast, like you would expect."

/**
 * The four areas of the site, in the order a reader learns them. The header
 * shows each label with its subtitle on a second line, and each area has its
 * own sidebar: the top-level group in astro.config.ts whose label is the
 * area's label. `bun nv site --check structure` reads this list.
 */
export const AREAS = [
  { label: 'Guides', subtitle: 'Examples, how to use', href: '/guides/' },
  { label: 'Syntax', subtitle: 'How to write Novis', href: '/syntax/' },
  { label: 'Reference', subtitle: 'Classes and functions', href: '/reference/' },
  { label: 'In-Depth', subtitle: 'Decisions, ideas', href: '/in-depth/' },
]

/** The install pages: a button in the header, after the four areas, with a sidebar of its own. */
export const INSTALL = { label: 'Install', href: '/install/' }

/**
 * Deep link into the repository tree (for e.g. `docs/rules/security/tainted-sources.md`).
 * @param {string} repoRelativePath
 * @returns {string}
 */
export function githubFile(repoRelativePath) {
  return `${GITHUB_URL}/blob/${GITHUB_BRANCH}/${repoRelativePath.replace(/^\/+/, '')}`
}

/**
 * The frozen decision record behind a rule, by its four-digit number. The
 * records are rationale the repository keeps and the site does not publish:
 * a rule says what is true now, and its `because` points back at why it was
 * decided. One home, in the repository.
 * @param {string} number four digits, e.g. '0046'
 * @returns {string}
 */
export function decisionRecord(number) {
  return githubFile(`docs/decisions/${number}.md`)
}
