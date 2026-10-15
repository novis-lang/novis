/**
 * A replay's frames, worked out when the website is built, and its HTML.
 *
 * A `.replay` beside a snippet lists the steps of an editor session, and the
 * `.replay.out` beside it is what the language server returned for them.
 * `tools/nv/cmd/site.ts` § *A snippet* is the home of both formats, and
 * `bun nv site` reads a `.replay` with `parseReplay` below, so the page and the
 * check read it the same way. `replayData` turns the two files into one frame
 * per keystroke, `replayMarkup` writes the HTML `src/components/Replay.astro`
 * shows, and `replay-player.mjs` plays it. The code's colours are passed in,
 * because `replay-colours.mjs` makes them with the website's packages.
 *
 * What is recorded: each completion list and each diagnostic, and where each
 * list opens, which is where the recording says the server was asked. What is
 * imitated: the narrowing of an open list as letters are typed
 * (`replay-narrow.mjs`), the typing pace, and the look of the editor. It is
 * close to VS Code and is not its exact behaviour.
 *
 * Nothing here imports a package, so `tools/nv/test/` runs it without the
 * website's packages.
 */

import { narrow } from './replay-narrow.mjs'

/** Where the cursor marker is in a `.replay`'s start text. */
export const CURSOR = '<|>'

/** `text`, a `.replay`, parsed as `{ start, cursor, steps }`, or why it is not one. */
export function parseReplay(text) {
  const lines = text.replace(/\r\n?/g, '\n').split('\n')
  if (lines[0] !== '--START--') return 'the first line is not `--START--`'
  const stepsAt = lines.indexOf('--STEPS--')
  if (stepsAt < 0) return 'no `--STEPS--` line'
  const marked = `${lines.slice(1, stepsAt).join('\n')}\n`
  const cursor = marked.indexOf(CURSOR)
  if (cursor < 0 || marked.indexOf(CURSOR, cursor + 1) >= 0) return `the start text has \`${CURSOR}\` ${cursor < 0 ? 'nowhere' : 'more than once'}`
  const steps = []
  for (const [i, line] of lines.slice(stepsAt + 1).entries()) {
    if (line.trim() === '') continue
    const where = `line ${stepsAt + 2 + i}`
    const [word, rest] = [line.split(' ', 1)[0], line.slice(line.indexOf(' ') + 1)]
    if (word === 'hover' && line === 'hover') steps.push({ kind: 'hover' })
    else if (word === 'accept' && line.includes(' ') && rest !== '') steps.push({ kind: 'accept', label: rest })
    else if (word === 'type' && line.includes(' ') && rest !== '') {
      const typed = unescape(rest)
      if (typed === null) return `${where}: only \`\\n\` and \`\\\\\` are escapes`
      steps.push({ kind: 'type', text: typed })
    } else return `${where}: \`${line}\` is not \`type TEXT\`, \`accept LABEL\` or \`hover\``
  }
  if (steps.length === 0) return 'no steps'
  return { start: marked.slice(0, cursor) + marked.slice(cursor + CURSOR.length), cursor, steps }
}

/** A `type` step's text with its escapes read, or null for an escape that is not one. */
function unescape(s) {
  let out = ''
  for (let i = 0; i < s.length; i++) {
    if (s[i] !== '\\') out += s[i]
    else if (s[i + 1] === 'n') (out += '\n'), i++
    else if (s[i + 1] === '\\') (out += '\\'), i++
    else return null
  }
  return out
}

/**
 * The answers a `.replay.out` records, in order: `{ kind: 'completion', step,
 * line, col, rows }` with each row `{ label, kind, right, after, insert }`, and
 * `{ kind: 'diagnostic', step, line, col, length, code, message }`. Steps,
 * lines and columns count from 0. A line that is not part of the format throws.
 */
