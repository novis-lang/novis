/**
 * Parser for the repository's decision records (docs/decisions/NNNN.md).
 *
 * The corpus is mechanically consistent — `tools/adr.py --check` in the repo
 * enforces the frozen shape — so this parser can rely on that shape and *fail
 * loudly* when it is not met, rather than guessing.
 *
 * Shape relied upon:
 *   ---
 *   date: 2026-08-20
 *   status: accepted             (or `retired`, or `superseded-by NNNN`)
 *   changes:
 *     creates:
 *       - topic/rule-slug
 *     modifies:
 *       - topic/rule-slug
 *   ---
 *   # ADR NNNN — Title
 *   - **Scope:** …               (may wrap over several lines)
 *   - **Depends on:** …
 *   - **Validated by:** …
 *   > **In short:** one-paragraph summary (may span multiple `>` lines)
 *   ## Context … (body)
 *
 * A record is frozen on acceptance: it never carries `Amends` / `Amended by`
 * fields, and the currently true rule is the fragment its `changes:` block
 * names under docs/rules/, not the record's body.
 */

import fs from 'node:fs'
import path from 'node:path'

const FILE_RE = /^(\d{4})\.md$/

/** @typedef {{
 *   number: string, file: string, title: string,
 *   fields: Record<string, string>, status: string, date: string,
 *   creates: string[], modifies: string[], inShort: string, body: string,
 * }} Adr */

/**
 * Extract every four-digit record number referenced in a field value.
 * Handles bare numbers ("0106"), linked ones ("[0020](0020.md)") and
 * comma/`and`-separated lists. Prose without a number yields [].
 * @param {string} value
 * @returns {string[]}
 */
export function numbersIn(value) {
  if (!value) return []
  const out = []
  for (const m of value.matchAll(/\b(\d{4})\b/g)) out.push(m[1])
  return [...new Set(out)]
}

/** Strip markdown emphasis/links/code down to plain text (for tooltips and meta descriptions). */
export function plainText(md) {
  return md
    .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1') // links -> text
    .replace(/[*_`]/g, '')
    .replace(/\s+/g, ' ')
    .trim()
}

/**
 * The YAML block between the two `---` lines, read with the three shapes the
 * corpus uses and nothing more: `key: value`, `key:` opening a mapping, and
 * `- item` lists under `changes.creates` / `changes.modifies`.
 * @param {string[]} lines
 * @returns {{ meta: Record<string, string>, creates: string[], modifies: string[], end: number }}
 */
function frontMatter(lines) {
  const meta = {}
  const lists = { creates: [], modifies: [] }
  if ((lines[0] ?? '').trim() !== '---') return { meta, ...lists, end: 0 }
  let current = null
  let i = 1
  for (; i < lines.length; i++) {
    const line = lines[i]
    if (line.trim() === '---') { i++; break }
    const item = /^\s+-\s+(.+)$/.exec(line)
    if (item && current) { lists[current].push(item[1].trim()); continue }
    const key = /^\s*([a-z]+):\s*(.*)$/.exec(line)
    if (!key) continue
    const [, name, value] = key
    if (name === 'creates' || name === 'modifies') { current = name; continue }
    if (name === 'changes') { current = null; continue }
    meta[name] = value.trim()
  }
  return { meta, ...lists, end: i }
}

/**
 * @param {string} adrDir absolute path to docs/decisions
 * @returns {{ adrs: Adr[], warnings: string[] }}
 */
export function loadAdrs(adrDir) {
  const warnings = []
  /** @type {Adr[]} */
  const adrs = []

  for (const file of fs.readdirSync(adrDir).sort()) {
    const m = FILE_RE.exec(file)
    if (!m) continue
    const [, number] = m
    const raw = fs.readFileSync(path.join(adrDir, file), 'utf8')
    const lines = raw.split(/\r?\n/)

    const fm = frontMatter(lines)
    if (fm.end === 0) warnings.push(`${file}: no YAML block between two --- lines`)
    let i = fm.end
    while (i < lines.length && lines[i].trim() === '') i++

    // Title
    const titleLine = lines[i] ?? ''
    const titleMatch = /^#\s*ADR\s+(\d{4})\s+(?:—|--)\s+(.+)$/.exec(titleLine)
    if (!titleMatch) {
      warnings.push(`${file}: no "# ADR ${number} — Title" after the YAML block (got: ${titleLine.slice(0, 60)})`)
    }
    const title = titleMatch ? titleMatch[2].trim() : `ADR ${number}`
    i++

    // Metadata bullet block: consecutive `- **Name:** value` bullets after the title,
    // where a bullet's value may wrap onto indented continuation lines.
    /** @type {Record<string,string>} */
    const fields = {}
    while (i < lines.length && lines[i].trim() === '') i++
    let currentField = null
    for (; i < lines.length; i++) {
      const line = lines[i]
      const fieldMatch = /^-\s+\*\*([^:*]+):\*\*\s*(.*)$/.exec(line)
      if (fieldMatch) {
        currentField = fieldMatch[1].trim()
        fields[currentField] = fieldMatch[2].trim()
        continue
      }
      if (currentField && /^\s+\S/.test(line)) {
        fields[currentField] += ' ' + line.trim()
        continue
      }
      break
    }

    // "In short" blockquote: the first `>` block after the metadata.
    let inShort = ''
    let bodyStart = i
    for (; i < lines.length; i++) {
      const line = lines[i]
      if (line.trim() === '') continue
      if (line.startsWith('>')) {
        const quote = []
        while (i < lines.length && (lines[i].startsWith('>') || lines[i].trim() === '')) {
          if (lines[i].startsWith('>')) quote.push(lines[i].replace(/^>\s?/, ''))
          else break
          i++
        }
        inShort = quote.join(' ').replace(/^\s*\*\*In short:\*\*\s*/i, '').trim()
        bodyStart = i
      }
      break
    }
    if (!inShort) warnings.push(`${file}: no "> **In short:**" block found`)

    const body = lines.slice(bodyStart).join('\n').trim()
    const rawStatus = fm.meta.status ?? ''
    // `accepted` -> "Accepted"; `superseded-by 0106` -> "Superseded by 0106".
    const status = rawStatus
      ? rawStatus.replace(/^superseded-by\s+/, 'superseded by ').replace(/^./, (c) => c.toUpperCase())
      : ''
    if (!status) warnings.push(`${file}: the YAML block names no status`)

    adrs.push({
      number,
      file,
      title,
      fields,
      status,
      date: fm.meta.date ?? '',
      creates: fm.creates,
      modifies: fm.modifies,
      inShort,
      body,
    })
  }

  return { adrs, warnings }
}
