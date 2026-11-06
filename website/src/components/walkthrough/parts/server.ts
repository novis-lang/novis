/**
 * Server: requests drawn as bars on a time axis, growing while the request runs.
 *
 * - `lane(label)` adds a request lane; on the lane, `start(label, tone)` starts the bar with its first
 *   segment, `phase(label, tone)` starts the next segment where the bar is, and `end()` stops it with
 *   a check mark.
 * - `marker(label, tone)` draws a dashed line across the lanes at the cursor's time, with a label: a
 *   moment in the request's life, such as when the browser has the page.
 * - `lane.tip` is an anchor at the growing end of the bar, which a card can ride.
 *
 * `data-pps` on the part's element is how many world units the bar grows per scene second.
 */
import { css, Panel, type Anchor, type Stage } from '../engine/stage'
import type { Track } from '../engine/time'

const LANE_TOP = 70
const LANE_GAP = 110

interface Segment {
  el: HTMLElement
  label: HTMLElement
  start: number
  end: number
}

export class Lane {
  private readonly segs: Segment[] = []
  private startAt = Infinity
  private endAt = Infinity
  private readonly endMark: HTMLElement
  private readonly endPop: Track
  private readonly labelOn: Track

  constructor(
    private readonly server: Server,
    readonly el: HTMLElement,
  ) {
    this.endMark = el.querySelector('.wt-end')!
    this.endPop = server.tl.track(0)
    this.labelOn = server.tl.track(0)
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
    this.labelOn.to(1, 'out')
    this.segment(label, tone)
  }

  phase(label: string, tone: string): void {
    this.segment(label, tone)
  }

  end(): number {
    this.endAt = this.server.tl.now
    const last = this.segs.at(-1)
    if (last) last.end = this.endAt
    return this.endPop.to(1, 'pop')
  }

  /** The x of scene time `t` on this lane, in units from the lane's left edge. */
  x(t: number): number {
    return Math.max(0, Math.min(t, this.endAt) - this.startAt) * this.scale()
  }

  /**
   * Units per scene second. A lane that ends is scaled so the whole request fills its width, since a
   * scene's pauses for narration make requests long; a lane that never ends grows at `data-pps`.
   */
  private scale(): number {
    if (!Number.isFinite(this.endAt)) return this.server.pps
    return (this.width * 0.86) / Math.max(this.endAt - this.startAt, 0.001)
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

  constructor(stage: Stage, el: HTMLElement) {
    super(stage, el)
    this.pps = Number(el.dataset.pps ?? 120)
    this.box = el.querySelector('.wt-lanes')!
  }

  lane(label: string): Lane {
    const el = document.createElement('div')
    el.className = 'wt-lane'
    el.style.top = `${LANE_TOP + this.lanes.length * LANE_GAP}px`
    el.innerHTML = '<div class="wt-lane-label"></div><div class="wt-end"><svg width="20" height="20" viewBox="0 0 20 20"><path d="M4 10.5l4 4 8-9" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/></svg></div>'
    el.querySelector('.wt-lane-label')!.textContent = label
    this.box.append(el)
    const lane = new Lane(this, el)
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

  /** The world position of a lane's left edge, at the middle of its bar. */
  laneOrigin(lane: Lane): { x: number; y: number } {
    return { x: this.boxAt.x + lane.el.offsetLeft, y: this.boxAt.y + lane.el.offsetTop + 30 }
  }

  measure(): void {
    this.boxAt = this.anchor(this.box, 'top')
    this.boxAt.x -= this.box.offsetWidth / 2
    for (const l of this.lanes) {
      l.width = l.el.offsetWidth
      l.left = l.el.offsetLeft
    }
  }

  render(t: number): void {
    this.renderPanel(t)
    for (const l of this.lanes) l.render(t)
    for (const m of this.markers) {
      const d = m.draw.value(t)
      css(m.el, 'transform', `translateX(${m.lane.left + m.lane.x(m.time)}px)`)
      css(m.el.querySelector('i')!, 'transform', `scaleY(${Math.max(0, d)})`)
      css(m.el, 'opacity', d > 0.001 ? '1' : '0')
      const lp = Math.max(0, Math.min(1, d * 1.4 - 0.4))
      css(m.label, 'opacity', String(lp))
      css(m.label, 'transform', `translateY(${(1 - lp) * 10}px) scale(${0.9 + 0.1 * lp})`)
    }
  }
}
