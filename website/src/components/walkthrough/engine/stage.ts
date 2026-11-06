/**
 * The stage of a walkthrough: the fixed-size world the parts sit in, the camera that frames it, and
 * the player that turns real time into scene time.
 *
 * The world is a design space of fixed units, `WORLD[layout]`, in which every part has a rect given
 * by the scene. Wide containers use the 16:9 `wide` layout, narrow ones (phones) the `tall` one, and
 * the scene gives every part a rect in both. The world is scaled to fit its container, so a scene is
 * composed once, like a video frame, and every measurement inside it is in world units.
 *
 * A part is a class over one element of the stage's markup, `[data-part]`, with tracks the scene
 * tweens and a `render(t)` that writes the styles for scene time `t`. A part never knows the scene;
 * the scene calls the part's methods. `parts/` holds the reusable ones.
 */
import { MOTION, motionOf, Timeline, type Beat, type Motion, type MotionName } from './time'

export type Layout = 'wide' | 'tall'
export interface Rect {
  x: number
  y: number
  w: number
  h: number
}
export interface Point {
  x: number
  y: number
}
/** A place in the world at scene time `t`; a part hands these out so others can point at it. */
export type Anchor = (t: number) => Point
/** A value that may differ per layout: one for both, or `{ wide, tall }`. */
export type PerLayout<T> = T | { wide: T; tall: T }
/** A direction of travel in the world. */
export type Dir = 'left' | 'right' | 'up' | 'down'

export const WORLD: Record<Layout, { w: number; h: number }> = {
  wide: { w: 1600, h: 900 },
  tall: { w: 900, h: 1600 },
}

/** Below this container width, in CSS pixels, the stage uses the `tall` layout. */
const TALL_BELOW = 640

const clamp01 = (v: number) => Math.min(1, Math.max(0, v))
export const mix = (a: number, b: number, p: number) => a + (b - a) * p

// The last value written per element and property, so an unchanged frame writes nothing.
const written = new WeakMap<HTMLElement | SVGElement, Map<string, string>>()

/** Sets a style property, or a custom property, only when it changed. */
export function css(el: HTMLElement | SVGElement, prop: string, value: string): void {
  let m = written.get(el)
  if (!m) written.set(el, (m = new Map()))
  if (m.get(prop) === value) return
  m.set(prop, value)
  el.style.setProperty(prop, value)
}

/** Sets an attribute only when it changed. */
export function attr(el: Element, name: string, value: string): void {
  if (el.getAttribute(name) !== value) el.setAttribute(name, value)
}

function parseRect(s: string | undefined): Rect | undefined {
  if (!s) return undefined
  const [x, y, w, h] = s.split(',').map(Number)
  return { x, y, w, h }
}

/** The base of every part. */
export abstract class Part {
  /** 1 while the camera frames this part, 0 while it frames others; written by the camera. */
  focus = 1
  /** 1 while the caption on screen is about other parts, 0 otherwise; written by the stage. */
  blur = 0

  constructor(
    readonly stage: Stage,
    readonly el: HTMLElement,
  ) {}

  get tl(): Timeline {
    return this.stage.tl
  }

  get name(): string {
    return this.el.dataset.part!
  }

  /** The part's rect in the current layout. */
  rect(): Rect {
    const r = parseRect(this.el.dataset[this.stage.layout]) ?? parseRect(this.el.dataset.wide)
    if (!r) throw new Error(`walkthrough: part ${this.name} has no rect`)
    return r
  }

  /** A point on one side of the part's rect, `along` that side from its top or left end (0 to 1). */
  edge(side: 'left' | 'right' | 'top' | 'bottom', along = 0.5): Point {
    const r = this.rect()
    if (side === 'left') return { x: r.x, y: r.y + r.h * along }
    if (side === 'right') return { x: r.x + r.w, y: r.y + r.h * along }
    if (side === 'top') return { x: r.x + r.w * along, y: r.y }
    return { x: r.x + r.w * along, y: r.y + r.h }
  }