export function parseRecording(text) {
  const asks = []
  for (const line of text.replace(/\r\n?/g, '\n').split('\n')) {
    const last = asks.at(-1)
    let m
    if ((m = /^completion (\d+) (\d+):(\d+)$/.exec(line))) {
      asks.push({ kind: 'completion', step: +m[1] - 1, line: +m[2] - 1, col: +m[3] - 1, rows: [] })
    } else if ((m = /^diagnostic (\d+) (\d+):(\d+)\+(\d+) (\S+)$/.exec(line))) {
      asks.push({ kind: 'diagnostic', step: +m[1] - 1, line: +m[2] - 1, col: +m[3] - 1, length: +m[4], code: m[5] === '-' ? '' : m[5], message: '' })
    } else if (line === '') {
      continue
    } else if (last?.kind === 'completion') {
      const [label, kind = '-', right = '', after = '', insert = ''] = line.split('\t')
      last.rows.push({ label, kind, right, after, insert: insert || label })
    } else if (last?.kind === 'diagnostic' && line.startsWith('  ')) {
      last.message += (last.message ? '\n' : '') + line.slice(2)
    } else {
      throw new Error(`\`${line}\` is not a line of a recording`)
    }
  }
  return asks
}

const nameChar = (c) => c !== undefined && /[A-Za-z0-9_]/.test(c)

/** The line and the column, both from 0, of `offset` in `text`. */
export function position(text, offset) {
  const before = text.slice(0, offset).split('\n')
  return { line: before.length - 1, col: before[before.length - 1].length }
}

// How long a frame stays before the next one, in milliseconds. The variation
// is fixed, so a replay plays the same way every time.
const TYPE_MS = (i) => 75 + ((i * 37) % 70)
const LIST_OPENS_MS = 450
const BEFORE_ACCEPT_MS = 900
const ACCEPT_MS = 650
const HOVER_MS = 3200
const FIRST_MS = 700

/**
 * A replay's frames, from `replay` (what `parseReplay` returns) and `asks`
 * (what `parseRecording` returns). The result is `{ start, asks, frames,
 * poster, steps }`:
 *
 * - each frame is `{ edit, cursor, list, hover, delay }`: the change since the
 *   frame before as `[offset, deleted, inserted]` or null, the cursor's offset,
 *   the open list as `{ ask, from }` (its answer and where its typed text
 *   starts) or null, the diagnostic shown as an index into `asks` or null, and
 *   how long the frame stays;
 * - `poster` is the frame shown when the page asks for no motion: the last one
 *   with a list open, or the last frame;
 * - `steps` is one sentence group per step, for the text under the replay.
 *
 * A step the recording does not fit throws, naming `where`.
 */
