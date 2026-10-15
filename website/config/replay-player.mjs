/**
 * Plays each replay on a page: the browser half of `src/components/Replay.astro`.
 *
 * `replay-frames.mjs` writes the replay's HTML and its frames, one per
 * keystroke, when the site is built. This script draws a frame into the code
 * area: the coloured text, the text cursor, the open completion list narrowed by
 * `replay-narrow.mjs`, and the red underline and its card on a hover step.
 *
 * The rules it keeps are `rule:testing/a-replay-plays-what-the-language-server-returned`'s:
 * a replay starts when at least half of it is in view, pauses when it leaves,
 * plays once and stops on its last frame. Only one replay on a page plays at a
 * time. With `prefers-reduced-motion` it does not start, and shows its poster
 * frame. Previous and next step one frame and pause. The frame is `inert`, so
 * the code never takes focus and a click on it does nothing.
 *
 * Each replay's root carries `data-replay-state` (`ready`, `playing`, `paused`
 * or `ended`) and `data-replay-frame`, which the browser test reads.
 */

import { codeHtml, escapeHtml, frameColours, position } from './replay-frames.mjs'
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

/** Starts every replay under `root` that has not started yet. */
export function startReplays(root = document) {
  for (const el of root.querySelectorAll('[data-replay]')) {
    if (!el.dataset.replayState) new Player(el)
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
    this.button = el.querySelector('[data-replay-do="play"]')
    this.timer = 0
    // A replay that left the view while playing plays on when it comes back.
    this.resume = false
    this.still = matchMedia('(prefers-reduced-motion: reduce)').matches
    this.button.addEventListener('click', () => (this.state === 'playing' ? this.pause() : this.play()))
    el.querySelector('[data-replay-do="previous"]').addEventListener('click', () => this.step(-1))
    el.querySelector('[data-replay-do="next"]').addEventListener('click', () => this.step(1))
    this.show(this.still ? this.data.poster : 0)
    this.set(this.still ? 'paused' : 'ready')
    new IntersectionObserver((entries) => this.seen(entries.at(-1).intersectionRatio >= 0.5), { threshold: [0, 0.5] }).observe(el)
  }

  // A replay that comes into view starts only when no other one plays, and a click on play always starts it.
  seen(inView) {
    if (inView && !this.still && current === null && (this.state === 'ready' || this.resume)) this.play()
    else if (!inView && this.state === 'playing') {
      this.pause()
      this.resume = true
    }
  }

  play() {
    if (current && current !== this) current.pause()
    current = this
    this.resume = false
    if (this.index >= this.last) this.show(0)
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
    this.resume = false
    this.set('paused')
  }

  end() {
    clearTimeout(this.timer)
    if (current === this) current = null
    this.set('ended')
  }

  step(by) {
    this.pause()
    this.show(Math.min(this.last, Math.max(0, this.index + by)))
    if (this.index === this.last) this.end()
  }

  set(state) {
    this.state = state
    this.el.dataset.replayState = state
    this.button.textContent = state === 'playing' ? 'Pause' : state === 'ended' ? 'Play again' : 'Play'
  }

  show(index) {
    this.index = index
    this.el.dataset.replayFrame = String(index)
    const frame = this.data.frames[index]
    const text = this.texts[index]
    const diagnostic = frame.hover === null ? null : this.data.asks[frame.hover]
    let html = codeHtml(text, frame.cursor, diagnostic, frameColours(this.data.paint, index))
    if (frame.list) html += this.list(frame, text)
    if (diagnostic) {
      html += `<div class="nv-hover" style="--nv-line: ${diagnostic.line + 1}; --nv-col: ${diagnostic.col}">`
      html += `<div class="nv-hover-message">${escapeHtml(diagnostic.message)}</div>`
      if (diagnostic.code) html += `<div class="nv-hover-code">${escapeHtml(diagnostic.code)}</div>`
      html += '</div>'
    }
    this.code.innerHTML = html
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
