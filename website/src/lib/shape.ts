/**
 * A shape type laid out the way `nvs fmt` lays out a long list: a type that
 * fits in `WIDTH` columns stays on one line, and one that does not opens across
 * lines. A union puts each case on its own line after a `|`, and a shape puts
 * each key on its own line, one level deeper, with a trailing comma. Every type
 * name links through `typeLink`, so the block reads like the method signatures
 * above it on the same page.
 */
import { TYPE_NAME, typeLink } from './core'

const WIDTH = 72
const INDENT = '    '

const escape = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
const span = (cls: string, s: string) => `<span class="${cls}">${escape(s)}</span>`

/** `text` split at `sep` where no bracket is open. */
function splitTopLevel(text: string, sep: string): string[] {
  const out: string[] = []
  let depth = 0
  let cur = ''
  for (const ch of text) {
    if ('<{('.includes(ch)) depth++
    if ('>})'.includes(ch)) depth--
    if (ch === sep && depth === 0) {
      out.push(cur)
      cur = ''
    } else {
      cur += ch
    }
  }
  out.push(cur)
  return out.map((p) => p.trim()).filter(Boolean)
}

/** The type on one line, its spacing normalised the way `nvs fmt` writes it: `{ a: int, b: ?string }`. */
const flat = (type: string) =>
  type
    .replace(/\s+/g, ' ')
    .replace(/\{ ?/g, '{ ')
    .replace(/,? ?\}/g, ' }')
    .replace(/\{ +\}/g, '{}')
    .replace(/\s*\|\s*/g, ' | ')
    .trim()

/** The lines of `type`, the first of which follows `lead` on the line it opens at `depth`. */
function layout(type: string, depth: number, lead: number): string[] {
  const one = flat(type)
  if (depth * INDENT.length + lead + one.length <= WIDTH) return [one]
  const cases = splitTopLevel(one, '|')
  if (cases.length > 1) {
    return cases.flatMap((c, i) => {
      const lines = layout(c, depth, i === 0 ? lead : 2)
      return i === 0 ? lines : [`| ${lines[0]}`, ...lines.slice(1)]
    })
  }
  // The first shape at the top level opens across lines; what is around it stays put.
  const open = one.indexOf('{')
  if (open < 0) return [one]
  let close = open
  for (let d = 0; close < one.length; close++) {
    if (one[close] === '{') d++
    if (one[close] === '}' && --d === 0) break
  }
  const keys = splitTopLevel(one.slice(open + 1, close), ',')
  const lines = [`${one.slice(0, open)}{`]
  for (const key of keys) {
    const colon = key.search(/\??:/)
    const name = colon < 0 ? key : key.slice(0, key.indexOf(':') + 1) + ' '
    const inner = colon < 0 ? [] : layout(key.slice(key.indexOf(':') + 1), depth + 1, name.length)
    const [first = '', ...rest] = colon < 0 ? [key] : inner.map((l, i) => (i === 0 ? name + l : l))
    lines.push(INDENT + first, ...rest.map((l) => INDENT + l))
    lines[lines.length - 1] += ','
  }
  lines.push(`}${one.slice(close + 1)}`)
  return lines
}

/** One line of type text with each type name linked and each shape key and punctuation marked. */
function lineHtml(line: string, owner: string): string {
  const indent = /^ */.exec(line)![0]
  const parts = line.slice(indent.length).split(TYPE_NAME)
  return (
    indent +
    parts
      .map((part, i) => {
        if (part === '') return ''
        if (i % 2 === 0) return span('sig-punct', part)
        // A name followed by `:` or `?:` is a key of a shape, not a type.
        if (/^\??:/.test(parts[i + 1] ?? '')) return span('sig-var', part)
        if (part === 'secret' || part === 'tainted') return span('sig-keyword', part)
        const href = typeLink(part, owner)
        return href ? `<a class="sig-type sig-link" href="${escape(href)}">${escape(part)}</a>` : span('sig-generic', part)
      })
      .join('')
  )
}

/** `type Name = …;` as HTML lines, laid out and linked, for a shape type `owner` declares. */
export function shapeLines(name: string, type: string, owner: string): string[] {
  const head = `${span('sig-keyword', 'type')} ${span('sig-name', name)} ${span('sig-punct', '=')} `
  const lines = layout(type, 0, `type ${name} = `.length)
  lines[lines.length - 1] += ';'
  return lines.map((line, i) => (i === 0 ? head : '') + lineHtml(line, owner))
}
