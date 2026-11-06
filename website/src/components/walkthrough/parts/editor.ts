/**
 * Editor: a code window showing one real snippet, coloured like every snippet on the site.
 *
 * - `highlight(from, to?)` moves one highlight bar onto a line or a range of lines; `unhighlight()`.
 * - `glow(strength)` makes the highlight shine, for the line the scene is about.
 * - `clear()` and `type(from, to, cps?)` hide the code and type it in again, with a caret.
 * - `lines(from, to?)` is an anchor at the left edge of a range, where a card can lift out of it.
 */
import { css, Panel, type Anchor, type Stage } from '../engine/stage'
import type { Motion, MotionName } from '../engine/time'

const LINE = 36
const PAD = 20

export class Editor extends Panel {
  private readonly hlY = this.tl.track(0)
  private readonly hlH = this.tl.track(LINE)
  private readonly hlOn = this.tl.track(0)
  private readonly hlGlow = this.tl.track(0)
  private readonly lineEls: HTMLElement[]
  private readonly chars: ReturnType<Stage['tl']['track']>[]
  private readonly caretLine = this.tl.track(1)
  private readonly caretOn = this.tl.track(0)
  private readonly hl: HTMLElement
  private readonly caret: HTMLElement
  private textLeft = 0

  constructor(stage: Stage, el: HTMLElement) {
    super(stage, el)
    this.lineEls = [...el.querySelectorAll<HTMLElement>('.wt-text')]
    this.chars = this.lineEls.map(() => this.tl.track(999))
    this.hl = el.querySelector('.wt-hl')!
    this.caret = el.querySelector('.wt-caret')!
  }

  /** How many lines the snippet has. */
  get count(): number {
    return this.lineEls.length
  }

  private len(line: number): number {
    return this.lineEls[line - 1]?.textContent?.length ?? 0
  }

  highlight(from: number, to = from, motion: MotionName | Motion = 'soft'): number {
    const y = PAD + (from - 1) * LINE
    const first = this.hlOn.value(this.tl.now) < 0.01
    // The first highlight appears in place; later ones slide from line to line.
    if (first) {
      this.hlY.set(y)
      this.hlH.set((to - from + 1) * LINE)
      return this.hlOn.to(1, 'out')
    }
    this.hlY.to(y, motion)
    return this.hlH.to((to - from + 1) * LINE, motion)
  }

  unhighlight(): number {
    this.hlGlow.to(0, 'out')
    return this.hlOn.to(0, 'out')
  }

  glow(strength = 1, motion: MotionName | Motion = 'soft'): number {
    return this.hlGlow.to(strength, motion)
  }

  /** Hides every line at the cursor, so `type` can bring them back. */
  clear(): void {
    for (const c of this.chars) c.set(0)
  }

  /** Types lines `from`..`to` at `cps` characters a second, with the caret; returns the seconds. */
  type(from: number, to = from, cps = 38): number {
    const start = this.tl.now
    this.caretOn.set(1)
    for (let l = from; l <= to; l++) {
      const n = this.len(l)
      this.caretLine.set(l)
      const dur = Math.max(n / cps, 0.05)
      this.chars[l - 1].to(n, 'linear', { dur })
      this.tl.wait(dur + 0.08)
    }
    const spent = this.tl.now - start
    this.tl.now = start
    this.caretOn.set(0, { delay: spent + 0.5 })
    return spent
  }

  /** The left edge of a range of lines, where a card lifts out of the code. */
  lines(from: number, to = from, dx = 0): Anchor {
    return () => {
      const a = this.lineAt[from - 1]
      const b = this.lineAt[to - 1]
      return { x: a.x + dx, y: (a.y + b.y) / 2 }
    }
  }

  private lineAt: { x: number; y: number }[] = []

  measure(): void {
    this.textLeft = this.lineEls[0]?.offsetLeft ?? 64
    this.lineAt = this.lineEls.map((e) => this.anchor(e, 'left'))
  }

  render(t: number): void {
    this.renderPanel(t)
    const on = this.hlOn.value(t)
    css(this.hl, 'opacity', String(on))
    css(this.hl, 'transform', `translateY(${this.hlY.value(t)}px)`)
    css(this.hl, 'height', `${this.hlH.value(t)}px`)
    css(this.hl, '--glow', String(this.hlGlow.value(t)))
    this.lineEls.forEach((el, i) => css(el, '--chars', String(Math.round(this.chars[i].value(t)))))

    const caretOn = this.caretOn.value(t)
    const line = Math.round(this.caretLine.value(t))
    const ch = Math.min(Math.round(this.chars[line - 1]?.value(t) ?? 0), this.len(line))
    // The caret blinks only while it waits, as an editor's does.
    const typing = ch < this.len(line)
    const blink = typing ? 1 : Math.cos(t * Math.PI * 2.2) > -0.2 ? 1 : 0
    css(this.caret, 'opacity', String(caretOn * blink))
    css(this.caret, 'transform', `translate(calc(${this.textLeft}px + ${ch}ch), ${PAD + (line - 1) * LINE + 4}px)`)
  }
}
