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
 * The five areas of the site, in the order a reader learns them. The header
 * shows each label with its subtitle on a second line, and each area has its
 * own sidebar: the top-level group in astro.config.ts whose label is the
 * area's label. `bun nv site --check structure` reads this list.
 */
export const AREAS = [
  { label: 'Guides', subtitle: 'Examples, how to use', href: '/guides/' },
  { label: 'Syntax', subtitle: 'How to write Novis', href: '/syntax/' },
  { label: 'Reference', subtitle: 'Classes and functions', href: '/reference/' },
  { label: 'Tools', subtitle: 'Editor, tests, debugger', href: '/tools/' },
  { label: 'In-Depth', subtitle: 'Decisions, ideas', href: '/in-depth/' },
]

/** The install pages: a button in the header, after the five areas, with a sidebar of its own. */
export const INSTALL = { label: 'Install', href: '/install/' }

/**
 * The ways to give money, in the order the sponsoring page lists them. A
 * channel whose `url` is empty has no account yet: the page shows it as
 * opening soon. Set the URL when the account exists. Bank transfer has no
 * URL by design: it points at the legal notice, where the contact is.
 */
export const SPONSOR_CHANNELS = [
  {
    label: 'GitHub Sponsors',
    url: '',
    text: 'For developers and teams. Give monthly or once, with your GitHub account. GitHub takes no fee when you give from a personal account.',
  },
  {
    label: 'Polar',
    url: '',
    text: 'For a gift without a GitHub account. You can pay by card, Apple Pay, Google Pay and many other methods. Polar handles the VAT and sales tax, and you get a receipt.',
  },
  {
    label: 'Bank transfer',
    url: '/impressum/',
    text: 'For companies and custom agreements. Write to us, and you get an invoice. We use the Austrian small business exemption, so the invoice has no VAT.',
  },
]

/**
 * Deep link into the repository tree (for e.g. `docs/rules/security/tainted-sources.md`).
 * @param {string} repoRelativePath
 * @returns {string}
 */
export function githubFile(repoRelativePath) {
  return `${GITHUB_URL}/blob/${GITHUB_BRANCH}/${repoRelativePath.replace(/^\/+/, '')}`
}