  /** Where a descendant of this part is in world units: its centre, or a corner. */
  anchor(target: string | HTMLElement, where: 'center' | 'left' | 'right' | 'top' | 'bottom' = 'center'): Point {
    const el = typeof target === 'string' ? this.el.querySelector<HTMLElement>(target) : target
    if (!el) throw new Error(`walkthrough: ${this.name} has no ${String(target)}`)
    let x = 0
    let y = 0
    for (let n: HTMLElement | null = el; n && n !== this.stage.world; n = n.offsetParent as HTMLElement | null) {
      x += n.offsetLeft
      y += n.offsetTop
    }
    const w = el.offsetWidth
    const h = el.offsetHeight
    if (where === 'left') return { x, y: y + h / 2 }
    if (where === 'right') return { x: x + w, y: y + h / 2 }
    if (where === 'top') return { x: x + w / 2, y }
    if (where === 'bottom') return { x: x + w / 2, y: y + h }
    return { x: x + w / 2, y: y + h / 2 }
  }

  /** Called after every layout change, before the next render. */
  measure(): void {}

  abstract render(t: number): void
}

/** How far a panel blurs while the caption is about other parts, in world units. */
const BLUR = 5

/**
 * A part that appears: fades and rises in with a little scale, dims when out of focus, and blurs while
 * the caption is about other parts.
 */
export abstract class Panel extends Part {
  protected readonly shown = this.tl.track(0)

  /** Brings the panel in at the cursor. */
  show(motion: MotionName | Motion = 'pop', delay = 0): number {
    return this.shown.to(1, motion, { delay })
  }

  hide(delay = 0): number {
    return this.shown.to(0, 'out', { delay })
  }

  protected renderPanel(t: number): void {
    const s = this.shown.value(t)
    const dim = mix(0.32, 1, this.focus)
    css(this.el, 'opacity', String(clamp01(s) * dim))
    css(this.el, 'transform', `translateY(${(1 - s) * 28}px) scale(${mix(0.94, 1, s)})`)
    css(this.el, 'visibility', s <= 0.001 ? 'hidden' : 'visible')
    // No filter at all while sharp: a filter, even a zero blur, makes the browser paint the panel apart.
    css(this.el, 'filter', this.blur > 0.01 ? `blur(${(this.blur * BLUR).toFixed(2)}px)` : 'none')
  }
}

/** What a shot frames: one part, several, the whole world, or a region of it. */
type Target = Part | Part[] | 'all' | Rect
const isRect = (t: Target): t is Rect => typeof t === 'object' && !Array.isArray(t) && !(t instanceof Part)
interface Shot {
  at: number
  motion: Motion
  target: Target
  pad?: number
  room?: PerLayout<Room>
  maxZoom: number
  only?: Layout
}
/** Extra world units a shot leaves on some of its sides, such as room for a caption beside a part. */
export type Room = Partial<Record<Side, number>>

/** The room the camera leaves around what it frames, per layout. A phone frames one part edge to edge. */
const PAD: Record<Layout, number> = { wide: 48, tall: 20 }
/**
 * Whether the view stays inside the world. On a phone a part at the top or bottom of the column would
 * otherwise leave a third of the stage empty, so the view shows the parts next to it instead.
 */
const INSIDE: Record<Layout, boolean> = { wide: false, tall: true }

/**
 * The camera: which part of the world fills the stage, and which parts are in focus.
 *
 * A phone's `tall` layout stacks the parts in one column, each about as wide as the world, so a shot
 * of two of them is no closer than the whole world. A scene therefore frames one part at a time there:
 * a shot with `only: 'wide'` is skipped in `tall`, and one with `only: 'tall'` exists only there.
 */
export class Camera {
  private shots: Shot[] = []
  /** The shots of each layout, built on its first frame after the last `focus`. */
  private inLayout: Partial<Record<Layout, Shot[]>> = {}

  constructor(private readonly stage: Stage) {}