export function replayData(replay, asks, where) {
  let text = replay.start
  let cursor = replay.cursor
  let next = 0
  let list = null
  let typedCount = 0
  const frames = [{ edit: null, cursor, list: null, hover: null, delay: FIRST_MS }]
  const steps = []
  const fail = (step, why) => {
    throw new Error(`${where}: step ${step + 1}: ${why}; run \`bun nv site --bless\` on the snippet`)
  }
  const kept = (l) => narrow(asks[l.ask].rows.map((r) => r.label), text.slice(l.from, cursor))
  for (const [step, s] of replay.steps.entries()) {
    if (s.kind === 'type') {
      let opened = false
      for (const c of s.text) {
        text = text.slice(0, cursor) + c + text.slice(cursor)
        const at = cursor
        cursor += c.length
        const ask = asks[next]
        const here = position(text, cursor)
        let delay = TYPE_MS(typedCount++)
        if (ask?.kind === 'completion' && ask.step === step && ask.line === here.line && ask.col === here.col) {
          // A letter opens a list at the start of its word, and a trigger character right after itself.
          list = { ask: next++, from: /[A-Za-z_]/.test(c) && !nameChar(text[cursor - 2]) ? cursor - 1 : cursor }
          opened = true
          delay = LIST_OPENS_MS
        } else if (!(list !== null && nameChar(c))) list = null
        frames.push({ edit: [at, 0, c], cursor, list, hover: null, delay })
      }
      const words = s.text.split('\n').map((part) => (part === '' ? '' : `Type \`${part}\`.`))
      let said = words.join(' Press Enter. ').trim()
      if (opened) said += ' A list of suggestions opens.'
      if (list !== null) said += ` It shows ${names(kept(list).map((r) => asks[list.ask].rows[r.index].label))}.`
      steps.push(said)
    } else if (s.kind === 'accept') {
      if (list === null) fail(step, `\`accept ${s.label}\` with no list open`)
      const first = kept(list)[0]
      const row = first === undefined ? undefined : asks[list.ask].rows[first.index]
      if (row?.label !== s.label) fail(step, `the list selects ${row ? `\`${row.label}\`` : 'nothing'}, not \`${s.label}\``)
      frames.at(-1).delay = Math.max(frames.at(-1).delay, BEFORE_ACCEPT_MS)
      frames.push({ edit: [list.from, cursor - list.from, row.insert], cursor: list.from + row.insert.length, list: null, hover: null, delay: ACCEPT_MS })
      text = text.slice(0, list.from) + row.insert + text.slice(cursor)
      cursor = list.from + row.insert.length
      list = null
      steps.push(`Press Enter to choose \`${row.label}\`.${row.insert === row.label ? '' : ` The editor writes \`${row.insert}\`.`}`)
    } else {
      const ask = asks[next]
      if (ask?.kind !== 'diagnostic' || ask.step !== step) fail(step, 'the recording has no diagnostic for this `hover`')
      frames.push({ edit: null, cursor, list, hover: next++, delay: HOVER_MS })
      steps.push(`Point at the red underline. The error says: ${ask.message}${ask.code ? ` The error code is \`${ask.code}\`.` : ''}`)
    }
  }
  if (next !== asks.length) fail(asks[next].step, 'the recording has an answer the steps do not ask for')
  const listed = frames.findLastIndex((f) => f.list !== null)
  return { start: replay.start, asks, frames, poster: listed < 0 ? frames.length - 1 : listed, steps, last: text }
}

/** Up to three labels as a short list, and how many more there are. */
function names(labels) {
  const shown = labels.slice(0, 3).map((l) => `\`${l}\``)
  const more = labels.length - shown.length
  if (labels.length === 0) return 'no suggestions'
  if (more > 0) return `${shown.join(', ')} and ${more} more`
  return shown.length === 1 ? shown[0] : `${shown.slice(0, -1).join(', ')} and ${shown.at(-1)}`
}

export const escapeHtml = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')

/** `text` with each part in backticks as `<code>`, and the rest escaped. */
const withCode = (text) => text.split('`').map((part, i) => (i % 2 ? `<code>${escapeHtml(part)}</code>` : escapeHtml(part))).join('')

/**
 * The code area's lines as HTML: a line number and the text of each line, the
 * text cursor at offset `cursor` when it is not null, and a red underline over
 * `squiggle`, `{ line, col, length }`, when it is not null.
 *
 * `colours` is null for plain text, or `{ styles, lines }`: `lines` has the
 * runs of each line as `[length, style]`, and `style` is an index into
 * `styles`, each the `style` attribute of a token's span. `replayPaint` writes
 * both.
 */
export function codeHtml(text, cursor, squiggle, colours = null) {
  const lines = text.split('\n')
  const at = cursor === null ? null : position(text, cursor)
  if (lines.length > 1 && lines.at(-1) === '' && at?.line !== lines.length - 1) lines.pop()
  return lines
    .map((line, n) => {
      const caret = at?.line === n ? at.col : -1
      const wavy = squiggle?.line === n ? [squiggle.col, squiggle.col + squiggle.length] : null
      const styleAt = []
      for (const [length, style] of colours?.lines[n] ?? []) styleAt.push(...Array(length).fill(colours.styles[style] || null))
      // A token's span is closed before the cursor or an underline starts or ends, so the spans nest.
      let html = ''
      let open = null
      const shut = () => {
        if (open !== null) html += '</span>'
        open = null
      }
      for (let i = 0; i <= line.length; i++) {
        if (wavy && i === wavy[1]) shut(), (html += '</span>')
        if (i === caret) shut(), (html += '<span class="nv-caret"></span>')
        if (wavy && i === wavy[0]) shut(), (html += '<span class="nv-squiggle">')
        if (i === line.length) break
        const style = styleAt[i] ?? null
        if (style !== open) {
          shut()
          if (style !== null) html += `<span style="${escapeHtml(style)}">`
          open = style
        }
        html += escapeHtml(line[i])
      }
      shut()
      return `<div class="nv-line"><span class="nv-ln">${n + 1}</span><span class="nv-text">${html}</span></div>`
    })
    .join('')
}

