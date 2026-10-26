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
 * list opens, which is where the recording says the server was asked; and each
 * rename, code action, code lens, call hierarchy and hover card, with the
 * edits a rename or an action makes. What is
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

/** The steps that are one word with nothing after it. */
const BARE = ['hover', 'card', 'lens', 'calls']

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
    const argued = line.includes(' ') && rest !== ''
    if (BARE.includes(word) && line === word) steps.push({ kind: word })
    else if (word === 'accept' && argued) steps.push({ kind: 'accept', label: rest })
    else if (word === 'action' && argued) steps.push({ kind: 'action', title: rest })
    else if (word === 'rename' && argued && !/\s/.test(rest)) steps.push({ kind: 'rename', name: rest })
    else if (word === 'type' && argued) {
      const typed = unescape(rest)
      if (typed === null) return `${where}: only \`\\n\` and \`\\\\\` are escapes`
      steps.push({ kind: 'type', text: typed })
    } else return `${where}: \`${line}\` is not a step: \`type TEXT\`, \`accept LABEL\`, \`rename NAME\`, \`action TITLE\`, ${BARE.map((b) => `\`${b}\``).join(', ')}`
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
 * The answers a `.replay.out` records, in order, each with its `step`. Steps,
 * lines and columns count from 0.
 *
 * - `{ kind: 'completion', line, col, rows }`, each row `{ label, kind, right, after, insert }`
 * - `{ kind: 'diagnostic', line, col, length, code, message }`
 * - `{ kind: 'card', line, col, markdown }`
 * - `{ kind: 'rename', line, col, others, edits }`
 * - `{ kind: 'action', line, col, titles, edits }`
 * - `{ kind: 'lens', lenses }`, each lens `{ line, col, title }`
 * - `{ kind: 'calls', line, col, name, incoming, outgoing }`, each call `{ name, count }`
 *
 * Each edit is `[line, col, endLine, endCol, text]`. A line that is not part of
 * the format throws.
 */
export function parseRecording(text) {
  const asks = []
  // A blank line inside a card's text, kept until an indented line shows the card goes on.
  let gap = false
  for (const line of text.replace(/\r\n?/g, '\n').split('\n')) {
    const last = asks.at(-1)
    const at = (l, c) => ({ line: +l - 1, col: +c - 1 })
    let m
    if (line === '') {
      gap = true
      continue
    }
    const blank = gap
    gap = false
    if ((m = /^completion (\d+) (\d+):(\d+)$/.exec(line))) {
      asks.push({ kind: 'completion', step: +m[1] - 1, ...at(m[2], m[3]), rows: [] })
    } else if ((m = /^diagnostic (\d+) (\d+):(\d+)\+(\d+) (\S+)$/.exec(line))) {
      asks.push({ kind: 'diagnostic', step: +m[1] - 1, ...at(m[2], m[3]), length: +m[4], code: m[5] === '-' ? '' : m[5], message: '' })
    } else if ((m = /^card (\d+) (\d+):(\d+)$/.exec(line))) {
      asks.push({ kind: 'card', step: +m[1] - 1, ...at(m[2], m[3]), markdown: '' })
    } else if ((m = /^rename (\d+) (\d+):(\d+) (\d+)$/.exec(line))) {
      asks.push({ kind: 'rename', step: +m[1] - 1, ...at(m[2], m[3]), others: +m[4], edits: [] })
    } else if ((m = /^action (\d+) (\d+):(\d+)$/.exec(line))) {
      asks.push({ kind: 'action', step: +m[1] - 1, ...at(m[2], m[3]), titles: [], edits: [] })
    } else if ((m = /^lens (\d+)$/.exec(line))) {
      asks.push({ kind: 'lens', step: +m[1] - 1, lenses: [] })
    } else if ((m = /^calls (\d+) (\d+):(\d+) (\S+)$/.exec(line))) {
      asks.push({ kind: 'calls', step: +m[1] - 1, ...at(m[2], m[3]), name: m[4], incoming: [], outgoing: [] })
    } else if (last?.kind === 'completion') {
      const [label, kind = '-', right = '', after = '', insert = ''] = line.split('\t')
      last.rows.push({ label, kind, right, after, insert: insert || label })
    } else if (last?.kind === 'diagnostic' && line.startsWith('  ')) {
      last.message += (last.message ? '\n' : '') + line.slice(2)
    } else if (last?.kind === 'card' && line.startsWith('  ')) {
      last.markdown += (last.markdown ? (blank ? '\n\n' : '\n') : '') + line.slice(2)
    } else if ((last?.kind === 'rename' || last?.kind === 'action') && (m = /^edit (\d+):(\d+)-(\d+):(\d+) (".*")$/.exec(line))) {
      last.edits.push([+m[1] - 1, +m[2] - 1, +m[3] - 1, +m[4] - 1, JSON.parse(m[5])])
    } else if (last?.kind === 'action' && line.startsWith('  ')) {
      last.titles.push(line.slice(2))
    } else if (last?.kind === 'lens' && (m = /^ {2}(\d+):(\d+) (.*)$/.exec(line))) {
      last.lenses.push({ ...at(m[1], m[2]), title: m[3] })
    } else if (last?.kind === 'calls' && (m = /^ {2}(in|out) (\S+) (\d+)$/.exec(line))) {
      ;(m[1] === 'in' ? last.incoming : last.outgoing).push({ name: m[2], count: +m[3] })
    } else {
      throw new Error(`\`${line}\` is not a line of a recording`)
    }
  }
  return asks
}

