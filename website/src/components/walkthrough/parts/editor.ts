/**
 * Editor: a code window showing real snippets, coloured like every snippet on the site.
 *
 * - `highlight(from, to?)` moves one highlight bar onto a line or a range of lines; `unhighlight()`.
 * - `glow(strength)` makes the highlight shine, for the line the scene is about.
 * - `clear()` and `type(from, to, cps?)` hide the code and type it in again, with a caret.
 * - `fold(from, to?)` takes lines out of the file at the cursor, and `insert(from, to?)` opens room
 *   for them again and types them in: a line the scene adds to the code. The lines below move, and
 *   the line numbers count only the lines that are there.
 * - `lines(from, to?)` is an anchor at the left edge of a range, where a card can lift out of it.
 *
 * A window with several files has a tab for each (`Editor.astro`'s `tabs`). `file(i)` cross-fades to
 * file `i`, and every call above then works on that file until the next `file`. `close(i)` takes the
 * tab of file `i` away, as a deleted file. Two files may share a tab, so `file` can change what one
 * file says.
 *
 * Line numbers in every call are the snippet's own, folded lines included. Files are counted from 0,
 * in the order of `src`.
 */
import { css, Panel, type Anchor, type Stage } from '../engine/stage'
import type { Motion, MotionName, Timeline, Track } from '../engine/time'

const LINE = 36
const PAD = 20

/** One file of the window: its lines, its highlight and its caret. */
class Sheet {
  /** The highlight's first line (0-based, fractional while it slides) and how many lines it covers. */
  readonly hlAt: Track
  readonly hlN: Track
  readonly hlOn: Track
  readonly hlGlow: Track
  readonly lineEls: HTMLElement[]
  readonly textEls: HTMLElement[]
  readonly numEls: HTMLElement[]
  readonly chars: Track[]
  /** How open each line is: 1 is in the file, 0 is folded away. */
  readonly open: Track[]
  readonly caretLine: Track
  readonly caretOn: Track
  /** 1 while this file is the one the window shows. */
  readonly on: Track
  readonly hl: HTMLElement
  readonly caret: HTMLElement
  /** The tab this file is shown under. */
  readonly tab: number
  textLeft = 0
  codeTop = 0
  textX = 0

  constructor(
    private readonly tl: Timeline,
    readonly code: HTMLElement,
    first: boolean,
  ) {
    this.hlAt = tl.track(0)
    this.hlN = tl.track(1)
    this.hlOn = tl.track(0)
    this.hlGlow = tl.track(0)
    this.lineEls = [...code.querySelectorAll<HTMLElement>('.wt-line')]
    this.textEls = this.lineEls.map((l) => l.querySelector<HTMLElement>('.wt-text')!)
    this.numEls = this.lineEls.map((l) => l.querySelector<HTMLElement>('.wt-ln')!)
    this.chars = this.lineEls.map(() => tl.track(999))
    this.open = this.lineEls.map(() => tl.track(1))
    this.caretLine = tl.track(1)
    this.caretOn = tl.track(0)
    this.on = tl.track(first ? 1 : 0)
    this.hl = code.querySelector('.wt-hl')!
    this.caret = code.querySelector('.wt-caret')!
    this.tab = Number(code.dataset.tab ?? 0)
  }

  len(line: number): number {
    return this.textEls[line - 1]?.textContent?.length ?? 0
  }

  /** The y of position `pos` (a 0-based line, fractional inside a line) from the top of the code, at `t`. */
  yAt(pos: number, t: number): number {
    let y = PAD
    const whole = Math.floor(pos)
    for (let i = 0; i < whole && i < this.open.length; i++) y += LINE * this.open[i].value(t)
    const part = pos - whole
    if (part > 0 && whole < this.open.length) y += part * LINE * this.open[whole].value(t)
    return y
  }

