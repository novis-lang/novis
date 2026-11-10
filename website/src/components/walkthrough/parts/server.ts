/**
 * Server: requests drawn as bars on a time axis, growing while the request runs.
 *
 * - `lane(label)` adds a request lane; on the lane, `start(label, tone)` starts the bar with its first
 *   segment, `phase(label, tone)` starts the next segment where the bar is, and `end()` stops it with
 *   a check mark.
 * - `lane(label, { child: true })` adds a thinner lane right under the last one: a task that runs as a
 *   child of that request, such as a part of a page, or a short request of its own. Its label is in
 *   its bar.
 * - `marker(label, tone)` draws a dashed line across the lanes at the cursor's time, with a label: a
 *   moment in the request's life, such as when the browser has the page.
 * - `clear()` fades out every lane and marker, so the next lanes start again at the top: the same
 *   request run a second time. `clear({ keep: 0.25 })` leaves them faded under the next round, which
 *   is drawn on their scale from its own start, so the two runs compare.
 * - `lane.tip` is an anchor at the growing end of the bar, which a card can ride.
 *
 * Every lane since the last `clear()` is on one time axis (`axis()`), so a request that starts later
 * starts further right.
 *
 * `data-pps` on the part's element is how many world units the bar grows per scene second.
 */
import { css, Panel, type Anchor, type Stage } from '../engine/stage'
import type { Track } from '../engine/time'

const LANE_TOP = 70
const LANE_H = 60
/** From one lane's bottom to the next lane's top: room for the next lane's label. */
const LANE_SPACE = 50
const CHILD_H = 36
/** From a lane's bottom to a child lane's top, and between two child lanes. */
const CHILD_SPACE = 12

interface Segment {
  el: HTMLElement
  label: HTMLElement
  start: number
  end: number
}

export class Lane {
  private readonly segs: Segment[] = []
  /** When the request starts and ends, in scene time; `Infinity` until the scene says. */
  startAt = Infinity
  endAt = Infinity
  private readonly endMark: HTMLElement
  private readonly endPop: Track
  private readonly labelOn: Track
  /** 1 until `Server.clear()` fades the lane out. */
  readonly kept: Track

  constructor(
    private readonly server: Server,
    readonly el: HTMLElement,
    /** The lanes since the last `clear()` share a number, and a time axis. */
    readonly round: number,
  ) {
    this.endMark = el.querySelector('.wt-end')!
    this.endPop = server.tl.track(0)
    this.labelOn = server.tl.track(0)
    this.kept = server.tl.track(1)
  }

  private segment(label: string, tone: string): void {
    const now = this.server.tl.now
    const last = this.segs.at(-1)
    if (last) last.end = now
    const el = document.createElement('div')
    el.className = 'wt-seg'
    el.style.setProperty('--wt-tone', tone)
    const span = document.createElement('span')
    span.textContent = label
    el.append(span)
    this.el.insertBefore(el, this.endMark)
    this.segs.push({ el, label: span, start: now, end: Infinity })
  }

  start(label: string, tone = 'var(--wt-run)'): void {
    this.startAt = this.server.tl.now
    this.server.axisChanged()
    this.labelOn.to(1, 'out')
    this.segment(label, tone)
  }

  phase(label: string, tone: string): void {
    this.segment(label, tone)
  }

  end(): number {
    this.endAt = this.server.tl.now
    this.server.axisChanged()
    const last = this.segs.at(-1)
    if (last) last.end = this.endAt
    return this.endPop.to(1, 'pop')
  }

  /**
   * The x of scene time `t` on this lane, in units from the lane's left edge, on the time axis every
   * lane of its round shares. Before the request starts it is where the request will start.
   */
  x(t: number): number {
    if (!Number.isFinite(this.startAt)) return 0
    const { t0, pps } = this.server.axis(this.round)
    return Math.max(0, Math.min(Math.max(t, this.startAt), this.endAt) - t0) * pps
  }

  /** The lane's width and left edge, measured on each layout change. */
  width = 1
  left = 0

  /** An anchor at the start of the lane, where a request comes in. */
  get origin(): Anchor {
    return () => this.server.laneOrigin(this)
  }

  /** An anchor at the bar's growing end. */
  get tip(): Anchor {
    return (t) => {
      const o = this.server.laneOrigin(this)
      return { x: o.x + this.x(t), y: o.y }
    }
  }

  render(t: number): void {
    const kept = this.kept.value(t)
    css(this.el, 'opacity', String(Math.max(0, kept)))
    css(this.el, 'visibility', kept <= 0.001 ? 'hidden' : 'visible')
    css(this.el.querySelector<HTMLElement>('.wt-lane-label')!, 'opacity', String(this.labelOn.value(t)))
    for (const s of this.segs) {
      const x0 = this.x(s.start)
      const x1 = this.x(Math.min(t, s.end))
      const on = t >= s.start
      css(s.el, 'visibility', on ? 'visible' : 'hidden')
      css(s.el, 'transform', `translateX(${x0}px)`)
      css(s.el, 'width', `${Math.max(0, x1 - x0)}px`)
      const running = on && t < s.end
      css(s.el, '--running', running ? '1' : '0')
      css(s.el, '--sheen', `${((t * 420) % 900) - 300}px`)
      // The label fades in once the segment is wide enough to hold it.
      css(s.label, 'opacity', String(Math.min(1, Math.max(0, (x1 - x0 - 40) / 120))))
    }
    const pop = this.endPop.value(t)
    css(this.endMark, 'opacity', String(Math.min(1, pop)))
    css(this.endMark, 'transform', `translateX(${this.x(this.endAt === Infinity ? t : this.endAt) + 12}px) scale(${pop})`)
  }
}

