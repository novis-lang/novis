/**
 * Plays each replay on a page: the browser half of `src/components/Replay.astro`.
 *
 * `replay-frames.mjs` writes the replay's HTML and its frames, one per
 * keystroke, when the site is built. This script draws a frame into the code
 * area: the coloured text, the text cursor, the open completion list narrowed by
 * `replay-narrow.mjs`, the red underline and its card on a hover step, and the
 * panel of a `rename`, `action`, `calls` or `card` step, or the code lenses of a
 * `lens` step, drawn by `replay-frames.mjs`'s `panelHtml` and `codeHtml`. A
 * debugger replay's frame has its breakpoints by the line numbers, the line
 * the program stopped on, and a panel with the call stack and the variables. A
 * bar under the code shows how far the replay has played. A line wider than
 * the window does not wrap: the code scrolls sideways, and after each frame the
 * player scrolls it so the cursor and the open list or card are in view, as an
 * editor does. The poster frame is scrolled the same way.
 *
 * The rules it keeps are `rule:testing/a-replay-plays-what-the-language-server-returned`'s:
 * a replay never starts by itself, and shows its poster frame until the reader
 * presses its button. The button plays from the first frame, pauses, and plays
 * again from the first frame after the end. A replay plays once and stops on
 * its last frame, pauses when it leaves the view, and only one replay on a page
 * plays at a time. The frame is `inert`, so the code never takes focus, a
 * click on it does nothing, and only the player scrolls it.
 *
 * Each replay's root carries `data-replay-state` (`ready`, `playing`, `paused`
 * or `ended`) and `data-replay-frame`, and has its player as `nvReplay`. The
 * browser test reads the first two and draws frames with `nvReplay.show`.
 */

import { buttonHtml, codeHtml, escapeHtml, frameColours, frameDebug, frameLenses, panelHtml, position } from './replay-frames.mjs'
import { narrow } from './replay-narrow.mjs'

/** The symbol and colour group of a completion row's kind icon. */
const KIND = {
  method: ['m', 'fn'],
  function: ['f', 'fn'],
  constructor: ['f', 'fn'],
  field: ['p', 'var'],
  property: ['p', 'var'],
  variable: ['v', 'var'],
  constant: ['c', 'var'],
  class: ['C', 'type'],
  interface: ['I', 'type'],
  struct: ['S', 'type'],
  enum: ['E', 'type'],
  enumMember: ['e', 'var'],
  module: ['N', 'type'],
  typeParameter: ['T', 'type'],
  keyword: ['k', 'word'],
  snippet: ['s', 'word'],
}

/** The most rows a list shows at once. */
const LIST_ROWS = 8

/** The replay that plays now, so a second one pauses it. */
let current = null

/** Sets up every replay under `root` that has no player yet. */
export function startReplays(root = document) {
  for (const el of root.querySelectorAll('[data-replay]')) {
    if (!el.dataset.replayState) el.nvReplay = new Player(el)
  }
}

class Player {
  constructor(el) {
    this.el = el
    this.data = JSON.parse(el.querySelector('[data-replay-data]').textContent)
    let text = this.data.start
    this.texts = this.data.frames.map(({ edit }) => {
      if (edit) text = text.slice(0, edit[0]) + edit[2] + text.slice(edit[0] + edit[1])
      return text
    })
    this.last = this.data.frames.length - 1
    this.code = el.querySelector('.nv-replay-code')
    this.progress = el.querySelector('.nv-replay-progress')
    this.button = el.querySelector('[data-replay-do="play"]')
    this.label = this.button.dataset.replayLabel
    this.timer = 0
    this.button.addEventListener('click', () => (this.state === 'playing' ? this.pause() : this.play()))
    this.show(this.data.poster)
    this.set('ready')
    const frame = el.querySelector('.nv-replay-frame')
    new IntersectionObserver((entries) => {
      if (!entries.at(-1).isIntersecting && this.state === 'playing') this.pause()
    }).observe(frame)
    // A web font that loads late, or a window that changes width, moves the cursor, so the scroll is set again.
    new ResizeObserver(() => this.follow()).observe(this.code)
    document.fonts?.ready.then(() => this.follow())
  }

  play() {
    if (current && current !== this) current.pause()
    current = this
    if (this.state === 'ready' || this.state === 'ended') {
      // The first frame starts from the left edge, as a file does when an editor opens it.
      this.code.scrollLeft = 0
      this.show(0)
    }
    this.set('playing')
    this.schedule()
  }

  schedule() {
    clearTimeout(this.timer)
    this.timer = setTimeout(() => {
      this.show(this.index + 1)
      if (this.index >= this.last) this.end()
      else this.schedule()
    }, this.data.frames[this.index].delay)
  }

  pause() {
    clearTimeout(this.timer)
    if (current === this) current = null
    this.set('paused')
  }