  render(t: number): void {
    const on = this.on.value(t)
    css(this.code, 'opacity', String(Math.min(1, Math.max(0, on))))
    css(this.code, 'visibility', on <= 0.001 ? 'hidden' : 'visible')
    if (on <= 0.001) return
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

    const hlOn = this.hlOn.value(t)
    const at = this.hlAt.value(t)
    const y0 = this.yAt(at, t)
    css(this.hl, 'opacity', String(hlOn))
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

export class Editor extends Panel {
  private readonly sheets: Sheet[]
  private readonly tabs: { el: HTMLElement; open: Track; on: Track }[]
  /** The file the scene's calls work on, from the last `file()` at authoring time. */
  private current = 0

  constructor(stage: Stage, el: HTMLElement) {
    super(stage, el)
    this.sheets = [...el.querySelectorAll<HTMLElement>('.wt-code')].map((code, i) => new Sheet(this.tl, code, i === 0))
    this.tabs = [...el.querySelectorAll<HTMLElement>('.wt-etab')].map((tab, i) => ({
      el: tab,
      open: this.tl.track(1),
      on: this.tl.track(i === this.sheets[0]?.tab ? 1 : 0),
    }))
  }

  private get sheet(): Sheet {
    return this.sheets[this.current]
  }

  /** How many lines the current file has. */
  get count(): number {
    return this.sheet.lineEls.length
  }

  /** Cross-fades to file `i`, and makes its tab the open one; returns the seconds. */
  file(i: number): number {
    if (!this.sheets[i]) throw new Error(`Editor ${this.name}: no file ${i}`)
    if (i === this.current) return 0
    const from = this.sheet
    const to = this.sheets[i]
    this.current = i
    from.on.to(0, 'out', { dur: 0.3 })
    for (const [k, tab] of this.tabs.entries()) if (k === from.tab || k === to.tab) tab.on.to(k === to.tab ? 1 : 0, 'out', { dur: 0.3 })
    return to.on.to(1, 'out', { dur: 0.35 })
  }

  /** Takes the tab of file `i` away, as a file that is deleted; returns the seconds. */
  close(i: number): number {
    const tab = this.tabs[this.sheets[i].tab]
    tab.on.to(0, 'out', { dur: 0.3 })
    return tab.open.to(0, 'soft')
  }

  highlight(from: number, to = from, motion: MotionName | Motion = 'soft'): number {
    const s = this.sheet
    const first = s.hlOn.value(this.tl.now) < 0.01
    // The first highlight appears in place; later ones slide from line to line.
    if (first) {
      s.hlAt.set(from - 1)
      s.hlN.set(to - from + 1)
      return s.hlOn.to(1, 'out')
    }
    s.hlAt.to(from - 1, motion)
    return s.hlN.to(to - from + 1, motion)
  }

  unhighlight(): number {
    this.sheet.hlGlow.to(0, 'out')
    return this.sheet.hlOn.to(0, 'out')
  }

  glow(strength = 1, motion: MotionName | Motion = 'soft'): number {
    return this.sheet.hlGlow.to(strength, motion)
  }

  /** Hides every line at the cursor, so `type` can bring them back. */
  clear(): void {
    for (const c of this.sheet.chars) c.set(0)
  }

  /** Takes lines `from`..`to` out of the current file, or of file `file`, at the cursor, with no motion. */
  fold(from: number, to = from, file = this.current): void {
    const s = this.sheets[file]
    for (let l = from; l <= to; l++) {
      s.open[l - 1].set(0)
      s.chars[l - 1].set(0)
    }
  }

  /**
   * Opens room for the folded lines `from`..`to`, the lines below moving down, and types them in.
   * Returns the seconds it takes; the cursor stays where it was.
   */
  insert(from: number, to = from, cps = 30): number {
    const start = this.tl.now
    let opened = 0
    for (let l = from; l <= to; l++) opened = Math.max(opened, this.sheet.open[l - 1].to(1, 'soft'))
    this.tl.wait(opened * 0.6)
    const typed = this.type(from, to, cps)
    this.tl.now = start
    return opened * 0.6 + typed
  }

  /** Types lines `from`..`to` at `cps` characters a second, with the caret; returns the seconds. */
  type(from: number, to = from, cps = 38): number {
    const s = this.sheet
    const start = this.tl.now
    s.caretOn.set(1)
    for (let l = from; l <= to; l++) {
      const n = s.len(l)
      s.caretLine.set(l)
      const dur = Math.max(n / cps, 0.05)
      s.chars[l - 1].to(n, 'linear', { dur })
      this.tl.wait(dur + 0.08)
    }
    const spent = this.tl.now - start
    this.tl.now = start
    s.caretOn.set(0, { delay: spent + 0.5 })
    return spent
  }

  /** The left edge of a range of lines of the current file, where a card lifts out of the code. */
  lines(from: number, to = from, dx = 0): Anchor {
    const s = this.sheet
    return (t) => ({ x: s.textX + dx, y: s.codeTop + (s.yAt(from - 1, t) + s.yAt(to, t)) / 2 })
  }

  measure(): void {
    for (const s of this.sheets) {
      s.textLeft = s.textEls[0]?.offsetLeft ?? 64
      s.codeTop = this.anchor(s.code, 'top').y
      s.textX = s.textEls[0] ? this.anchor(s.textEls[0], 'left').x : 0
    }
  }

  render(t: number): void {
    this.renderPanel(t)
    for (const s of this.sheets) s.render(t)
    for (const tab of this.tabs) {
      const open = Math.max(0, tab.open.value(t))
      css(tab.el, '--open', open.toFixed(3))
      css(tab.el, '--on', Math.max(0, Math.min(1, tab.on.value(t))).toFixed(3))
      css(tab.el, 'display', open <= 0.001 ? 'none' : '')
    }
  }
}
