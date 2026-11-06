/**
 * The parts that move between other parts: they have no rect of their own and follow anchors.
 *
 * - `Card`: a labelled chip, such as a function waiting to run. `appear(at)` pops it in at an
 *   anchor, `moveTo(at)` flies it to another along a slight arc, `follow(at)` keeps it on a moving
 *   anchor, `glow(strength)` makes it shine, `label(text)` swaps its text, `leave()` fades it out.
 * - `Pointer`: the visitor's mouse. `moveTo(at)` glides it there along an arc, `click()` presses.
 * - `Wire`: a line between two anchors that `pulse(dir)` sends a dot along, as a request or a
 *   response travelling over the network.
 */
import { css, mix, Part, type Anchor, type Dir, type PerLayout, type Stage } from '../engine/stage'
import { motionOf, type Motion, type MotionName, type Track } from '../engine/time'

interface Leg {
  at: number
  motion: Motion
  from: Anchor
  to: Anchor
  arc: number
  /** Stays on `to` after arriving, however `to` moves. */
  follow: boolean
}

/** A path through anchors over time, shared by the card and the pointer. */
class Path {
  readonly legs: Leg[] = []

  constructor(private readonly stage: Stage) {}

  add(to: Anchor, motion: Motion, arc: number, follow = false, delay = 0): number {
    const prev = this.legs.at(-1)
    const at = this.stage.tl.now + delay
    const from: Anchor = prev ? this.frozen(prev, at) : to
    this.legs.push({ at, motion, from, to, arc, follow })
    return delay + motion.dur
  }

  /** Where a leg leaves the path at `when`: a following leg keeps following, others stay where they ended. */
  private frozen(leg: Leg, when: number): Anchor {
    return leg.follow ? (t) => this.pointOf(leg, t) : () => this.pointOf(leg, when)
  }

  private pointOf(leg: Leg, t: number): { x: number; y: number } {
    const raw = leg.motion.dur === 0 ? 1 : Math.min(1, Math.max(0, (t - leg.at) / leg.motion.dur))
    const p = leg.motion.ease(raw)
    const a = leg.from(t)
    const b = leg.to(t)
    // A gentle arc: the path bows sideways in the middle, as a hand moves.
    const dx = b.x - a.x
    const dy = b.y - a.y
    const len = Math.hypot(dx, dy) || 1
    const bow = Math.sin(Math.PI * Math.min(1, Math.max(0, p))) * leg.arc * len
    return { x: mix(a.x, b.x, p) - (dy / len) * bow, y: mix(a.y, b.y, p) + (dx / len) * bow }
  }

  point(t: number): { x: number; y: number } | undefined {
    let leg: Leg | undefined
    for (const l of this.legs) if (l.at <= t) leg = l
    return leg ? this.pointOf(leg, t) : undefined
  }
}

export class Card extends Part {
  private readonly path: Path
  private readonly on: Track
  private readonly glowT: Track
  private readonly body: HTMLElement
  private readonly text: HTMLElement
  private readonly labels: { at: number; text: string }[] = []

  constructor(stage: Stage, el: HTMLElement) {
    super(stage, el)
    this.path = new Path(stage)
    this.on = stage.tl.track(0)
    this.glowT = stage.tl.track(0)
    this.body = el.querySelector('.wt-card-body')!
    this.text = el.querySelector('.wt-card-text')!
    this.labels.push({ at: -1, text: this.text.textContent ?? '' })
  }

  appear(at: Anchor, delay = 0): number {
    this.path.add(at, motionOf('step'), 0, false, delay)
    return this.on.to(1, 'pop', { delay })
  }

  moveTo(at: Anchor, motion: MotionName | Motion = 'soft', arc = 0.12): number {
    return this.path.add(at, motionOf(motion), arc)
  }

  /** Flies to a moving anchor and stays on it. */
  follow(at: Anchor, motion: MotionName | Motion = 'soft', arc = 0.08): number {
    return this.path.add(at, motionOf(motion), arc, true)
  }

  glow(strength = 1): number {
    return this.glowT.to(strength, 'soft')
  }

  label(text: string): void {
    this.labels.push({ at: this.tl.now, text })
  }

  leave(): number {
    return this.on.to(0, 'out')
  }

  render(t: number): void {
    const on = this.on.value(t)
    const pt = this.path.point(t)
    css(this.el, 'visibility', pt && on > 0.001 ? 'visible' : 'hidden')
    if (!pt) return
    let text = this.labels[0].text
    for (const l of this.labels) if (l.at <= t) text = l.text
    if (this.text.textContent !== text) this.text.textContent = text
    css(this.el, 'transform', `translate(${pt.x}px, ${pt.y}px)`)
    css(this.el, 'opacity', String(Math.min(1, Math.max(0, on))))
    css(this.body, 'transform', `translate(-50%, -50%) scale(${0.6 + 0.4 * on})`)
    css(this.body, '--glow', String(this.glowT.value(t)))
  }
}

export class Pointer extends Part {
  private readonly path: Path
  private readonly on: Track
  private readonly press: Track
  private readonly ring: HTMLElement
  private readonly arrow: SVGElement
  private clicks: number[] = []

  constructor(stage: Stage, el: HTMLElement) {
    super(stage, el)
    this.path = new Path(stage)
    this.on = stage.tl.track(0)
    this.press = stage.tl.track(0)
    this.ring = el.querySelector('.wt-click')!
    this.arrow = el.querySelector('svg')!
  }

  /** Glides to `at`; the first call brings the pointer in from a little below. */
  moveTo(at: Anchor, motion: MotionName | Motion = 'soft'): number {
    if (this.path.legs.length === 0) {
      this.path.add((t) => ({ x: at(t).x + 120, y: at(t).y + 160 }), motionOf('step'), 0)
      this.on.to(1, 'out')
    }
    return this.path.add(at, motionOf(motion), 0.1)
  }