/** The offset in `text` of line `line` and column `col`, both from 0. */
export function offsetAt(text, line, col) {
  let at = 0
  for (let n = 0; n < line; n++) {
    const end = text.indexOf('\n', at)
    if (end < 0) return text.length
    at = end + 1
  }
  return Math.min(at + col, text.length)
}

/**
 * `text` with `edits` made, each `[line, col, endLine, endCol, text]` in the
 * text before any of them, and where the cursor moves. A cursor inside a
 * replaced range stays as far into the new text as it was, at most its end.
 * The result is `{ text, cursor, edit }`, and `edit` is the whole change as
 * one `[offset, deleted, inserted]`.
 */
export function applyEdits(text, cursor, edits) {
  const spans = edits.map(([l, c, el, ec, t]) => ({ from: offsetAt(text, l, c), to: offsetAt(text, el, ec), t })).sort((a, b) => b.from - a.from)
  let out = text
  let moved = cursor
  for (const { from, to, t } of spans) {
    out = out.slice(0, from) + t + out.slice(to)
    if (to <= cursor) moved += t.length - (to - from)
    else if (from <= cursor) moved += from - cursor + Math.min(cursor - from, t.length)
  }
  let head = 0
  while (head < text.length && head < out.length && text[head] === out[head]) head++
  let tail = 0
  while (tail < text.length - head && tail < out.length - head && text[text.length - 1 - tail] === out[out.length - 1 - tail]) tail++
  return { text: out, cursor: moved, edit: [head, text.length - head - tail, out.slice(head, out.length - tail)] }
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
// A list that opens stays long enough to read its rows, and each letter that narrows it stays long
// enough to see which rows are left.
const LIST_OPENS_MS = 1500
const NARROW_MS = 600
const BEFORE_ACCEPT_MS = 900
const ACCEPT_MS = 650
const HOVER_MS = 3200
const RENAME_MS = 1600
const MENU_MS = 1800
const EDITED_MS = 1800
const FIRST_MS = 700

/**
 * A replay's frames, from `replay` (what `parseReplay` returns) and `asks`
 * (what `parseRecording` returns). The result is `{ start, asks, frames,
 * poster, steps }`:
 *
 * - each frame is `{ edit, cursor, list, hover, panel, delay }`: the change
 *   since the frame before as `[offset, deleted, inserted]` or null, the
 *   cursor's offset, the open list as `{ ask, from }` (its answer and where its
 *   typed text starts) or null, the diagnostic shown as an index into `asks` or
 *   null, the panel `panelHtml` draws or null, and how long the frame stays;
 * - `poster` is the frame shown before the replay plays: the last one with a
 *   list open, else the last one with a card or a panel, else the last frame;
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
  const frames = [{ edit: null, cursor, list: null, hover: null, panel: null, delay: FIRST_MS }]
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
        } else if (list !== null && nameChar(c)) {
          if (kept(list).length > 0) delay = NARROW_MS
        } else list = null
        frames.push({ edit: [at, 0, c], cursor, list, hover: null, panel: null, delay })
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
      frames.push({ edit: [list.from, cursor - list.from, row.insert], cursor: list.from + row.insert.length, list: null, hover: null, panel: null, delay: ACCEPT_MS })
      text = text.slice(0, list.from) + row.insert + text.slice(cursor)
      cursor = list.from + row.insert.length
      list = null
      steps.push(`Press Enter to choose \`${row.label}\`.${row.insert === row.label ? '' : ` The editor writes \`${row.insert}\`.`}`)
    } else if (s.kind === 'hover') {
      const ask = asks[next]
      if (ask?.kind !== 'diagnostic' || ask.step !== step) fail(step, 'the recording has no diagnostic for this `hover`')
      frames.push({ edit: null, cursor, list, hover: next++, panel: null, delay: HOVER_MS })
      steps.push(`Point at the red underline. The error says: ${ask.message}${ask.code ? ` The error code is \`${ask.code}\`.` : ''}`)
    } else {
      const index = next++
      const ask = asks[index]
      if (ask?.kind !== s.kind || ask.step !== step) fail(step, `the recording has no answer for this \`${s.kind}\``)
      list = null
      const here = position(text, cursor)
      const panel = { ask: index, line: here.line, col: here.col }
      if (s.kind === 'rename' || s.kind === 'action') {
        // The step's menu or input box, then the code after the edit.
        if (s.kind === 'action' && !ask.titles.includes(s.title)) fail(step, `the code actions are ${names(ask.titles, true)}, not "${s.title}"`)
        const own = ask.edits.find(([l, c, el, ec]) => (l < here.line || (l === here.line && c <= here.col)) && (el > here.line || (el === here.line && ec >= here.col)))
        if (s.kind === 'rename' && own) [panel.line, panel.col] = own
        const old = own ? text.slice(offsetAt(text, own[0], own[1]), offsetAt(text, own[2], own[3])) : ''
        frames.push({ edit: null, cursor, list: null, hover: null, panel: { ...panel, name: s.name, chosen: s.title }, delay: s.kind === 'rename' ? RENAME_MS : MENU_MS })
        const made = applyEdits(text, cursor, ask.edits)
        text = made.text
        cursor = made.cursor
        const after = s.kind === 'rename' && ask.others > 0 ? { ...panel, done: true } : null
        frames.push({ edit: made.edit, cursor, list: null, hover: null, panel: after, delay: EDITED_MS })
        if (s.kind === 'rename') {
          const where = ask.others > 0 ? ` and in ${count(ask.others, 'other file')}` : ''
          steps.push(`Press F2 and type \`${s.name}\`. The editor renames \`${old}\` in ${count(ask.edits.length, 'place')} in this file${where}.`)
        } else {
          steps.push(`Press Ctrl+. to show the code actions here. The menu has ${names(ask.titles, true)}. Choose "${s.title}", and the editor changes the code.`)
        }
      } else {
        frames.push({ edit: null, cursor, list: null, hover: null, panel, delay: HOVER_MS })
        if (s.kind === 'lens') {
          steps.push(`The editor shows ${count(ask.lenses.length, 'code lens', 'code lenses')}, short notes above the code: ${names(ask.lenses.map((l) => l.title), true)}.`)
        } else if (s.kind === 'calls') {
          const called = ask.incoming.length ? `It is called by ${names(ask.incoming.map((c) => c.name))}.` : 'Nothing calls it.'
          const calls = ask.outgoing.length ? `It calls ${names(ask.outgoing.map((c) => c.name))}.` : 'It calls no other method.'
          steps.push(`Open the call hierarchy of \`${ask.name}\`. ${called} ${calls}`)
        } else {
          const signature = /^```\w*\n(.*)/.exec(ask.markdown)?.[1]
          steps.push(`Point at the code. A card shows ${signature ? `\`${signature}\` and ` : ''}its documentation.`)
        }
      }
    }
  }
  if (next !== asks.length) fail(asks[next].step, 'the recording has an answer the steps do not ask for')
  const listed = frames.findLastIndex((f) => f.list !== null)
  const shown = frames.findLastIndex((f) => f.panel !== null || f.hover !== null)
  return { start: replay.start, asks, frames, poster: listed >= 0 ? listed : shown >= 0 ? shown : frames.length - 1, steps, last: text }
}

/** `n` and the noun, as `1 place` or `3 places`. */
const count = (n, one, many = `${one}s`) => `${n} ${n === 1 ? one : many}`

/** Up to three labels as a short list, and how many more there are. Each is code, or in quotes with
 * `quote`, for a title that may have code inside it. */
function names(labels, quote = false) {
  const shown = labels.slice(0, 3).map((l) => (quote ? `"${l}"` : `\`${l}\``))
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
 *
 * `lenses`, when it is given, is the code lenses `{ line, title }` to show,
 * each above its line with that line's indent.
 */
export function codeHtml(text, cursor, squiggle, colours = null, lenses = []) {
  const lines = text.split('\n')
  const at = cursor === null ? null : position(text, cursor)
  if (lines.length > 1 && lines.at(-1) === '' && at?.line !== lines.length - 1) lines.pop()
  return lines
    .map((line, n) => {
      const titles = lenses.filter((l) => l.line === n).map((l) => escapeHtml(l.title))
      const indent = /^\s*/.exec(line)[0]
      const lens = titles.length ? `<div class="nv-line nv-lens"><span class="nv-ln"></span><span class="nv-text">${indent}${titles.join(' | ')}</span></div>` : ''
      return lens + lineHtml(line, n)
    })
    .join('')

  function lineHtml(line, n) {
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
  }
}

/** The lenses `codeHtml` shows in a frame with `panel`: those of a `lens` answer, and none otherwise. */
export const frameLenses = (asks, panel) => (panel && asks[panel.ask].kind === 'lens' ? asks[panel.ask].lenses : [])

/**
 * The HTML of a frame's panel, `{ ask, line, col, ... }`, over the answer
 * `ask`: the rename box and then how many other files changed, the menu of
 * code actions with the chosen one selected, the call hierarchy, or the hover
 * card. A lens has no panel of its own, because `codeHtml` shows it.
 */
export function panelHtml(ask, panel) {
  const box = (cls, body) => `<div class="nv-hover ${cls}" style="--nv-line: ${panel.line + 1}; --nv-col: ${panel.col}">${body}</div>`
  if (ask.kind === 'rename') {
    if (panel.done) return box('nv-renamed', escapeHtml(`Also renamed in ${count(ask.others, 'other file')}.`))
    return box('nv-rename', `<span class="nv-rename-input">${escapeHtml(panel.name)}</span>`)
  }
  if (ask.kind === 'action') {
    const rows = ask.titles.map((t) => `<li class="nv-row${t === panel.chosen ? ' nv-selected' : ''}"><span class="nv-label">${escapeHtml(t)}</span></li>`)
    return `<ul class="nv-list nv-actions" style="--nv-line: ${panel.line + 1}; --nv-col: ${panel.col}">${rows.join('')}</ul>`
  }
  if (ask.kind === 'calls') {
    const group = (title, calls) =>
      `<div class="nv-calls-title">${title}</div>` +
      (calls.length ? calls.map((c) => `<div class="nv-calls-row"><code>${escapeHtml(c.name)}</code>${c.count > 1 ? ` <span class="nv-hover-code">${count(c.count, 'call')}</span>` : ''}</div>`).join('') : '<div class="nv-hover-code">None</div>')
    return box('nv-calls', `<div class="nv-card-head"><code>${escapeHtml(ask.name)}</code></div>${group('Called by', ask.incoming)}${group('Calls', ask.outgoing)}`)
  }
  if (ask.kind === 'card') return box('nv-card', markdownHtml(ask.markdown))
  return ''
}

/**
 * A hover card's Markdown as HTML: code fences, `---` rules, `- ` lists and
 * paragraphs, with `` `code` `` and `**bold**` inside a line. This is all a card from the
 * server has.
 */
export function markdownHtml(markdown) {
  const out = []
  const lines = markdown.split('\n')
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]
    if (line.startsWith('```')) {
      const code = []
      while (++i < lines.length && !lines[i].startsWith('```')) code.push(lines[i])
      out.push(`<pre class="nv-card-code">${escapeHtml(code.join('\n'))}</pre>`)
    } else if (/^-{3,}$/.test(line.trim())) out.push('<hr>')
    else if (line.startsWith('- ')) {
      const items = [line]
      while (lines[i + 1]?.startsWith('- ')) items.push(lines[++i])
      out.push(`<ul>${items.map((l) => `<li>${inline(l.slice(2))}</li>`).join('')}</ul>`)
    } else if (line.trim() !== '') {
      const words = [line]
      while (lines[i + 1] !== undefined && lines[i + 1].trim() !== '' && !/^(```|- |-{3,}$)/.test(lines[i + 1])) words.push(lines[++i])
      out.push(`<p>${inline(words.join(' '))}</p>`)
    }
  }
  return out.join('')
}

/** One line of a card's Markdown as HTML: `` `code` `` and `**bold**`. */
const inline = (text) =>
  text
    .split('`')
    .map((part, i) => (i % 2 ? `<code>${escapeHtml(part)}</code>` : escapeHtml(part).replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')))
    .join('')

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

const svg = (body) => `<svg viewBox="0 0 24 24" width="1em" height="1em" aria-hidden="true" focusable="false">${body}</svg>`
const STROKE = 'fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"'

/** The replay's icons as inline SVG: the header's editor window, and the button's play, pause and play again. */
export const ICONS = {
  editor: svg(`<rect x="3" y="4" width="18" height="16" rx="2" ${STROKE}/><path d="M3 9h18M8 13l-2 2 2 2M12 17h4" ${STROKE}/>`),
  play: svg('<path d="M8 5.5v13l10.5-6.5z" fill="currentColor"/>'),
  pause: svg('<path d="M7 5h3.5v14H7zM13.5 5H17v14h-3.5z" fill="currentColor"/>'),
  again: svg(`<path d="M4 12a8 8 0 1 0 2.4-5.7M4 4.5v4h4" ${STROKE}/>`),
}

/** The button's label for a recording with an answer of each kind, the first that applies. */
const PANEL_LABELS = [
  ['rename', 'Preview the rename'],
  ['action', 'Preview the code action'],
  ['calls', 'Preview the call hierarchy'],
  ['lens', 'Preview the code lenses'],
  ['card', 'Preview the hover card'],
]

/** The button's label before the replay plays: what the recording shows, from what the server returned. */
export function playLabel(asks) {
  for (const [kind, label] of PANEL_LABELS) if (asks.some((a) => a.kind === kind)) return label
  if (asks.some((a) => a.kind === 'diagnostic')) return 'Preview the error message'
  if (asks.some((a) => a.kind === 'completion')) return 'Preview the autocompletion'
  return 'Play the recording'
}

/** The button's content: an icon and a label. */
export const buttonHtml = (icon, label) => `${ICONS[icon]}<span>${escapeHtml(label)}</span>`

/**
 * The HTML of one replay: one panel with a header and a notice, the editor
 * window, the play button, the steps as text, and the frames as JSON for
 * `replay-player.mjs`. `title` is the window's file name, `install` the
 * address of the editor install page, and `where` names the snippet in an
 * error. `colours`, when it is given, colours the code as `replayPaint`
 * describes, and the code is plain text without it.
 *
 * The root has Starlight's `not-content` class, so the page's text styles do
 * not add margins between the code's lines and the list stays on its line.
 */
export function replayMarkup({ replay, recording, title, install, where, colours = null }) {
  const data = replayOf({ replay, recording, where })
  const texts = frameTexts(data)
  // The code area is as tall as the longest frame, so it does not grow while the replay plays. Its font
  // gets smaller until the widest line, `cols`, fits, and stops at 12px. `Replay.astro`'s style does the
  // sums, and `replay-player.mjs` scrolls a line that is still too wide.
  // A frame with code lenses has one more row for each line they are above.
  const lensRows = (i) => new Set(frameLenses(data.asks, data.frames[i].panel).map((l) => l.line)).size
  const rows = Math.max(...texts.map((text, i) => text.split('\n').length + lensRows(i)))
  const cols = Math.max(...texts.flatMap((text) => text.split('\n').map((line) => line.length)))
  const paint = colours ? replayPaint(texts, colours, where) : null
  const label = playLabel(data.asks)
  const json = JSON.stringify({ start: data.start, asks: data.asks, frames: data.frames, poster: data.poster, paint }).replace(/</g, '\\u003c')
  return [
    '<figure class="nv-replay not-content" data-replay data-motion>',
    '<figcaption class="nv-replay-head">',
    `<span class="nv-replay-icon">${ICONS.editor}</span>`,
    '<span class="nv-replay-heading"><span class="nv-replay-title">How it looks in your editor</span>',
    `<span class="nv-replay-note">A recording from the <a href="${escapeHtml(install)}">Novis editor extension</a>. You cannot type here.</span></span>`,
    '</figcaption>',
    `<div class="nv-replay-frame" inert aria-hidden="true" style="--nv-cols: ${cols}">`,
    `<div class="nv-replay-bar"><span class="nv-replay-dots"><i></i><i></i><i></i></span><span class="nv-replay-tab">${escapeHtml(title)}</span></div>`,
    `<div class="nv-replay-code" style="--nv-rows: ${rows}">${codeHtml(data.last, null, null, frameColours(paint, texts.length - 1))}</div>`,
    '<div class="nv-replay-progress"><span></span></div>',
    '</div>',
    `<div class="nv-replay-controls"><button type="button" class="nv-button nv-button-primary nv-replay-button" data-replay-do="play" data-replay-label="${escapeHtml(label)}">${buttonHtml('play', label)}</button></div>`,
    `<details class="nv-replay-steps"><summary>What this recording shows</summary><ol>${data.steps.map((s) => `<li>${withCode(s)}</li>`).join('')}</ol></details>`,
    `<script type="application/json" data-replay-data>${json}</script>`,
    '</figure>',
  ].join('')
}
