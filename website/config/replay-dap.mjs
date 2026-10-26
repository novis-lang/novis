/**
 * A debugger replay's frames, worked out when the website is built.
 *
 * A `.dap` beside a snippet lists the breakpoints and the steps of a debug
 * session, and the `.dap.out` beside it is what `nvs dap` returned at each
 * stop. `tools/nv/cmd/site.ts` § *A snippet* is the home of both formats, and
 * `bun nv site` reads a `.dap` with `parseDap` below, so the page and the check
 * read it the same way. `dapData` turns the two files into one frame per stop,
 * in the shape `replay-frames.mjs`'s `replayData` returns, so the same markup
 * and the same player show it. The code never changes in a debugger replay:
 * each frame shows where the program stopped, and its panel shows the call
 * stack and the variables.
 *
 * Nothing here imports a package, so `tools/nv/test/` runs it without the
 * website's packages.
 */

/** A step of a `.dap`, and the request it sends: the header's § *A snippet*. */
export const DAP_STEPS = { continue: 'continue', next: 'next', in: 'stepIn', out: 'stepOut' }

/** `text`, a `.dap`, parsed as `{ breaks, steps }`, breakpoint lines counted from 1, or why it is not one. */
export function parseDap(text) {
  const breaks = []
  const steps = []
  for (const [i, raw] of text.replace(/\r\n?/g, '\n').split('\n').entries()) {
    const line = raw.trim()
    if (line === '') continue
    const at = /^break (\d+)$/.exec(line)
    if (at) {
      if (steps.length > 0) return `line ${i + 1}: a \`break\` after a step; list every breakpoint first`
      if (Number(at[1]) < 1) return `line ${i + 1}: lines count from 1`
      breaks.push(Number(at[1]))
    } else if (line in DAP_STEPS) steps.push(line)
    else return `line ${i + 1}: \`${line}\` is not \`break LINE\` or a step: ${Object.keys(DAP_STEPS).map((s) => `\`${s}\``).join(', ')}`
  }
  if (breaks.length === 0) return 'no `break` line'
  return { breaks, steps }
}

/**
 * The answers a `.dap.out` records, in order, each with its `step`: 0 for the
 * start of the session, and then each step's number from 1. Lines count from 1,
 * as they do in the file.
 *
 * - `{ kind: 'stop', step, reason, frames, variables }`, each frame `{ file, line, name }` and each
 *   variable `{ name, type, value }`
 * - `{ kind: 'end', step, exit }`
 *
 * A line that is not part of the format throws.
 */
export function parseDapRecording(text) {
  const asks = []
  for (const [i, line] of text.replace(/\r\n?/g, '\n').split('\n').entries()) {
    if (line.trim() === '') continue
    const bad = () => {
      throw new Error(`.dap.out line ${i + 1}: \`${line}\` is not part of the format`)
    }
    const stop = /^stop (\d+) (\S+)$/.exec(line)
    const end = /^end (\d+) (-?\d+)$/.exec(line)
    const last = asks.at(-1)
    if (stop) asks.push({ kind: 'stop', step: Number(stop[1]), reason: stop[2], frames: [], variables: [] })
    else if (end) asks.push({ kind: 'end', step: Number(end[1]), exit: Number(end[2]) })
    else if (line.startsWith('  frame ') && last?.kind === 'stop') {
      const frame = /^ {2}frame (.*):(\d+) (.*)$/.exec(line) ?? bad()
      last.frames.push({ file: frame[1], line: Number(frame[2]), name: frame[3] })
    } else if (line.startsWith('  var ') && last?.kind === 'stop') {
      const [name, type, value] = line.slice('  var '.length).split('\t')
      if (value === undefined) bad()
      last.variables.push({ name, type, value })
    } else bad()
  }
  return asks
}

const FIRST_MS = 900
const STOP_MS = 3600

/** The key a reader presses for each step in VS Code, and what the step does. */
const SAY = {
  continue: 'Press F5 to continue.',
  next: 'Press F10 to step over the line.',
  in: 'Press F11 to step into the call.',
  out: 'Press Shift+F11 to step out of the method.',
}

/** Up to `n` items as a short list in words, and how many more there are. */
function listed(items, n = 3) {
  const shown = items.slice(0, n)
  const more = items.length - shown.length
  if (more > 0) return `${shown.join(', ')} and ${more} more`
  return shown.length === 1 ? shown[0] : `${shown.slice(0, -1).join(', ')} and ${shown.at(-1)}`
}

/**
 * A debugger replay's frames, from `dap` (what `parseDap` returns), `asks`
 * (what `parseDapRecording` returns) and `text`, the snippet, whose file name is
 * `file`. The result has the fields `replayData` returns, and `breaks`, the
 * breakpoint lines counted from 0. Each frame after the first has a panel
 * `{ ask, line, col }` at the line where the program stopped, or after the last
 * line once it ended.
 *
 * A recording the steps do not fit, and a stop in another file, throw, naming `where`.
 */
export function dapData(dap, asks, text, file, where) {
  const fail = (step, why) => {
    throw new Error(`${where}: ${step === 0 ? 'the start' : `step ${step}`}: ${why}; run \`bun nv site --bless\` on the snippet`)
  }
  const lines = text.replace(/\n$/, '').split('\n')
  const frames = [{ edit: null, cursor: null, list: null, hover: null, panel: null, delay: FIRST_MS }]
  const steps = []
  const said = ['Set a breakpoint', dap.breaks.length === 1 ? ` on line ${dap.breaks[0]}` : `s on lines ${listed(dap.breaks.map(String), dap.breaks.length)}`, ' and start debugging.']
  let next = 0
  for (let step = 0; step <= dap.steps.length; step++) {
    const ask = asks[next]
    if (ask?.step !== step) fail(step, 'the recording has no answer for it')
    const lead = step === 0 ? said.join('') : SAY[dap.steps[step - 1]]
    if (ask.kind === 'end') {
      if (step < dap.steps.length) fail(step + 1, 'the program has already ended')
      frames.push({ edit: null, cursor: null, list: null, hover: null, panel: { ask: next++, line: lines.length - 1, col: 0 }, delay: STOP_MS })
      steps.push(`${lead} The program runs to its end. It ends with exit code \`${ask.exit}\`.`)
      break
    }
    const top = ask.frames[0]
    if (top === undefined) fail(step, 'the stop has no call stack')
    if (top.file !== file) fail(step, `the program stops in \`${top.file}\`, and the page shows only \`${file}\``)
    const indent = /^\s*/.exec(lines[top.line - 1] ?? '')[0].length
    frames.push({ edit: null, cursor: null, list: null, hover: null, panel: { ask: next++, line: top.line - 1, col: indent }, delay: STOP_MS })
    const values = ask.variables.map((v) => `\`${v.name}\` is \`${v.value}\``)
    const inside = ask.frames.length > 1 ? ` in \`${top.name}\`` : ''
    steps.push(`${lead} The program stops on line ${top.line}${inside}.${values.length ? ` ${listed(values)}.` : ''}`)
  }
  if (next !== asks.length) fail(asks[next].step, 'the recording has an answer the steps do not ask for')
  const stopped = frames.findLastIndex((f) => f.panel !== null && asks[f.panel.ask].kind === 'stop')
  return { start: text, asks, frames, poster: stopped >= 0 ? stopped : frames.length - 1, steps, last: text, breaks: dap.breaks.map((b) => b - 1) }
}

/** The rows a stop's panel takes, so the code area is tall enough for it. */
export const panelRows = (ask) => (ask.kind === 'stop' ? ask.frames.length + Math.max(1, ask.variables.length) + 3 : 2)