  /**
   * Frames `target` at the cursor: one part, several, the whole world or a region. `room` leaves extra
   * world units on some sides, so a caption beside the part does not cover it.
   */
  focus(
    target: Target,
    opts: {
      motion?: MotionName | Motion
      pad?: number
      room?: PerLayout<Room>
      maxZoom?: number
      delay?: number
      only?: Layout
    } = {},
  ): number {
    const motion = motionOf(opts.motion ?? 'glide')
    const at = this.stage.tl.now + (opts.delay ?? 0)
    this.shots.push({ at, motion, target, pad: opts.pad, room: opts.room, maxZoom: opts.maxZoom ?? 1.9, only: opts.only })
    this.shots.sort((a, b) => a.at - b.at)
    this.inLayout = {}
    // The scene is timed once for both layouts, so a shot for one layout still returns its length.
    return (opts.delay ?? 0) + motion.dur
  }

  /** The parts a shot frames, or `null` when it frames the world or a region and every part is in focus. */
  private parts(target: Target): Part[] | null {
    if (target === 'all' || isRect(target)) return null
    return Array.isArray(target) ? target : [target]
  }

  private rectOf(shot: Shot | undefined): Rect {
    const world = WORLD[this.stage.layout]
    const all = { x: 0, y: 0, w: world.w, h: world.h }
    if (!shot || shot.target === 'all') return all
    const rs = isRect(shot.target) ? [shot.target] : this.parts(shot.target)!.map((p) => p.rect())
    const pad = shot.pad ?? PAD[this.stage.layout]
    const room: Room = (shot.room && this.stage.pick(shot.room)) ?? {}
    const x0 = Math.min(...rs.map((r) => r.x)) - pad - (room.left ?? 0)
    const y0 = Math.min(...rs.map((r) => r.y)) - pad - (room.above ?? 0)
    const x1 = Math.max(...rs.map((r) => r.x + r.w)) + pad + (room.right ?? 0)
    const y1 = Math.max(...rs.map((r) => r.y + r.h)) + pad + (room.below ?? 0)
    // Never closer than maxZoom: grow the rect around its centre to the world's aspect at that zoom.
    const minW = world.w / shot.maxZoom
    const minH = world.h / shot.maxZoom
    const w = Math.max(x1 - x0, minW)
    const h = Math.max(y1 - y0, minH)
    return { x: (x0 + x1) / 2 - w / 2, y: (y0 + y1) / 2 - h / 2, w, h }
  }

  /** Writes the world's transform and every part's focus for scene time `t`. */
  render(t: number): void {
    const layout = this.stage.layout
    const shots = (this.inLayout[layout] ??= this.shots.filter((s) => !s.only || s.only === layout))
    let i = -1
    for (let k = 0; k < shots.length; k++) if (shots[k].at <= t) i = k
    const cur = shots[i]
    const prev = shots[i - 1]
    const p = cur ? (cur.motion.dur === 0 ? 1 : cur.motion.ease(Math.min(1, (t - cur.at) / cur.motion.dur))) : 1
    const a = this.rectOf(prev)
    const b = this.rectOf(cur)
    const world = WORLD[this.stage.layout]
    const kA = Math.min(world.w / a.w, world.h / a.h)
    const kB = Math.min(world.w / b.w, world.h / b.h)
    // Zoom moves in log space and the centre in a straight line, so a pull-back feels even.
    const k = Math.exp(mix(Math.log(kA), Math.log(kB), p))
    let cx = mix(a.x + a.w / 2, b.x + b.w / 2, p)
    let cy = mix(a.y + a.h / 2, b.y + b.h / 2, p)
    if (INSIDE[layout]) {
      const vw = world.w / k
      const vh = world.h / k
      if (vw < world.w) cx = Math.min(Math.max(cx, vw / 2), world.w - vw / 2)
      if (vh < world.h) cy = Math.min(Math.max(cy, vh / 2), world.h - vh / 2)
    }
    this.stage.frame(k, cx, cy)

    const inA = new Set((prev && this.parts(prev.target)) ?? this.stage.parts)
    const inB = new Set((cur && this.parts(cur.target)) ?? this.stage.parts)
    const q = clamp01(p)
    for (const part of this.stage.parts) part.focus = mix(inA.has(part) ? 1 : 0, inB.has(part) ? 1 : 0, q)
  }
}

