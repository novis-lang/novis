/**
 * ADR hover tooltips, site-wide.
 *
 * Every link whose target is an ADR page (`/docs/adr/NNNN/`) gets an instant
 * tooltip on hover and on keyboard focus, showing the ADR's number, title,
 * status and its "In short" summary. Clicking still navigates.
 *
 * The index is bundled from src/data/adrs.json at build time — no fetch, so
 * the tooltip is instant. Accessible: the card is linked to the trigger via
 * aria-describedby while shown, follows focus, and Escape dismisses it.
 */
import adrs from '../data/adrs.json'

type AdrEntry = (typeof adrs)[number]

const byNumber = new Map<string, AdrEntry>(adrs.map((a) => [a.number, a]))
const ADR_HREF = /\/docs\/adr\/(\d{4})\/?(?:#.*)?$/

let card: HTMLDivElement | null = null
let currentAnchor: HTMLAnchorElement | null = null

function ensureCard(): HTMLDivElement {
  if (card) return card
  card = document.createElement('div')
  card.id = 'nv-adr-tooltip'
  card.setAttribute('role', 'tooltip')
  card.hidden = true
  document.body.appendChild(card)
  return card
}

function show(anchor: HTMLAnchorElement, adr: AdrEntry) {
  const el = ensureCard()
  currentAnchor = anchor
  el.innerHTML = ''

  const head = document.createElement('div')
  head.className = 'nv-adr-tooltip-head'
  const num = document.createElement('span')
  num.className = 'nv-adr-tooltip-number'
  num.textContent = `ADR ${adr.number}`
  const status = document.createElement('span')
  status.className = 'nv-adr-tooltip-status'
  status.dataset.status = adr.status.toLowerCase()
  status.textContent = adr.status
  head.append(num, status)

  const title = document.createElement('p')
  title.className = 'nv-adr-tooltip-title'
  title.textContent = adr.title

  el.append(head, title)

  if (adr.inShort) {
    const summary = document.createElement('p')
    summary.className = 'nv-adr-tooltip-summary'
    summary.textContent = adr.inShort.length > 280 ? adr.inShort.slice(0, 277) + '…' : adr.inShort
    el.append(summary)
  }

  // Position: below the anchor, clamped to the viewport.
  const rect = anchor.getBoundingClientRect()
  el.hidden = false
  const width = Math.min(380, window.innerWidth - 24)
  el.style.maxWidth = `${width}px`
  const left = Math.max(12, Math.min(rect.left, window.innerWidth - width - 12))
  const top = rect.bottom + 8
  el.style.left = `${left + window.scrollX}px`
  // Flip above when there is no room below.
  const height = el.offsetHeight
  if (top + height > window.innerHeight - 12 && rect.top - height - 8 > 0) {
    el.style.top = `${rect.top - height - 8 + window.scrollY}px`
  } else {
    el.style.top = `${top + window.scrollY}px`
  }

  anchor.setAttribute('aria-describedby', 'nv-adr-tooltip')
}

function hide() {
  if (card) card.hidden = true
  if (currentAnchor) {
    currentAnchor.removeAttribute('aria-describedby')
    currentAnchor = null
  }
}

function anchorFor(target: EventTarget | null): { anchor: HTMLAnchorElement; adr: AdrEntry } | null {
  if (!(target instanceof Element)) return null
  const anchor = target.closest('a')
  if (!anchor || !(anchor instanceof HTMLAnchorElement)) return null
  const match = ADR_HREF.exec(anchor.getAttribute('href') ?? '')
  if (!match) return null
  // No tooltip on the ADR index's own rows — the summary is already visible there.
  if (anchor.closest('[data-nv-no-adr-tooltip]')) return null
  const adr = byNumber.get(match[1])
  if (!adr) return null
  // Not on the page describing itself.
  if (ADR_HREF.exec(location.pathname)?.[1] === match[1]) return null
  return { anchor, adr }
}

document.addEventListener('mouseover', (event) => {
  const found = anchorFor(event.target)
  if (found) show(found.anchor, found.adr)
})
document.addEventListener('mouseout', (event) => {
  if (anchorFor(event.target)) hide()
})
document.addEventListener('focusin', (event) => {
  const found = anchorFor(event.target)
  if (found) show(found.anchor, found.adr)
  else hide()
})
document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape') hide()
})
document.addEventListener('scroll', hide, { passive: true })