  end() {
    clearTimeout(this.timer)
    if (current === this) current = null
    this.set('ended')
  }

  set(state) {
    this.state = state
    this.el.dataset.replayState = state
    this.button.innerHTML = state === 'playing' ? buttonHtml('pause', 'Pause') : state === 'ended' ? buttonHtml('again', 'Play again') : buttonHtml('play', this.label)
    this.bar()
  }

  /** The progress bar: empty before the replay plays, then the share of frames shown. */
  bar() {
    const done = this.state === 'ready' ? 0 : this.last === 0 ? 1 : this.index / this.last
    this.progress.style.setProperty('--nv-progress', String(done))
  }

  show(index) {
    this.index = index
    this.el.dataset.replayFrame = String(index)
    const frame = this.data.frames[index]
    const text = this.texts[index]
    const diagnostic = frame.hover === null ? null : this.data.asks[frame.hover]
    let html = codeHtml(text, frame.cursor, diagnostic, frameColours(this.data.paint, index), frameLenses(this.data.asks, frame.panel), frameDebug(this.data, frame.panel))
    if (frame.list) html += this.list(frame, text)
    if (frame.panel) html += panelHtml(this.data.asks[frame.panel.ask], frame.panel)
    if (diagnostic) {
      html += `<div class="nv-hover" style="--nv-line: ${diagnostic.line + 1}; --nv-col: ${diagnostic.col}">`
      html += `<div class="nv-hover-message">${escapeHtml(diagnostic.message)}</div>`
      if (diagnostic.code) html += `<div class="nv-hover-code">${escapeHtml(diagnostic.code)}</div>`
      html += '</div>'
    }
    this.code.innerHTML = html
    this.follow()
    if (this.state) this.bar()
  }

  /**
   * Scrolls the code sideways, as an editor does, so the cursor and the open list or card are in view
   * right of the line numbers, with a margin of up to 4 characters on each side. A frame that is
   * already in view with its margin keeps the scroll it has. A list or a card still past the right
   * edge moves left until it fits, as in VS Code.
   */
  follow() {
    const code = this.code
    const boxes = [...code.querySelectorAll('.nv-list, .nv-hover')]
    for (const box of boxes) box.style.translate = ''
    const view = code.getBoundingClientRect()
    const gutter = code.querySelector('.nv-ln')?.getBoundingClientRect().width ?? 0
    const margin = 4 * 0.6 * parseFloat(getComputedStyle(code).fontSize)
    const parts = [code.querySelector('.nv-caret'), ...boxes].filter((e) => e !== null).map((e) => e.getBoundingClientRect())
    if (parts.length > 0) {
      // Where the parts start and end, measured from the left edge of the code's whole width.
      const from = Math.min(...parts.map((r) => r.left)) - view.left + code.scrollLeft
      const to = Math.max(...parts.map((r) => r.right)) - view.left + code.scrollLeft
      // Parts that only just fit get a smaller margin, so the margin never pushes one of them out.
      const pad = Math.max(0, Math.min(margin, (code.clientWidth - gutter - (to - from)) / 2))
      let left = code.scrollLeft
      if (to + pad > left + code.clientWidth) left = to + pad - code.clientWidth
      if (from - pad < left + gutter) left = from - pad - gutter
      if (left !== code.scrollLeft) code.scrollLeft = Math.max(0, left)
    }
    const edge = view.left + code.clientWidth
    for (const box of boxes) {
      const over = box.getBoundingClientRect().right - edge
      if (over > 0) box.style.translate = `${-Math.ceil(over)}px 0`
    }
  }

  /** The open list, narrowed by what was typed since it opened, best row first and selected. */
  list(frame, text) {
    const { rows } = this.data.asks[frame.list.ask]
    const kept = narrow(
      rows.map((r) => r.label),
      text.slice(frame.list.from, frame.cursor),
    ).slice(0, LIST_ROWS)
    if (kept.length === 0) return ''
    const at = position(text, frame.list.from)
    const items = kept.map(({ index, matched }, i) => {
      const row = rows[index]
      const [symbol, group] = KIND[row.kind] ?? ['·', 'word']
      let label = ''
      for (let j = 0; j < row.label.length; j++) label += matched.includes(j) ? `<mark>${escapeHtml(row.label[j])}</mark>` : escapeHtml(row.label[j])
      return (
        `<li class="nv-row${i === 0 ? ' nv-selected' : ''}">` +
        `<span class="nv-kind nv-kind-${group}" title="${escapeHtml(row.kind)}">${symbol}</span>` +
        `<span class="nv-label">${label}</span><span class="nv-after">${escapeHtml(row.after)}</span>` +
        `<span class="nv-right">${escapeHtml(row.right)}</span></li>`
      )
    })
    return `<ul class="nv-list" style="--nv-line: ${at.line + 1}; --nv-col: ${at.col}">${items.join('')}</ul>`
  }
}
