/**
 * The visitor statistics: one page view per page load, sent to the Matomo
 * server in STATS (config/site.mjs). src/content/docs/datenschutz.mdx
 * § *Visitor statistics* is what the reader is promised, and this file is
 * the client half of keeping it.
 *
 * No Matomo JavaScript runs on the site. Its tracker reads the screen size,
 * the plugins and more, and sets cookies unless told not to; this file sends
 * the Tracking HTTP API's few required fields itself, so the reader's
 * browser runs only code from this repository. What is sent:
 *
 * - the page address without its query string or fragment, so nothing a
 *   reader typed into a URL reaches the server;
 * - the page title;
 * - the referrer's origin only, and only when it is another site.
 *
 * No `_id`, no cookie, no screen size and no plugin flags. Matomo then builds
 * its visit hash from the shortened IP address, the user agent and the
 * language header, with a salt it replaces every day, so a reader cannot be
 * followed from one day to the next.
 *
 * Nothing is sent when the reader turned the statistics off on the privacy
 * page, when the browser sends Global Privacy Control or Do Not Track, in a
 * dev build, or on any host but SITE_URL's: a fork, a preview or a local
 * build never counts.
 *
 * The request is a `no-cors` POST with `credentials: 'omit'` and no
 * Referer header: the browser sends no cookie to the stats host, and the page
 * reads nothing back from it.
 */
import { SITE_URL, STATS } from '@config/site.mjs'

/** The local storage key the privacy page's switch sets, `'1'` when the reader turned counting off. */
const OPT_OUT_KEY = 'nv-stats-opt-out'

/** Whether the browser asks sites not to track: Global Privacy Control or Do Not Track. */
export function browserDeclines(): boolean {
  const nav = navigator as Navigator & { globalPrivacyControl?: boolean; doNotTrack?: string | null }
  return nav.globalPrivacyControl === true || nav.doNotTrack === '1'
}

/** Whether the reader turned counting off on the privacy page. */
export function optedOut(): boolean {
  try {
    return localStorage.getItem(OPT_OUT_KEY) === '1'
  } catch {
    // Storage blocked: the reader could not have opted out in this browser.
    return false
  }
}

/** Stores the reader's choice. Opting back in removes the key, so a counted reader stores nothing. */
export function setOptedOut(out: boolean): void {
  try {
    if (out) localStorage.setItem(OPT_OUT_KEY, '1')
    else localStorage.removeItem(OPT_OUT_KEY)
  } catch {
    // Storage blocked: the switch cannot remember, and the page says nothing was stored.
  }
}

/** The referrer's origin when it is another site, or null. */
function externalReferrer(): string | null {
  if (!document.referrer) return null
  try {
    const ref = new URL(document.referrer)
    return ref.hostname === location.hostname ? null : ref.origin
  } catch {
    return null
  }
}

/** Sends one page view for the current page, unless any rule above says not to. */
export function trackPageView(): void {
  if (!import.meta.env.PROD || !STATS.url) return
  if (location.hostname !== new URL(SITE_URL).hostname) return
  if (browserDeclines() || optedOut()) return

  const body = new URLSearchParams({
    idsite: String(STATS.siteId),
    rec: '1',
    apiv: '1',
    // A 204 with no body, in place of the 1x1 GIF.
    send_image: '0',
    url: location.origin + location.pathname,
    action_name: document.title,
  })
  const ref = externalReferrer()
  if (ref) body.set('urlref', ref)

  fetch(new URL('matomo.php', STATS.url), {
    method: 'POST',
    body,
    mode: 'no-cors',
    credentials: 'omit',
    referrerPolicy: 'no-referrer',
    keepalive: true,
  }).catch(() => {
    // A blocked or failed count changes nothing for the reader.
  })
}