/** Which side of the point it is about a caption shows on. */
export type Side = 'above' | 'below' | 'left' | 'right'

/** Where a step's caption shows and what it is about; see `Stage.say`. */
export interface Say {
  /** The side of the point the caption sits on. */
  side: PerLayout<Side>
  /** The point the caption points at. By default, the middle of the part's side that faces the caption. */
  point?: PerLayout<Anchor>
  /** The parts that stay sharp while the caption shows; the other panels blur. By default, the part. */
  focus?: Part[]
  /**
   * World units between the point and the caption, on top of the usual small gap, so the caption
   * does not cover something next to the point. A line joins them.
   */
  lift?: number
  /** Whether the scene slows down to let the caption be read. On by default. */
  hold?: boolean
}

/** One step's caption on the stage: where it points from scene time `at` until its step ends. */
interface Callout {
  beat: number
  at: number
  part: Part | null
  side: PerLayout<Side>
  point?: PerLayout<Anchor>
  focus: Set<Part> | null
  lift: number
}

/** Real seconds to read a caption: a base and a time per word, a slow reader's pace. */
const READ_BASE = 0.7
const READ_PER_WORD = 0.24
/** Scene seconds between one word's arrival and the next's. */
const WORD_GAP = 0.03
/** Scene seconds a caption takes to fade out after its step, and the blur to come and go. */
const FADE = 0.3
/** The space a caption keeps from the stage's edge, in CSS pixels. */
const MARGIN = 10

const words = (s: string) => s.split(' ').filter(Boolean)

/** The stage: world, parts, camera, captions and the player. */
export class Stage {
  readonly tl = new Timeline()
  readonly camera = new Camera(this)
  readonly parts: Part[] = []
  readonly world: HTMLElement
  layout: Layout = 'wide'

  private readonly viewport: HTMLElement
  private readonly caption: HTMLElement
  private fit = 1
  private vw = 0
  private vh = 0
  /** The camera's last frame: zoom and centre. */
  private cam = { k: 1, cx: 0, cy: 0 }
  private says: Callout[] = []
  /** The caption's size and font size in CSS pixels, measured when its text changes; -1 to measure. */
  private capW = -1
  private capH = 0
  private capEm = 16

  // Player state.
  t = 0
  private playing = false
  private stopAt = Infinity
  private last = 0
  private raf = 0
  private visible = true
  /** The index in `says` of the caption whose words are in the caption element. */
  private sayShown = -1
  private readonly reduced = matchMedia('(prefers-reduced-motion: reduce)').matches
  private readonly listeners: Array<() => void> = []

  constructor(readonly root: HTMLElement) {
    this.world = root.querySelector('.wt-world')!
    this.viewport = root.querySelector('.wt-viewport')!
    this.caption = root.querySelector('.wt-caption')!
  }

  /** Builds the part class `P` over the element `[data-part=name]`. */
  part<P extends Part>(name: string, P: new (stage: Stage, el: HTMLElement) => P): P {
    const el = this.root.querySelector<HTMLElement>(`[data-part="${name}"]`)
    if (!el) throw new Error(`walkthrough: no part ${name}`)
    const p = new P(this, el)
    this.parts.push(p)
    return p
  }