export class Server extends Panel {
  readonly pps: number
  private readonly lanes: Lane[] = []
  private readonly markers: { el: HTMLElement; lane: Lane; time: number; draw: Track; label: HTMLElement }[] = []
  private readonly box: HTMLElement
  private boxAt = { x: 0, y: 0 }
  private round = 0
  /** The bottom of the last lane of this round, from the top of the lanes' box; null before the first. */
  private bottom: number | null = null

  constructor(stage: Stage, el: HTMLElement) {
    super(stage, el)
    this.pps = Number(el.dataset.pps ?? 120)
    this.box = el.querySelector('.wt-lanes')!
  }

  lane(label: string, opts: { child?: boolean } = {}): Lane {
    const child = opts.child === true
    const el = document.createElement('div')
    el.className = child ? 'wt-lane wt-lane-child' : 'wt-lane'
    const top = this.bottom === null ? LANE_TOP : this.bottom + (child ? CHILD_SPACE : LANE_SPACE)
    this.bottom = top + (child ? CHILD_H : LANE_H)
    el.style.top = `${top}px`
    el.innerHTML = '<div class="wt-lane-label"></div><div class="wt-end"><svg width="20" height="20" viewBox="0 0 20 20"><path d="M4 10.5l4 4 8-9" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/></svg></div>'
    el.querySelector('.wt-lane-label')!.textContent = label
    this.box.append(el)
    const lane = new Lane(this, el, this.round)
    this.lanes.push(lane)
    return lane
  }

  /** A dashed line at the cursor's time on `lane`, with a label. */
  marker(lane: Lane, label: string, tone = 'var(--wt-moment)'): number {
    const el = document.createElement('div')
    el.className = 'wt-marker'
    el.style.setProperty('--wt-tone', tone)
    el.innerHTML = '<i></i><span></span>'
    const span = el.querySelector('span')!
    span.textContent = label
    this.box.append(el)
    const draw = this.tl.track(0)
    const d = draw.to(1, 'soft')
    this.markers.push({ el, lane, time: this.tl.now, draw, label: span })
    return d
  }

  /**
   * Fades out every lane and marker at the cursor; the next lane starts at the top again. With
   * `keep`, they fade only to that opacity and stay under the next round, which then uses their scale.
   */
  clear(opts: { keep?: number } = {}): number {
    const keep = opts.keep ?? 0
    let d = 0
    for (const l of this.lanes) if (l.round === this.round) d = l.kept.to(keep, 'out')
    if (keep > 0) this.scaleOf.set(this.round + 1, this.round)
    this.round++
    this.bottom = null
    return d
  }

  /** A round drawn over a kept one, and the kept round whose scale it uses. */
  private readonly scaleOf = new Map<number, number>()
  private axes = new Map<number, { t0: number; pps: number }>()

  /**
   * The time axis every lane of a round shares, so a later request starts further right and a marker
   * lines up on every lane. It starts where the round's first request starts. While any request never
   * ends, a second is `data-pps` units. When every request ends, the axis is scaled so that the span
   * from the first start to the last end fills 86% of the lane, since a scene's pauses for narration
   * make requests long. It is worked out once after each change.
   */
  axis(round = this.round): { t0: number; pps: number } {
    const known = this.axes.get(round)
    if (known) return known
    const started = this.lanes.filter((l) => l.round === round && Number.isFinite(l.startAt))
    const t0 = started.length ? Math.min(...started.map((l) => l.startAt)) : 0
    const ended = started.length > 0 && started.every((l) => Number.isFinite(l.endAt))
    const t1 = ended ? Math.max(...started.map((l) => l.endAt)) : 0
    const under = this.scaleOf.get(round)
    const pps = under !== undefined ? this.axis(under).pps : ended ? (started[0].width * 0.86) / Math.max(t1 - t0, 0.001) : this.pps
    const axis = { t0, pps }
    this.axes.set(round, axis)
    return axis
  }

  /** Called when a lane starts or ends, and on a layout change. */
  axisChanged(): void {
    this.axes.clear()
  }

  /** The world position of a lane's left edge, at the middle of its bar. */
  laneOrigin(lane: Lane): { x: number; y: number } {
    return { x: this.boxAt.x + lane.el.offsetLeft, y: this.boxAt.y + lane.el.offsetTop + lane.el.offsetHeight / 2 }
  }

  measure(): void {
    this.boxAt = this.anchor(this.box, 'top')
    this.boxAt.x -= this.box.offsetWidth / 2
    for (const l of this.lanes) {
      l.width = l.el.offsetWidth
      l.left = l.el.offsetLeft
    }
    this.axisChanged()
  }

  render(t: number): void {
    this.renderPanel(t)
    for (const l of this.lanes) l.render(t)
    for (const m of this.markers) {
      const d = m.draw.value(t)
      const kept = m.lane.kept.value(t)
      const x = m.lane.x(m.time)
      css(m.el, 'transform', `translateX(${m.lane.left + x}px)`)
      const left = x > m.lane.width * 0.6
      if (m.el.classList.contains('wt-marker-left') !== left) m.el.classList.toggle('wt-marker-left', left)
      css(m.el.querySelector('i')!, 'transform', `scaleY(${Math.max(0, d)})`)
      css(m.el, 'opacity', d > 0.001 ? String(Math.max(0, kept)) : '0')
      const lp = Math.max(0, Math.min(1, d * 1.4 - 0.4))
      css(m.label, 'opacity', String(lp))
      css(m.label, 'transform', `translateY(${(1 - lp) * 10}px) scale(${0.9 + 0.1 * lp})`)
    }
  }
}
