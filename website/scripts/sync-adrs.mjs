/**
 * `npm run sync:adrs` — publish the repository's ADRs into the website.
 *
 * Reads   ../docs/adr/NNNN-slug.md            (the corpus, tool-owned by the repo)
 * Writes  src/content/docs/docs/adr/NNNN.md   (generated pages — NEVER edit by hand;
 *                                              plain .md so ADR prose is never parsed as JSX)
 *         src/data/adrs.json                  (index: tooltips, sidebar, listing page)
 *
 * Everything under src/content/docs/docs/adr/ is regenerated from scratch on
 * every run; human prose about ADRs belongs in the ADRs themselves, in the
 * repository. Idempotent: run it whenever docs/adr changed.
 */

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { loadAdrs, plainText } from './lib/adr.mjs'
import { githubFile } from '../config/site.mjs'

const here = path.dirname(fileURLToPath(import.meta.url))
const websiteDir = path.resolve(here, '..')
const repoDir = path.resolve(websiteDir, '..')
const adrDir = path.join(repoDir, 'docs', 'adr')
const outPagesDir = path.join(websiteDir, 'src', 'content', 'docs', 'docs', 'adr')
const outDataDir = path.join(websiteDir, 'src', 'data')

const { adrs, warnings } = loadAdrs(adrDir)

// ---------------------------------------------------------------- link rewriting

const byNumber = new Map(adrs.map((a) => [a.number, a]))

/** Site URL of one ADR page. */
const adrUrl = (num) => `/docs/adr/${num}/`

/**
 * Rewrite the link targets an ADR body uses so they work on the site:
 *  - `NNNN-slug.md` (+`#fragment`)      -> /docs/adr/NNNN/
 *  - `../spec/…`, `../../crates/…` etc. -> the file on GitHub
 * Also tags ADR links with a tooltip marker the client script picks up.
 */
function rewriteBody(md, selfNumber) {
  // Markdown links whose target ends in .md and starts with an ADR number.
  md = md.replace(/\]\((\d{4})-[^)#]*\.md(#[^)]*)?\)/g, (_, num, frag) => `](${adrUrl(num)}${frag ?? ''})`)
  // Relative repo links (../.. up out of docs/adr) -> GitHub.
  md = md.replace(/\]\((\.\.\/[^)]+)\)/g, (whole, rel) => {
    const repoPath = path.posix.normalize(path.posix.join('docs/adr', rel))
    if (repoPath.startsWith('..')) return whole
    return `](${githubFile(repoPath)})`
  })
  // Bare in-directory file references that are not links stay as they are —
  // the corpus always links, tools/adr.py checks that.
  void selfNumber
  return md
}

/** Linked "ADR NNNN — title" for a metadata field that names numbers. */
function linkedRefs(nums) {
  return nums
    .map((n) => {
      const target = byNumber.get(n)
      const title = target ? target.title : `ADR ${n}`
      return `<a href="${adrUrl(n)}" data-adr="${n}">${n}</a> <span class="adr-ref-title">${escapeHtml(title)}</span>`
    })
    .join('<br/>')
}

function escapeHtml(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

/** Render a metadata field's markdown value inline (links + code only). */
function inlineMd(md) {
  return escapeHtml(md)
    .replace(/\[([^\]]*)\]\((\d{4})-[^)#]*\.md(#[^)]*)?\)/g, (_, text, num, frag) => `<a href="${adrUrl(num)}${frag ?? ''}" data-adr="${num}">${text}</a>`)
    .replace(/\[([^\]]*)\]\((\.\.\/[^)]+)\)/g, (whole, text, rel) => {
      const repoPath = path.posix.normalize(path.posix.join('docs/adr', rel))
      return repoPath.startsWith('..') ? whole : `<a href="${githubFile(repoPath)}">${text}</a>`
    })
    .replace(/\[([^\]]*)\]\(([^)]+)\)/g, '<a href="$2">$1</a>')
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/\*([^*]+)\*/g, '<em>$1</em>')
    .replace(/\b(\d{4})\b(?![^<]*<\/a>)/g, (whole, num) => (byNumber.has(num) ? `<a href="${adrUrl(num)}" data-adr="${num}">${num}</a>` : whole))
}

// ---------------------------------------------------------------- page generation

fs.rmSync(outPagesDir, { recursive: true, force: true })
fs.mkdirSync(outPagesDir, { recursive: true })
fs.mkdirSync(outDataDir, { recursive: true })