  /**
   * Shows the current step's caption at the cursor, as a bubble beside `part` that points at it. The
   * other panels blur while it shows, and the scene slows almost to a stop for the time it takes to
   * read, so the viewer reads where the thing happens and then watches it. A step calls this once;
   * a step that does not shows its caption at the bottom of the stage from its start.
   */
  say(part: Part, opts: Say): void {
    const beat = this.tl.beats.length - 1
    if (beat < 0) throw new Error('walkthrough: say() belongs inside a step')
    if (this.says.some((s) => s.beat === beat)) throw new Error(`walkthrough: step ${beat + 1} already has its caption`)
    const at = this.tl.now
    const focus = new Set(opts.focus ?? [part])
    this.says.push({ beat, at, part, side: opts.side, point: opts.point, focus, lift: opts.lift ?? 0 })
    if (opts.hold === false) return
    const n = words(this.tl.beats[beat].caption).length
    // The hold starts once the words are in; the viewer has been reading since the first one.
    const enter = 0.1 + n * WORD_GAP + MOTION.snappy.dur * 0.5
    this.tl.wait(enter)
    this.tl.hold(Math.max(0.8, READ_BASE + READ_PER_WORD * n - enter))
  }

  /** Called once the scene is authored: lays out, renders the first frame and wires the controls. */
  start(): void {
    for (const [i, b] of this.tl.beats.entries())
      if (!this.says.some((s) => s.beat === i))
        this.says.push({ beat: i, at: b.start, part: null, side: 'below', focus: null, lift: 0 })
    this.says.sort((a, b) => a.at - b.at)
    new ResizeObserver(() => this.relayout()).observe(this.root)
    document.addEventListener('fullscreenchange', () => this.relayout())
    new IntersectionObserver(([e]) => {
      this.visible = e.isIntersecting
      if (this.visible && this.playing) this.loop()
    }).observe(this.root)
    this.relayout()
    // `?step=N` opens on the end of step N; otherwise the end of the first step is the still frame
    // behind the start button.
    const step = Number(new URLSearchParams(location.search).get('step'))
    if (step > 0 && this.tl.beats[step - 1]) {
      this.seek(this.stepEnd(step - 1))
      this.root.dataset.started = ''
    } else this.seek(this.stepEnd(0))
    this.root.dataset.ready = ''
    // The stage is reachable from the console as `<root>.wt`, to seek and inspect frames.
    Object.assign(this.root, { wt: this })
  }

  /** Called on every frame and seek; the controls listen here. */
  onUpdate(fn: () => void): void {
    this.listeners.push(fn)
  }

  get duration(): number {
    return this.tl.duration
  }

  get isPlaying(): boolean {
    return this.playing
  }

  /** The index of the beat scene time `t` is in, the last one past the end. */
  beatAt(t = this.t): number {
    const beats = this.tl.beats
    for (let i = 0; i < beats.length; i++) if (t < beats[i].end) return i
    return beats.length - 1
  }

  /**
   * Where a stop at the end of step `i` lands: a tenth of a millisecond before the next step starts.
   * At the boundary itself the next step's caption would start empty and a change it makes at its
   * first instant, such as a row's status, would already show. `next()`, `prev()`, `?step=N` and the
   * still frames all stop here.
   */
  stepEnd(i: number): number {
    const b = this.tl.beats[i]
    return b ? Math.max(b.start, b.end - 1e-4) : 0
  }

  /** The value of `v` for the current layout. */
  pick<T>(v: PerLayout<T>): T {
    return v !== null && typeof v === 'object' && 'wide' in v && 'tall' in v ? v[this.layout] : (v as T)
  }

  private relayout(): void {
    const full = document.fullscreenElement === this.root
    const box = this.root.getBoundingClientRect()
    this.layout = !full && box.width < TALL_BELOW ? 'tall' : 'wide'
    this.root.dataset.layout = this.layout
    const world = WORLD[this.layout]
    const availW = full ? innerWidth : box.width
    const availH = full ? innerHeight : (availW * world.h) / world.w
    this.fit = Math.min(availW / world.w, availH / world.h)
    this.vw = world.w * this.fit
    this.vh = world.h * this.fit
    css(this.viewport, 'width', `${this.vw}px`)
    css(this.viewport, 'height', `${this.vh}px`)
    css(this.world, 'width', `${world.w}px`)
    css(this.world, 'height', `${world.h}px`)
    for (const p of this.parts) {
      // A free part, such as a card or the pointer, places itself in render().
      if (!p.el.dataset.wide) continue
      const r = p.rect()
      css(p.el, 'left', `${r.x}px`)
      css(p.el, 'top', `${r.y}px`)
      css(p.el, 'width', `${r.w}px`)
      css(p.el, 'height', `${r.h}px`)
    }
    for (const p of this.parts) p.measure()
    // The caption's font size follows the stage's width.
    this.capW = -1
    this.render()
  }

