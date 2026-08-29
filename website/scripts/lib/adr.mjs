/**
 * Parser for the repository's ADR corpus (docs/adr/NNNN-slug.md).
 *
 * The corpus is mechanically consistent — `tools/adr.py` in the repo root
 * enforces the metadata block, the closed heading set and amends-symmetry —
 * so this parser can rely on that shape and *fail loudly* when it is not met,
 * rather than guessing.
 *
 * Shape relied upon:
 *   # ADR NNNN — Title
 *   - **Status:** Accepted
 *   - **Date:** 2026-08-20
 *   - **Scope:** …          (may wrap over several lines)
 *   - **Amends:** …
 *   - **Amended by:** 0106  (bare numbers, comma-separated, or prose)
 *   - **Validated by:** …
 *   > **In short:** one-paragraph summary (may span multiple `>` lines)
 *   ## Context … (body)
 */

import fs from 'node:fs'
import path from 'node:path'

const FILE_RE = /^(\d{4})-(.+)\.md$/

/** @typedef {{
 *   number: string, slug: string, file: string, title: string,
 *   fields: Record<string, string>, status: string, date: string,
 *   amends: string[], amendedBy: string[], inShort: string, body: string,
 * }} Adr */

/**
 * Extract every four-digit ADR number referenced in a metadata field value.
 * Handles bare numbers ("0106"), linked ones ("[0020](0020-….md)") and
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
 * @param {string} adrDir absolute path to docs/adr
 * @returns {{ adrs: Adr[], warnings: string[] }}
 */
export function loadAdrs(adrDir) {
  const warnings = []
  /** @type {Adr[]} */
  const adrs = []

  for (const file of fs.readdirSync(adrDir).sort()) {
    const m = FILE_RE.exec(file)
    if (!m) continue
    const [, number, slug] = m
    const raw = fs.readFileSync(path.join(adrDir, file), 'utf8')
    const lines = raw.split(/\r?\n/)

    // Title
    const titleLine = lines[0] ?? ''
    const titleMatch = /^#\s*ADR\s+(\d{4})\s+—\s+(.+)$/.exec(titleLine)
    if (!titleMatch) {
      warnings.push(`${file}: first line is not "# ADR ${number} — Title" (got: ${titleLine.slice(0, 60)})`)
    }
    const title = titleMatch ? titleMatch[2].trim() : slug.replace(/-/g, ' ')

    // Metadata bullet block: consecutive `- **Name:** value` bullets after the title,
    // where a bullet's value may wrap onto indented continuation lines.
    /** @type {Record<string,string>} */
    const fields = {}
    let i = 1
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

    adrs.push({
      number,
      slug,
      file,
      title,
      fields,
      status: fields['Status'] ?? '',
      date: fields['Date'] ?? '',
      amends: numbersIn(fields['Amends'] ?? ''),
      amendedBy: numbersIn(fields['Amended by'] ?? ''),
      inShort,
      body,
    })
  }

  return { adrs, warnings }
}
