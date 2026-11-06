/**
 * The speaker-notes window: a second browser window for the person who presents or records a
 * walkthrough. N on the stage opens it. It shows the current step, its caption, the step's `notes`,
 * how long the step still runs, and the next step. Its buttons and keys drive the stage, so the stage
 * can fill a screen that is being recorded while the presenter reads and steers from this window.
 *
 * The window is an `about:blank` popup whose page this module builds and updates from the opener,
 * so it needs no route of its own. Its colours are the stage's tokens, read once when it opens.
 * Times are in real seconds: a step the scene slows down for its moment lasts longer than its scene
 * time, and a presenter times their words against the clock.
 */
import type { Stage } from './stage'

const WIDTH = 560
const HEIGHT = 680

/** Samples per scene second of the table below. */
const RATE = 120

/**
 * The real time at each scene time, at the scene's playback speed: entry `k` is the real seconds from
 * the start to scene time `k / RATE`. It is built once, when the window first opens.
 */
function realClock(stage: Stage): (t: number) => number {
  const n = Math.ceil(stage.duration * RATE) + 1
  const table = new Float64Array(n)
  for (let k = 1; k < n; k++) table[k] = table[k - 1] + 1 / RATE / Math.max(stage.tl.speed.value((k - 0.5) / RATE), 0.05)
  return (t) => {
    const x = Math.min(Math.max(t * RATE, 0), n - 1)
    const k = Math.floor(x)
    return k >= n - 1 ? table[n - 1] : table[k] + (table[k + 1] - table[k]) * (x - k)
  }
}

const fmt = (s: number) => `${s.toFixed(1)} s`

/**
 * The page's `@font-face` rules, so the window uses the site's font. The popup's base URL is the
 * page's, so their relative URLs load the same files. A sheet from another origin cannot be read and
 * is skipped.
 */
function fontFaces(): string {
  let out = ''
  for (const sheet of Array.from(document.styleSheets)) {
    try {
      for (const rule of Array.from(sheet.cssRules)) if (rule instanceof CSSFontFaceRule) out += rule.cssText + '\n'
    } catch {
      // A cross-origin sheet.
    }
  }
  return out
}

const STYLE = `
  * { box-sizing: border-box }
  body { margin: 0; padding: 20px 22px; background: var(--bg); color: var(--text);
    font: 16px/1.5 var(--font); display: flex; flex-direction: column; gap: 14px; min-height: 100vh }
  header { display: flex; align-items: baseline; gap: 10px }
  .n { font-weight: 700; font-variant-numeric: tabular-nums }
  .title { font-weight: 600 }
  .moment { margin-left: auto; padding: 2px 10px; border-radius: 999px; background: var(--moment);
    color: var(--on-moment); font-size: 13px; font-weight: 700 }
  .bar { height: 6px; border-radius: 3px; background: var(--line); overflow: hidden }
  .bar i { display: block; height: 100%; background: var(--run); transform-origin: 0 0 }
  .times { display: flex; justify-content: space-between; color: var(--muted); font-size: 13px;
    font-variant-numeric: tabular-nums }
  .label { color: var(--muted); font-size: 12px; font-weight: 600; text-transform: uppercase;
    letter-spacing: .06em; margin-bottom: 4px }
  .caption { color: var(--muted) }
  .notes { font-size: 24px; line-height: 1.4; font-weight: 600; flex: 1 }
  .notes.empty { color: var(--faint); font-weight: 400; font-size: 18px }
  .next { border-top: 1px solid var(--line); padding-top: 12px; color: var(--muted) }
  .next b { color: var(--text) }
  nav { display: flex; gap: 8px }
  button { flex: 1; padding: 10px; border-radius: 10px; border: 1px solid var(--line);
    background: transparent; color: var(--text); font: inherit; font-weight: 600; cursor: pointer }
  button:hover { border-color: var(--muted) }
  button:focus-visible { outline: 2px solid var(--run); outline-offset: 2px }
  .keys { color: var(--faint); font-size: 12px; text-align: center }
`

const BODY = `
  <header><span class="n"></span><span class="title"></span><span class="moment" hidden>The moment</span></header>
  <div><div class="bar"><i></i></div><div class="times"><span class="left"></span><span class="total"></span></div></div>
  <div><div class="label">Caption</div><div class="caption"></div></div>
  <div class="notes"></div>
  <div class="next"></div>
  <nav><button data-do="prev">Back</button><button data-do="toggle">Play</button><button data-do="next">Next step</button></nav>
  <div class="keys">Space plays and pauses · → next step · ← back · Home goes to the start</div>
`

