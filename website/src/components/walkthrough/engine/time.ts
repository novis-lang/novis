/**
 * The time model of a walkthrough: everything on the stage is a pure function of the scene time `t`,
 * in seconds.
 *
 * A scene is authored once, when the page loads: each part adds tweens to its own tracks at the
 * timeline's cursor, `tl.now`, and the author moves the cursor with `tl.wait`. Playing, scrubbing,
 * stepping backwards and jumping to a step all evaluate the same tracks at another `t`, so every
 * frame is the same frame however it was reached.
 *
 * The motion maths is Motion's (motion.dev): `spring` is evaluated at a time, never run as an
 * animation, so a spring can be scrubbed like any other tween.
 */
import { spring as springGenerator, cubicBezier } from 'motion'

/** Maps progress 0..1 to eased progress; a spring may overshoot 1 before it settles. */
export type Ease = (p: number) => number

/** A motion: an ease and the seconds it takes. */
export interface Motion {
  ease: Ease
  dur: number
}

/** A spring by its look: `visual` is roughly when it arrives, `bounce` 0 (none) to 1. */
export function spring(visual: number, bounce = 0): Motion {
  const gen = springGenerator({ keyframes: [0, 1], visualDuration: visual, bounce })
  // The spring's settle time, sampled once: the tween is over when the generator is done.
  let ms = 0
  while (ms < 10_000 && !gen.next(ms).done) ms += 8
  const dur = ms / 1000
  return { dur, ease: (p) => (p >= 1 ? 1 : gen.next(p * ms).value) }
}

/** A cubic-bezier tween of `dur` seconds. */
export function bezier(dur: number, x1: number, y1: number, x2: number, y2: number): Motion {
  return { dur, ease: cubicBezier(x1, y1, x2, y2) }
}

/**
 * The house motions. A part picks one by name, so every walkthrough moves with the same character.
 * - `soft`: most moves; arrives without a visible bounce.
 * - `snappy`: small things that answer an action, such as a status.
 * - `pop`: something new arriving; a little overshoot.
 * - `glide`: the camera; slow, no overshoot.
 * - `out`: a fade or a colour change; the site's own ease.
 * - `linear`: something that tracks time itself, such as a growing bar.
 * - `step`: an instant change, for a value with no in-between.
 */
export const MOTION = {
  soft: spring(0.55, 0.12),
  snappy: spring(0.32, 0.2),
  pop: spring(0.42, 0.42),
  glide: spring(1.1, 0),
  out: bezier(0.45, 0.2, 0.7, 0.2, 1),
  linear: { dur: 1, ease: (p: number) => p },
  step: { dur: 0, ease: () => 1 },
} satisfies Record<string, Motion>

export type MotionName = keyof typeof MOTION

/** A motion by name or as given, with its duration overridden when `dur` is set. */
export function motionOf(m: MotionName | Motion, dur?: number): Motion {
  const base = typeof m === 'string' ? MOTION[m] : m
  return dur === undefined ? base : { ease: base.ease, dur }
}

interface Tween {
  at: number
  dur: number
  from: number
  to: number
  ease: Ease
}

/** One number over time: a start value and the tweens that change it. */
export class Track {
  private tweens: Tween[] = []

  constructor(
    private readonly tl: Timeline,
    private readonly initial: number,
  ) {}

  /** Tweens to `to` from whatever the track is at the cursor (plus `delay`), and returns the seconds it takes. */
  to(to: number, motion: MotionName | Motion = 'soft', opts: { delay?: number; dur?: number } = {}): number {
    const m = motionOf(motion, opts.dur)
    const at = this.tl.now + (opts.delay ?? 0)
    // A tween that starts while another runs takes over from the value at that moment.
    const from = this.value(at)
    let i = this.tweens.length
    while (i > 0 && this.tweens[i - 1].at > at) i--
    this.tweens.splice(i, 0, { at, dur: m.dur, from, to, ease: m.ease })
    return (opts.delay ?? 0) + m.dur
  }

  /** Sets the value at the cursor with no in-between. */
  set(to: number, opts: { delay?: number } = {}): void {
    this.to(to, 'step', opts)
  }

  /** The value at scene time `t`. */
  value(t: number): number {
    let last: Tween | undefined
    for (const tw of this.tweens) {
      if (tw.at > t) break
      last = tw
    }
    if (!last) return this.initial
    const p = last.dur === 0 ? 1 : Math.min(1, (t - last.at) / last.dur)
    return last.from + (last.to - last.from) * last.ease(p)
  }

  /** When the last tween ends. */
  get end(): number {
    return this.tweens.reduce((e, tw) => Math.max(e, tw.at + tw.dur), 0)
  }
}

/** A step of the walkthrough: what the controls step through and the caption shows. */
export interface Beat {
  title: string
  caption: string
  /** What the presenter says here; shown only in presenter mode. */
  notes?: string
  /** The one step the walkthrough exists for. */
  moment?: boolean
  start: number
  end: number
}

/** How fast the scene runs while a caption is read: slow enough to look still, not stopped. */
const HOLD = 0.12

/** The cursor, the tracks and the beats of one scene. */
export class Timeline {
  /** The authoring cursor, in scene seconds. */
  now = 0
  readonly beats: Beat[] = []
  private readonly tracks: Track[] = []
  /** Playback speed over scene time; a scene slows down for its moment. */
  readonly speed: Track
  /** A second speed, multiplied in: almost still while a caption is read (`hold`). */
  private readonly holding: Track

  constructor() {
    this.speed = new Track(this, 1)
    this.holding = new Track(this, 1)
  }

  /** The playback speed at scene time `t`: real seconds times this is scene seconds. */
  rate(t: number): number {
    return this.speed.value(t) * this.holding.value(t)
  }

  /**
   * Slows the scene almost to a stop for about `real` seconds of real time, from the cursor, and moves
   * the cursor past it, so what is authored next happens after the viewer has read. Things already
   * moving keep moving, slowly, so the frame does not freeze.
   */
  hold(real: number): void {
    const down = 0.12
    const up = 0.3
    const span = real * HOLD * this.speed.value(this.now)
    this.holding.to(HOLD, 'out', { dur: down })
    this.holding.to(1, 'out', { dur: up, delay: span })
    this.now += span + up * 0.5
  }

  track(initial = 0): Track {
    const tr = new Track(this, initial)
    this.tracks.push(tr)
    return tr
  }

  /** Moves the cursor on by `s` seconds. */
  wait(s: number): void {
    this.now += s
  }

  /** Runs `fn` with the cursor `s` seconds later, then puts the cursor back: a move that overlaps. */
  after(s: number, fn: () => void): void {
    const saved = this.now
    this.now += s
    fn()
    this.now = saved
  }

  /** Authors one beat; its end is where `fn` leaves the cursor. While `fn` runs, it is the last beat. */
  beat(title: string, info: { caption: string; notes?: string; moment?: boolean }, fn: () => void): void {
    const beat: Beat = { title, ...info, start: this.now, end: this.now }
    this.beats.push(beat)
    fn()
    beat.end = this.now
  }

  /** Changes the playback speed at the cursor, smoothly. */
  slow(factor: number, ramp = 0.6): void {
    this.speed.to(factor, 'out', { dur: ramp })
  }

  get duration(): number {
    return Math.max(this.now, ...this.tracks.map((t) => t.end))
  }
}
