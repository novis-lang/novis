/**
 * Browser: a browser window with a tab, an address bar, the page, and the Network panel of its
 * developer tools.
 *
 * - `screen(name)` cross-fades to one of the page's screens, `[data-screen]` in the markup. The
 *   elements of a screen marked `data-stagger` arrive one after another.
 * - `loading(on)` turns the tab's spinner on or off.
 * - `request({ method, path })` adds a row to the Network panel and returns it; `row.finish(status,
 *   ms)` ends it. A pending row's time counts up, so a slow request is seen to be slow.
 * - `highlightRow(row)` rings a row, for the moment a scene points at it.
 * - `at(selector)` is an anchor at an element of the page, where the pointer clicks.
 */
import { attr, css, mix, Panel, type Anchor, type Stage } from '../engine/stage'
import type { Track } from '../engine/time'

const ROW = 40

export interface Row {
  finish(status: number, ms: number): void
}

interface RowState {
  el: HTMLElement
  shown: Track
  hi: Track
  start: number
  end: number
  status: number
  ms: number
}

export class Browser extends Panel {
  private readonly screens = new Map<string, { el: HTMLElement; on: Track; items: HTMLElement[] }>()
  private readonly spin = this.tl.track(0)
  private readonly rows: RowState[] = []
  private readonly rowBox: HTMLElement
  private readonly count: HTMLElement
  private readonly spinner: HTMLElement
  private readonly favicon: HTMLElement
  private readonly anchors = new Map<string, { x: number; y: number }>()

  constructor(stage: Stage, el: HTMLElement) {
    super(stage, el)
    this.rowBox = el.querySelector('.wt-rows')!
    this.count = el.querySelector('.wt-dt-count')!
    this.spinner = el.querySelector('.wt-spinner')!
    this.favicon = el.querySelector('.wt-favicon')!
    el.querySelectorAll<HTMLElement>('[data-screen]').forEach((s, i) => {
      this.screens.set(s.dataset.screen!, {
        el: s,
        on: this.tl.track(i === 0 ? 1 : 0),
        items: [...s.querySelectorAll<HTMLElement>('[data-stagger]')],
      })
    })
  }

  /** Shows the screen `name` and hides the others; returns when the last staggered item has arrived. */
  screen(name: string): number {
    let longest = 0
    for (const [n, s] of this.screens) {
      if (n === name) longest = s.on.to(1, 'linear', { dur: 0.6 + s.items.length * 0.12 })
      else s.on.to(0, 'out', { dur: 0.25 })
    }
    return longest
  }

  loading(on: boolean): number {
    return this.spin.to(on ? 1 : 0, 'snappy')
  }

  request(req: { method: string; path: string }): Row {
    const el = document.createElement('div')
    el.className = 'wt-row'
    el.innerHTML =
      '<div class="wt-row-hi"></div><span class="wt-name"></span><span class="wt-method"></span>' +
      '<span class="wt-status wt-pending">pending</span><span class="wt-time"></span><span class="wt-wf"><i></i></span>'
    el.querySelector('.wt-name')!.textContent = req.path
    el.querySelector('.wt-method')!.textContent = req.method
    this.rowBox.append(el)
    const state: RowState = { el, shown: this.tl.track(0), hi: this.tl.track(0), start: this.tl.now, end: Infinity, status: 0, ms: 0 }
    state.shown.to(1, 'snappy')
    this.rows.push(state)
    const tl = this.tl
    return {
      finish(status: number, ms: number) {
        state.end = tl.now
        state.status = status
        state.ms = ms
      },
    }
  }

  /** Rings the row `i` (0 is the first request), or every row when `i` is undefined. */
  highlightRow(i?: number, on = true): number {
    let d = 0
    this.rows.forEach((r, k) => {
      if (i === undefined || i === k) d = r.hi.to(on ? 1 : 0, on ? 'pop' : 'out')
    })
    return d
  }

  /** An anchor at an element of the page, such as a button. */
  at(selector: string): Anchor {
    return () => this.anchors.get(selector) ?? this.anchor(selector)
  }

  measure(): void {
    this.anchors.clear()
    for (const sel of this.el.querySelectorAll<HTMLElement>('[data-anchor]')) {
      this.anchors.set(`[data-anchor="${sel.dataset.anchor}"]`, this.anchor(sel))
    }
  }

  render(t: number): void {
    this.renderPanel(t)
    const spin = this.spin.value(t)
    css(this.spinner, 'opacity', String(spin))
    css(this.spinner, 'transform', `rotate(${(t * 400) % 360}deg) scale(${mix(0.6, 1, spin)})`)
    css(this.favicon, 'opacity', String(1 - spin))
    css(this.favicon, 'transform', `scale(${mix(1, 0.6, spin)})`)

    for (const s of this.screens.values()) {
      const on = s.on.value(t)
      // The screen fades in quickly, and its staggered items arrive over the rest of the tween.
      css(s.el, 'opacity', String(Math.min(1, on * 4)))
      css(s.el, 'visibility', on <= 0.001 ? 'hidden' : 'visible')
      s.items.forEach((item, k) => {
        const n = s.items.length
        const p = Math.min(1, Math.max(0, (on * (n + 2) - k) / 2.2))
        const e = 1 - Math.pow(1 - p, 3)
        css(item, 'opacity', String(e))
        css(item, 'transform', `translateY(${(1 - e) * 18}px)`)
      })
    }

    let visible = 0
    this.rows.forEach((r, k) => {
      const shown = r.shown.value(t)
      if (shown > 0.001) visible++
      css(r.el, 'opacity', String(Math.min(1, shown)))
      css(r.el, 'transform', `translateY(${k * ROW + (1 - shown) * -12}px)`)
      css(r.el.querySelector<HTMLElement>('.wt-row-hi')!, 'opacity', String(Math.min(1, r.hi.value(t))))
      const done = t >= r.end
      const status = r.el.querySelector<HTMLElement>('.wt-status')!
      const text = done ? String(r.status) : 'pending'
      if (status.textContent !== text) status.textContent = text
      status.classList.toggle('wt-pending', !done)
      if (done && r.status < 400) attr(status, 'data-ok', '')
      else status.removeAttribute('data-ok')
      // The time counts up while the request runs, and stops at its real figure.
      const span = (Number.isFinite(r.end) ? r.end : t + 1) - r.start
      const ms = done ? r.ms : Math.round(Math.max(0, t - r.start) / Math.max(span, 0.001) * (r.ms || 0))
      const time = r.el.querySelector<HTMLElement>('.wt-time')!
      const tText = shown > 0 && (done || r.ms) ? `${ms} ms` : ''
      if (time.textContent !== tText) time.textContent = tText
      const wf = Math.max(0, Math.min(t, r.end) - r.start)
      css(r.el.querySelector<HTMLElement>('.wt-wf i')!, 'width', `${Math.min(100, 8 + wf * 60)}%`)
    })
    const label = `${visible} request${visible === 1 ? '' : 's'}`
    if (this.count.textContent !== label) this.count.textContent = label
  }
}