  /** Called by the camera: world zoom `k` (1 = the whole world fills the stage) centred on (cx, cy). */
  frame(k: number, cx: number, cy: number): void {
    this.cam = { k, cx, cy }
    const s = this.fit * k
    css(this.world, 'transform', `translate(${this.vw / 2 - cx * s}px, ${this.vh / 2 - cy * s}px) scale(${s})`)
  }

  render(): void {
    const t = this.t
    this.camera.render(t)
    this.renderBlur(t)
    for (const p of this.parts) p.render(t)
    this.renderCaption(t)
    for (const fn of this.listeners) fn()
  }

  /** How much a caption is on the stage at `t`: 0 before it, 1 while it shows, 0 again after its step. */
  private presence(c: Callout, t: number): number {
    const end = this.tl.beats[c.beat].end
    return MOTION.out.ease(clamp01((t - c.at) / FADE)) * (1 - MOTION.out.ease(clamp01((t - end) / FADE)))
  }

  /** Blurs every panel a caption on the stage is not about. */
  private renderBlur(t: number): void {
    for (const p of this.parts) p.blur = 0
    for (const c of this.says) {
      if (!c.focus) continue
      const r = this.presence(c, t)
      if (r > 0) for (const p of this.parts) if (!c.focus.has(p)) p.blur = Math.max(p.blur, r)
    }
  }

  /** Where world point `p` is on the stage, in CSS pixels from its top left corner. */
  private toScreen(p: Point): Point {
    const s = this.fit * this.cam.k
    return { x: this.vw / 2 + (p.x - this.cam.cx) * s, y: this.vh / 2 + (p.y - this.cam.cy) * s }
  }

  private renderCaption(t: number): void {
    let i = -1
    for (let k = 0; k < this.says.length; k++) if (this.says[k].at <= t) i = k
    const c: Callout | undefined = this.says[i]
    const beat: Beat | undefined = c && this.tl.beats[c.beat]
    const side = c?.part ? this.pick(c.side) : 'none'
    if (i !== this.sayShown) {
      this.sayShown = i
      this.caption.replaceChildren(
        ...words(beat?.caption ?? '').map((w) => {
          const s = document.createElement('span')
          s.textContent = w + ' '
          return s
        }),
      )
      this.caption.classList.toggle('is-moment', !!beat?.moment)
      this.capW = -1
    }
    this.caption.dataset.side = side
    if (!c || !beat) {
      css(this.caption, 'opacity', '0')
      return
    }
    if (this.capW < 0) {
      this.capW = this.caption.offsetWidth
      this.capH = this.caption.offsetHeight
      this.capEm = parseFloat(getComputedStyle(this.caption).fontSize) || 16
    }
    const w = this.capW
    const h = this.capH
    const lead = c.lift * this.fit * this.cam.k
    const gap = this.capEm * 0.8 + lead

    // The bubble sits on its side of the point and stays on the stage; its tail points at the point.
    let x = (this.vw - w) / 2
    let y = this.vh - h - MARGIN * 2
    let tail = 0
    if (c.part) {
      const part = c.part
      const facing = { above: 'top', below: 'bottom', left: 'left', right: 'right' } as const
      const pt = this.toScreen(c.point ? this.pick(c.point)(t) : part.edge(facing[side as Side]))
      if (side === 'above' || side === 'below') {
        x = pt.x - w / 2
        y = side === 'above' ? pt.y - gap - h : pt.y + gap
      } else {
        x = side === 'left' ? pt.x - gap - w : pt.x + gap
        y = pt.y - h / 2
      }
      x = Math.min(Math.max(x, MARGIN), this.vw - w - MARGIN)
      y = Math.min(Math.max(y, MARGIN), this.vh - h - MARGIN)
      const r = this.capEm
      tail = side === 'above' || side === 'below' ? Math.min(Math.max(pt.x - x, r), w - r) : Math.min(Math.max(pt.y - y, r), h - r)
    }

    // The bubble grows out of its tail, then the words arrive one after another.
    const since = this.reduced ? 99 : t - c.at
    const grow = MOTION.pop.ease(clamp01(since / MOTION.pop.dur))
    const origin =
      side === 'above' ? `${tail}px ${h}px` : side === 'below' ? `${tail}px 0` : side === 'left' ? `${w}px ${tail}px` : side === 'right' ? `0 ${tail}px` : '50% 100%'
    css(this.caption, '--tail', `${tail.toFixed(1)}px`)
    css(this.caption, '--lead', `${lead > 0 ? (gap - this.capEm * 0.15).toFixed(1) : 0}px`)
    css(this.caption, 'transform-origin', origin)
    css(this.caption, 'transform', `translate(${x.toFixed(1)}px, ${y.toFixed(1)}px) scale(${mix(0.86, 1, grow).toFixed(4)})`)
    css(this.caption, 'opacity', this.reduced ? '1' : this.presence(c, t).toFixed(3))
    const spans = this.caption.children
    for (let k = 0; k < spans.length; k++) {
      const p = MOTION.snappy.ease(clamp01((since - 0.08 - k * WORD_GAP) / MOTION.snappy.dur))
      const el = spans[k] as HTMLElement
      css(el, 'opacity', String(clamp01(p)))
      css(el, 'transform', `translateY(${((1 - p) * 0.4).toFixed(3)}em)`)
    }
  }