const FIELD_ORDER = ['Status', 'Date', 'Scope', 'Amends', 'Amended by', 'Validated by']

for (const adr of adrs) {
  const rows = []
  for (const field of FIELD_ORDER) {
    const value = adr.fields[field]
    if (value === undefined) continue
    let rendered
    if ((field === 'Amends' || field === 'Amended by') && (field === 'Amends' ? adr.amends : adr.amendedBy).length > 0) {
      rendered = linkedRefs(field === 'Amends' ? adr.amends : adr.amendedBy)
    } else {
      rendered = inlineMd(value)
    }
    rows.push({ field, rendered })
  }
  // Any field outside the known order still renders, at the end.
  for (const [field, value] of Object.entries(adr.fields)) {
    if (!FIELD_ORDER.includes(field)) rows.push({ field, rendered: inlineMd(value) })
  }

  const amendedNote =
    adr.amendedBy.length > 0
      ? `<div class="adr-amended-note">This decision has been folded forward: ` +
        `its body already reflects ${adr.amendedBy
          .map((n) => `<a href="${adrUrl(n)}" data-adr="${n}">ADR ${n}</a>`)
          .join(', ')}.</div>\n`
      : ''

  const metaHtml =
    `<div class="adr-meta" data-status="${escapeHtml(adr.status.toLowerCase())}">\n` +
    rows.map((r) => `  <div class="adr-meta-row"><span class="adr-meta-field">${r.field}</span><span class="adr-meta-value">${r.rendered}</span></div>`).join('\n') +
    `\n</div>\n`

  const inShortHtml = adr.inShort
    ? `<div class="adr-inshort"><span class="adr-inshort-label">In short</span><p>${inlineMd(adr.inShort)}</p></div>\n`
    : ''

  const description = plainText(adr.inShort).slice(0, 240) || `Architecture decision record ${adr.number} — ${adr.title}`

  const frontmatter = [
    '---',
    `title: ${JSON.stringify(`ADR ${adr.number} — ${adr.title}`)}`,
    `description: ${JSON.stringify(description)}`,
    'sidebar:',
    `  label: ${JSON.stringify(`${adr.number} ${adr.title}`)}`,
    'editUrl: false',
    'tableOfContents:',
    '  minHeadingLevel: 2',
    '  maxHeadingLevel: 3',
    '---',
  ].join('\n')

  const body = rewriteBody(adr.body, adr.number)

  const page = `${frontmatter}

<!-- GENERATED FILE — do not edit. Source: docs/adr/${adr.file}; regenerate with \`npm run sync:adrs\`. -->

${metaHtml}${amendedNote}${inShortHtml}
<a class="adr-source-link" href="${githubFile(`docs/adr/${adr.file}`)}">View the source of this decision on GitHub</a>

${body}
`
  fs.writeFileSync(path.join(outPagesDir, `${adr.number}.md`), page)
}

// ---------------------------------------------------------------- index page + data

const indexData = adrs.map((a) => ({
  number: a.number,
  title: a.title,
  status: a.status,
  date: a.date,
  inShort: plainText(a.inShort),
  amends: a.amends,
  amendedBy: a.amendedBy,
  url: adrUrl(a.number),
}))
fs.writeFileSync(path.join(outDataDir, 'adrs.json'), JSON.stringify(indexData, null, 2) + '\n')

const indexPage = `---
title: Architecture Decision Records
description: Every settled decision in Novis's design, one record per decision — with its status, what it amends and what amends it.
tableOfContents: false
editUrl: false
---

{/* GENERATED FILE — do not edit. Regenerate with \`npm run sync:adrs\`. */}

import AdrIndex from '@components/AdrIndex.astro'

Novis records every settled design decision as an Architecture Decision Record.
An ADR's body always states the **current** rule: a later decision is folded into
the earlier record's text, never left as an overlay. When a record has been
amended, the note at its top names the decisions folded into it.

<AdrIndex />
`
fs.writeFileSync(path.join(outPagesDir, 'index.mdx'), indexPage)

// ---------------------------------------------------------------- report

console.log(`sync:adrs — ${adrs.length} ADRs published, ${warnings.length} warning(s)`)
for (const w of warnings) console.warn(`  warn: ${w}`)
if (warnings.length > 0) process.exitCode = 0 // warnings are a report, not a failure