  click(): number {
    this.press.to(1, 'snappy')
    this.press.to(0, 'soft', { delay: 0.16 })
    this.clicks.push(this.tl.now + 0.08)
    return 0.45
  }

  leave(): number {
    return this.on.to(0, 'out')
  }

  render(t: number): void {
    const pt = this.path.point(t)
    const on = this.on.value(t)
    css(this.el, 'visibility', pt && on > 0.001 ? 'visible' : 'hidden')
    if (!pt) return
    css(this.el, 'transform', `translate(${pt.x}px, ${pt.y}px)`)
    css(this.el, 'opacity', String(on))
    css(this.arrow, 'transform', `scale(${1 - 0.18 * this.press.value(t)})`)
    // The ring of the last click spreads and fades.
    const c = this.clicks.filter((at) => at <= t).at(-1)
    const r = c === undefined ? 1 : Math.min(1, (t - c) / 0.6)
    css(this.ring, 'opacity', String(c === undefined ? 0 : (1 - r) * 0.9))
    css(this.ring, 'transform', `scale(${0.3 + r * 0.9})`)
  }
}

const DIR: Record<Dir, { x: number; y: number }> = {
  left: { x: -1, y: 0 },
  right: { x: 1, y: 0 },
  up: { x: 0, y: -1 },
  down: { x: 0, y: 1 },
}
/** How far, in world units, the handles of a loop reach out from its two ends. */
const LOOP_REACH = 120

export class Wire extends Part {
  private readonly path: SVGPathElement
  private readonly pulses: { el: SVGCircleElement; at: number; dur: number; back: boolean }[] = []
  private readonly drawT: Track
  private from: PerLayout<Anchor> = () => ({ x: 0, y: 0 })
  private to: PerLayout<Anchor> = () => ({ x: 0, y: 0 })
  private leave: PerLayout<Dir> = 'right'
  private arrive: PerLayout<Dir> = 'right'
  private ends: Part[] = []

  constructor(stage: Stage, el: HTMLElement) {
    super(stage, el)
    this.path = el.querySelector('path')!
    this.drawT = stage.tl.track(0)
  }

  /**
   * Draws the wire between two anchors at the cursor. `leave` is the direction it sets off in and
   * `arrive` the direction it travels in at the end, so `right` and `right` is an S-curve and `left`
   * and `right` a loop out to the left. The anchors and both directions may differ per layout. `ends`
   * are the parts it joins, and it dims while the camera frames neither.
   */
  connect(
    from: PerLayout<Anchor>,
    to: PerLayout<Anchor>,
    opts: { leave?: PerLayout<Dir>; arrive?: PerLayout<Dir>; ends?: Part[] } = {},
  ): number {
    this.from = from
    this.to = to
    this.leave = opts.leave ?? 'right'
    this.arrive = opts.arrive ?? this.leave
    this.ends = opts.ends ?? []
    return this.drawT.to(1, 'soft')
  }

  private focusOf(_t: number): number {
    return this.ends.length ? Math.max(...this.ends.map((p) => p.focus)) : 1
  }

  disconnect(): number {
    return this.drawT.to(0, 'out')
  }

  /** Sends a dot along the wire: forward (from → to) or back. */
  pulse(dir: 'forward' | 'back', dur = 0.7, tone = 'var(--wt-run)'): number {
    const el = document.createElementNS('http://www.w3.org/2000/svg', 'circle')
    el.setAttribute('r', '9')
    el.setAttribute('class', 'wt-pulse')
    el.style.setProperty('--wt-tone', tone)
    this.el.querySelector('svg')!.append(el)
    this.pulses.push({ el, at: this.tl.now, dur, back: dir === 'back' })
    return dur
  }

  render(t: number): void {
    const stage = this.stage
    const a = stage.pick(this.from)(t)
    const b = stage.pick(this.to)(t)
    const u = DIR[stage.pick(this.leave)]
    const v = DIR[stage.pick(this.arrive)]
    // A cubic curve whose ends point along `u` and `v`. Facing the same way it is an S-curve with its
    // handles half way across; facing apart it is a loop that bulges out by a fixed reach.
    const along = Math.abs((b.x - a.x) * u.x + (b.y - a.y) * u.y)
    const reach = u.x * v.x + u.y * v.y > 0 ? along / 2 : LOOP_REACH
    const d = `M${a.x},${a.y} C${a.x + u.x * reach},${a.y + u.y * reach} ${b.x - v.x * reach},${b.y - v.y * reach} ${b.x},${b.y}`
    if (this.path.getAttribute('d') !== d) this.path.setAttribute('d', d)
    const draw = this.drawT.value(t)
    css(this.path, 'stroke-dasharray', '1 1')
    css(this.path, 'stroke-dashoffset', String(1 - Math.min(1, Math.max(0, draw))))
    css(this.path, 'opacity', String(Math.min(1, Math.max(0, draw)) * Math.max(0.35, this.focusOf(t))))
    const total = this.path.getTotalLength()
    for (const p of this.pulses) {
      const raw = (t - p.at) / p.dur
      const on = raw >= 0 && raw <= 1
      css(p.el, 'opacity', on ? String(Math.min(1, Math.sin(Math.PI * raw) * 3)) : '0')
      if (!on) continue
      const e = raw < 0.5 ? 2 * raw * raw : 1 - Math.pow(-2 * raw + 2, 2) / 2
      const pt = this.path.getPointAtLength(total * (p.back ? 1 - e : e))
      p.el.setAttribute('cx', String(pt.x))
      p.el.setAttribute('cy', String(pt.y))
    }
  }
}
