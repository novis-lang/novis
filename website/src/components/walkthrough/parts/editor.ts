/**
 * Editor: a code window showing one real snippet, coloured like every snippet on the site.
 *
 * - `highlight(from, to?)` moves one highlight bar onto a line or a range of lines; `unhighlight()`.
 * - `glow(strength)` makes the highlight shine, for the line the scene is about.
 * - `clear()` and `type(from, to, cps?)` hide the code and type it in again, with a caret.
 * - `fold(from, to?)` takes lines out of the file at the cursor, and `insert(from, to?)` opens room
 *   for them again and types them in: a line the scene adds to the code. The lines below move, and
 *   the line numbers count only the lines that are there.
 * - `lines(from, to?)` is an anchor at the left edge of a range, where a card can lift out of it.
 *
 * Line numbers in every call are the snippet's own, folded lines included.
 */
import { css, Panel, type Anchor, type Stage } from '../engine/stage'
import type { Motion, MotionName, Track } from '../engine/time'

const LINE = 36
const PAD = 20

export class Editor extends Panel {
  /** The highlight's first line (0-based, fractional while it slides) and how many lines it covers. */
  private readonly hlAt = this.tl.track(0)
  private readonly hlN = this.tl.track(1)
  private readonly hlOn = this.tl.track(0)
  private readonly hlGlow = this.tl.track(0)
  private readonly lineEls: HTMLElement[]
  private readonly textEls: HTMLElement[]
  private readonly numEls: HTMLElement[]
  private readonly chars: Track[]
  /** How open each line is: 1 is in the file, 0 is folded away. */
  private readonly open: Track[]
  private readonly caretLine = this.tl.track(1)
  private readonly caretOn = this.tl.track(0)
  private readonly hl: HTMLElement
  private readonly caret: HTMLElement
  private readonly code: HTMLElement
  private textLeft = 0
  private codeTop = 0
  private textX = 0

  constructor(stage: Stage, el: HTMLElement) {
    super(stage, el)
    this.lineEls = [...el.querySelectorAll<HTMLElement>('.wt-line')]
    this.textEls = this.lineEls.map((l) => l.querySelector<HTMLElement>('.wt-text')!)
    this.numEls = this.lineEls.map((l) => l.querySelector<HTMLElement>('.wt-ln')!)
    this.chars = this.lineEls.map(() => this.tl.track(999))
    this.open = this.lineEls.map(() => this.tl.track(1))
    this.hl = el.querySelector('.wt-hl')!
    this.caret = el.querySelector('.wt-caret')!
    this.code = el.querySelector('.wt-code')!
  }

  /** How many lines the snippet has. */
  get count(): number {
    return this.lineEls.length
  }

  private len(line: number): number {
    return this.textEls[line - 1]?.textContent?.length ?? 0
  }

  /** The y of position `pos` (a 0-based line, fractional inside a line) from the top of the code, at `t`. */
  private yAt(pos: number, t: number): number {
    let y = PAD
    const whole = Math.floor(pos)
    for (let i = 0; i < whole && i < this.open.length; i++) y += LINE * this.open[i].value(t)
    const part = pos - whole
    if (part > 0 && whole < this.open.length) y += part * LINE * this.open[whole].value(t)
    return y
  }

  highlight(from: number, to = from, motion: MotionName | Motion = 'soft'): number {
    const first = this.hlOn.value(this.tl.now) < 0.01
    // The first highlight appears in place; later ones slide from line to line.
    if (first) {
      this.hlAt.set(from - 1)
      this.hlN.set(to - from + 1)
      return this.hlOn.to(1, 'out')
    }
    this.hlAt.to(from - 1, motion)
    return this.hlN.to(to - from + 1, motion)
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

  /** Takes lines `from`..`to` out of the file at the cursor, with no motion. */
  fold(from: number, to = from): void {
    for (let l = from; l <= to; l++) {
      this.open[l - 1].set(0)
      this.chars[l - 1].set(0)
    }
  }

  /**
   * Opens room for the folded lines `from`..`to`, the lines below moving down, and types them in.
   * Returns the seconds it takes; the cursor stays where it was.
   */
  insert(from: number, to = from, cps = 30): number {
    const start = this.tl.now
    let opened = 0
    for (let l = from; l <= to; l++) opened = Math.max(opened, this.open[l - 1].to(1, 'soft'))
    this.tl.wait(opened * 0.6)
    const typed = this.type(from, to, cps)
    this.tl.now = start
    return opened * 0.6 + typed
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
    return (t) => ({ x: this.textX + dx, y: this.codeTop + (this.yAt(from - 1, t) + this.yAt(to, t)) / 2 })
  }

  measure(): void {
    this.textLeft = this.textEls[0]?.offsetLeft ?? 64
    this.codeTop = this.anchor(this.code, 'top').y
    this.textX = this.textEls[0] ? this.anchor(this.textEls[0], 'left').x : 0
  }

  render(t: number): void {
    this.renderPanel(t)
    let shown = 0
    this.lineEls.forEach((el, i) => {
      const open = this.open[i].value(t)
      css(el, 'height', `${(LINE * Math.max(0, open)).toFixed(2)}px`)
      css(el, 'opacity', String(Math.min(1, Math.max(0, open * 1.6 - 0.6))))
      css(el, 'overflow', open < 0.999 ? 'hidden' : 'visible')
      if (open > 0.5) shown++
      const num = open > 0.5 ? String(shown) : ''
      if (this.numEls[i].textContent !== num) this.numEls[i].textContent = num
      css(this.textEls[i], '--chars', String(Math.round(this.chars[i].value(t))))
    })

    const on = this.hlOn.value(t)
    const at = this.hlAt.value(t)
    const y0 = this.yAt(at, t)
    css(this.hl, 'opacity', String(on))
    css(this.hl, 'transform', `translateY(${y0}px)`)
    css(this.hl, 'height', `${this.yAt(at + this.hlN.value(t), t) - y0}px`)
    css(this.hl, '--glow', String(this.hlGlow.value(t)))

    const caretOn = this.caretOn.value(t)
    const line = Math.round(this.caretLine.value(t))
    const ch = Math.min(Math.round(this.chars[line - 1]?.value(t) ?? 0), this.len(line))
    // The caret blinks only while it waits, as an editor's does.
    const typing = ch < this.len(line)
    const blink = typing ? 1 : Math.cos(t * Math.PI * 2.2) > -0.2 ? 1 : 0
    css(this.caret, 'opacity', String(caretOn * blink))
    css(this.caret, 'transform', `translate(calc(${this.textLeft}px + ${ch}ch), ${this.yAt(line - 1, t) + 4}px)`)
  }
}