/** A replay's `.replay` and `.replay.out` as `replayData` returns them. `where` names the snippet in an error. */
export function replayOf({ replay, recording, where }) {
  const parsed = parseReplay(replay)
  if (typeof parsed === 'string') throw new Error(`${where}: the .replay is not valid: ${parsed}`)
  return replayData(parsed, parseRecording(recording), where)
}

/** The code's text in each frame of `data`, which `replayData` returned. */
export function frameTexts(data) {
  let text = data.start
  return data.frames.map(({ edit }) => {
    if (edit) text = text.slice(0, edit[0]) + edit[2] + text.slice(edit[0] + edit[1])
    return text
  })
}

/**
 * The colours of each frame's code, from `colours(text)`, which gives each line
 * of a text as runs `[text, style]`. The result is `{ styles, lines, frames }`:
 * each distinct style and each distinct line once, and per frame the index of
 * each of its lines in `lines`. A line whose runs are not its text throws.
 */
export function replayPaint(texts, colours, where) {
  const styles = ['']
  const lines = []
  const seen = new Map()
  const index = (list, map, value) => {
    const key = JSON.stringify(value)
    if (!map.has(key)) map.set(key, list.push(value) - 1)
    return map.get(key)
  }
  const styleIds = new Map([['""', 0]])
  const frames = texts.map((text) => {
    const coloured = colours(text)
    return text.split('\n').map((line, n) => {
      const runs = coloured[n] ?? []
      if (runs.map(([t]) => t).join('') !== line && !(line === '' && n === coloured.length)) {
        throw new Error(`${where}: the colours of line ${n + 1} are not its text \`${line}\`: ${JSON.stringify(runs)}`)
      }
      return index(lines, seen, runs.map(([t, style]) => [t.length, index(styles, styleIds, style)]))
    })
  })
  return { styles, lines, frames }
}

/** The colours of frame `index`, as `codeHtml` reads them, from what `replayPaint` returned. */
export const frameColours = (paint, index) => (paint ? { styles: paint.styles, lines: paint.frames[index].map((i) => paint.lines[i]) } : null)

/**
 * The HTML of one replay: the caption, the editor frame, the buttons, the
 * steps as text, and the frames as JSON for `replay-player.mjs`. `title` is the
 * tab's title, `install` the address of the editor install page, and `where`
 * names the snippet in an error. `colours`, when it is given, colours the code
 * as `replayPaint` describes, and the code is plain text without it.
 */
export function replayMarkup({ replay, recording, title, install, where, colours = null }) {
  const data = replayOf({ replay, recording, where })
  const texts = frameTexts(data)
  // The code area is as tall as the longest frame, so it does not grow while the replay plays.
  const rows = Math.max(...texts.map((text) => text.split('\n').length))
  const paint = colours ? replayPaint(texts, colours, where) : null
  const json = JSON.stringify({ start: data.start, asks: data.asks, frames: data.frames, poster: data.poster, paint }).replace(/</g, '\\u003c')
  return [
    '<figure class="nv-replay" data-replay>',
    `<figcaption class="nv-replay-caption">A recording from the <a href="${escapeHtml(install)}">Novis editor extension</a>. You cannot type here.</figcaption>`,
    '<div class="nv-replay-frame" inert aria-hidden="true">',
    `<div class="nv-replay-bar"><span class="nv-replay-tab">${escapeHtml(title)}</span><span class="nv-replay-badge">Recording</span></div>`,
    `<div class="nv-replay-code" style="--nv-rows: ${rows}">${codeHtml(data.last, null, null, frameColours(paint, texts.length - 1))}</div>`,
    '</div>',
    '<div class="nv-replay-controls">',
    '<button type="button" data-replay-do="previous">Previous</button>',
    '<button type="button" data-replay-do="play">Play</button>',
    '<button type="button" data-replay-do="next">Next</button>',
    '</div>',
    `<details class="nv-replay-steps"><summary>What this recording shows</summary><ol>${data.steps.map((s) => `<li>${withCode(s)}</li>`).join('')}</ol></details>`,
    `<script type="application/json" data-replay-data>${json}</script>`,
    '</figure>',
  ].join('')
}