  seek(t: number): void {
    this.t = Math.min(Math.max(0, t), this.duration)
    this.render()
  }

  play(until = Infinity): void {
    if (this.t >= this.duration - 1e-3) this.t = 0
    if (this.reduced) {
      // Without motion, playing shows the end of the next beat, a still frame.
      const i = this.t >= this.tl.beats[this.beatAt()].end - 1e-3 ? this.beatAt() + 1 : this.beatAt()
      this.seek(this.stepEnd(Math.min(i, this.tl.beats.length - 1)))
      return
    }
    this.stopAt = until
    this.playing = true
    this.root.dataset.playing = ''
    this.last = performance.now()
    this.loop()
  }

  pause(): void {
    this.playing = false
    delete this.root.dataset.playing
    cancelAnimationFrame(this.raf)
    this.render()
  }

  toggle(): void {
    if (this.playing) this.pause()
    else this.play()
  }

  /** Plays to the end of the next beat and stops there: one press, one step. */
  next(): void {
    const beats = this.tl.beats
    let i = this.beatAt()
    if (this.t >= beats[i].end - 1e-3) i++
    if (i >= beats.length) return
    if (this.t < beats[i].start) this.t = beats[i].start
    this.play(this.stepEnd(i))
  }

  /** Goes back to the end of the beat before the current position, paused. */
  prev(): void {
    const beats = this.tl.beats
    let target = 0
    for (const [k, b] of beats.entries()) if (b.end < this.t - 0.05) target = this.stepEnd(k)
    this.pause()
    this.seek(target)
  }

  private loop(): void {
    cancelAnimationFrame(this.raf)
    if (!this.playing || !this.visible) return
    this.raf = requestAnimationFrame((now) => {
      // A long gap (a background tab) counts as one frame, so the scene never jumps.
      const dt = Math.min(now - this.last, 50) / 1000
      this.last = now
      this.t = Math.min(this.t + dt * this.tl.rate(this.t), this.duration, this.stopAt)
      const done = this.t >= this.duration || this.t >= this.stopAt
      if (done) this.playing = false
      if (done) delete this.root.dataset.playing
      this.render()
      this.loop()
    })
  }
}
