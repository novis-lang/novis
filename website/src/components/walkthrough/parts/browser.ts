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
 * - `clearRows()` empties the Network panel, as a browser does when it loads a page again.
 * - `fill(name)` swaps a part of the page from its placeholder to its content, with a flash, and
 *   `unfill(name)` puts the placeholder back with no motion. The part is `[data-fill="<name>"]` in the
 *   markup, with a `.wt-was` child (the placeholder) and a `.wt-now` child (the content).
 * - `at(selector, where?)` is an anchor at an element of the browser, where the pointer clicks or a
 *   caption points.
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
  /** Its place in the panel, counted from the last `clearRows()`. */
  index: number
  /** 1 until `clearRows()` takes it away. */
  kept: Track
}

interface Fill {
  was: HTMLElement
  now: HTMLElement
  /** 0 shows the placeholder, 1 the content. */
  on: Track
  flash: Track
}

export class Browser extends Panel {
  private readonly screens = new Map<string, { el: HTMLElement; on: Track; items: HTMLElement[] }>()
  private readonly spin = this.tl.track(0)
  private readonly rows: RowState[] = []
  /** Rows since the last `clearRows()`. */
  private shownRows = 0
  private readonly fills = new Map<string, Fill>()
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
    el.querySelectorAll<HTMLElement>('[data-fill]').forEach((f) => {
      this.fills.set(f.dataset.fill!, {
        was: f.querySelector('.wt-was')!,
        now: f.querySelector('.wt-now')!,
        on: this.tl.track(0),
        flash: this.tl.track(0),
      })
    })
  }

  private fillOf(name: string): Fill {
    const f = this.fills.get(name)
    if (!f) throw new Error(`Browser: no [data-fill="${name}"]`)
    return f
  }

  /** Shows the content of the part `name` in place of its placeholder, with a flash; returns the seconds. */
  fill(name: string): number {
    const f = this.fillOf(name)
    f.flash.set(1)
    f.flash.to(0, 'out', { dur: 1.4 })
    return f.on.to(1, 'snappy')
  }

  /** Shows the placeholder of the part `name` again, with no motion. */
  unfill(name: string): void {
    this.fillOf(name).on.set(0)
  }

  /** Shows the content of the part `name` with no motion, as a page that arrived whole. */
  filled(name: string): void {
    this.fillOf(name).on.set(1)
  }

  /** Empties the Network panel at the cursor, as a browser does when the page is loaded again. */
  clearRows(): number {
    let d = 0
    for (const r of this.rows) d = Math.max(d, r.kept.to(0, 'out', { dur: 0.3 }))
    this.shownRows = 0
    return d
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
    const state: RowState = {
      el,
      shown: this.tl.track(0),
      hi: this.tl.track(0),
      start: this.tl.now,
      end: Infinity,
      status: 0,
      ms: 0,
      index: this.shownRows++,
      kept: this.tl.track(1),
    }
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

  /**
   * Rings the row `i` (0 is the first request since the last `clearRows()`), or every row in the
   * panel when `i` is undefined.
   */
  highlightRow(i?: number, on = true): number {
    let d = 0
    const now = this.tl.now
    for (const r of this.rows) {
      if (r.kept.value(now) < 0.5) continue
      if (i === undefined || i === r.index) d = r.hi.to(on ? 1 : 0, on ? 'pop' : 'out')
    }
    return d
  }

  /** An anchor at an element of the browser, such as a button: its centre, or the middle of one side. */
  at(selector: string, where: 'center' | 'left' | 'right' | 'top' | 'bottom' = 'center'): Anchor {
    const key = `${selector} ${where}`
    return () => {
      let p = this.anchors.get(key)
      // Measured once per layout, on the first frame that needs it.
      if (!p) this.anchors.set(key, (p = this.anchor(selector, where)))
      return p
    }
  }

  measure(): void {
    this.anchors.clear()
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

    for (const f of this.fills.values()) {
      const on = f.on.value(t)
      css(f.was, 'opacity', String(Math.max(0, 1 - on * 1.6)))
      css(f.now, 'opacity', String(Math.min(1, Math.max(0, on))))
      css(f.now, 'transform', `translateY(${(1 - on) * 10}px) scale(${mix(0.96, 1, on)})`)
      css(f.was.parentElement!, '--flash', String(Math.max(0, f.flash.value(t))))
    }

    let visible = 0
    this.rows.forEach((r) => {
      const shown = r.shown.value(t) * r.kept.value(t)
      if (shown > 0.001 && r.kept.value(t) > 0.5) visible++
      css(r.el, 'opacity', String(Math.min(1, Math.max(0, shown))))
      css(r.el, 'visibility', shown <= 0.001 ? 'hidden' : 'visible')
      css(r.el, 'transform', `translateY(${r.index * ROW + (1 - r.shown.value(t)) * -12}px)`)
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