/** Wires N on the stage to open the notes window, and keeps an open one up to date. */
export function bindNotes(stage: Stage): void {
  const root = stage.root
  const beats = stage.tl.beats
  let real: ((t: number) => number) | null = null
  let win: Window | null = null
  let shown = -1
  let playing: boolean | null = null

  const q = <E extends HTMLElement = HTMLElement>(s: string) => win!.document.querySelector<E>(s)!

  const update = () => {
    if (!win || win.closed || !real) return
    const i = stage.beatAt()
    const b = beats[i]
    if (!b) return
    const length = real(b.end) - real(b.start)
    const done = real(Math.min(Math.max(stage.t, b.start), b.end)) - real(b.start)
    if (i !== shown) {
      shown = i
      q('.n').textContent = `${i + 1} / ${beats.length}`
      q('.title').textContent = b.title
      q('.moment').hidden = !b.moment
      q('.caption').textContent = b.caption
      const notes = q('.notes')
      notes.textContent = b.notes ?? 'No notes for this step.'
      notes.classList.toggle('empty', !b.notes)
      const next = beats[i + 1]
      q('.next').innerHTML = next ? '<div class="label">Next</div><b></b><div></div>' : '<div class="label">Next</div>The end.'
      if (next) {
        q('.next b').textContent = `${i + 2}. ${next.title}`
        q('.next div:last-child').textContent = next.caption
      }
      win.document.title = `${i + 1}. ${b.title} · Notes`
    }
    q('.bar i').style.transform = `scaleX(${length > 0 ? done / length : 1})`
    q('.left').textContent = `${fmt(Math.max(0, length - done))} left in this step`
    q('.total').textContent = `${fmt(real(stage.duration) - real(stage.t))} left in all`
    if (playing !== stage.isPlaying) {
      playing = stage.isPlaying
      q('[data-do="toggle"]').textContent = playing ? 'Pause' : 'Play'
    }
  }

  const act = (what: string) => {
    root.dataset.started = ''
    if (what === 'toggle') stage.toggle()
    else if (what === 'next') stage.next()
    else if (what === 'prev') stage.prev()
    else if (what === 'home') {
      stage.pause()
      stage.seek(0)
    }
  }

  const open = () => {
    if (win && !win.closed) {
      win.focus()
      return
    }
    win = window.open('', 'wt-notes', `popup,width=${WIDTH},height=${HEIGHT}`)
    if (!win) return
    real ??= realClock(stage)
    const doc = win.document
    const css = getComputedStyle(root)
    const token = (n: string) => css.getPropertyValue(n).trim()
    const light = document.documentElement.dataset.theme === 'light'
    doc.documentElement.style.cssText = [
      ['--bg', token('--wt-bg')],
      ['--text', token('--wt-text')],
      ['--muted', token('--wt-muted')],
      ['--faint', token('--wt-faint')],
      ['--line', token('--wt-line-strong')],
      ['--run', token('--wt-run')],
      ['--moment', token('--wt-moment')],
      // Gold is light in both themes, so the badge's text is the darker of the two ends.
      ['--on-moment', token(light ? '--wt-text' : '--wt-bg')],
      ['--font', css.fontFamily],
      ['color-scheme', light ? 'light' : 'dark'],
    ]
      .map(([k, v]) => `${k}: ${v}`)
      .join(';')
    doc.head.innerHTML = '<meta charset="utf-8"><style></style>'
    doc.head.querySelector('style')!.textContent = fontFaces() + STYLE
    doc.body.innerHTML = BODY
    doc.body.addEventListener('click', (e) => {
      const what = (e.target as HTMLElement).closest<HTMLElement>('[data-do]')?.dataset.do
      if (what) act(what)
    })
    doc.addEventListener('keydown', (e) => {
      if (e.altKey || e.ctrlKey || e.metaKey) return
      const what = { ' ': 'toggle', k: 'toggle', ArrowRight: 'next', ArrowLeft: 'prev', Home: 'home' }[e.key]
      if (!what) return
      e.preventDefault()
      act(what)
    })
    shown = -1
    playing = null
    update()
  }

  stage.onUpdate(update)
  root.addEventListener('keydown', (e) => {
    if (e.key !== 'n' || e.altKey || e.ctrlKey || e.metaKey) return
    e.preventDefault()
    open()
  })
  addEventListener('pagehide', () => win?.close())
}
