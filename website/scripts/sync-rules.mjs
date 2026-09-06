/**
 * `npm run sync:rules` — publish the repository's rulebook into the website.
 *
 * Reads   ../docs/rules/_index.json          the chapters and their order
 *         ../docs/rules/<topic>.json         each chapter's rules, in reading order
 *         ../docs/rules/<topic>/<slug>.md    each rule's prose, as written
 *         ../config/rule-sections.mjs        where a chapter is cut into pages
 *
 * Writes  src/data/rules.json                             the index the components and the sidebar read
 *         src/content/docs/docs/rules/<topic>/index.md    one chapter page
 *         src/content/docs/docs/rules/<topic>/<sec>.md    one page per section
 *
 * Everything under src/content/docs/docs/rules/ is regenerated from scratch on every
 * run, except the handwritten hub at index.mdx. A rule's prose belongs in the
 * repository, where the rule is: nothing here is edited by hand and nothing here is a
 * second home for anything.
 *
 * WHY PLAIN .md AND NOT .mdx
 *
 * Rule prose is real Markdown a person wrote, full of `#[Attribute(…)]`, `{field: T}`
 * and `<T>`, all of which MDX would try to read as JSX. So a page is `.md`, and the
 * per-rule chrome is raw HTML around it — a Markdown HTML block ends at a blank line,
 * so the prose between the wrappers is parsed as ordinary Markdown, code fences,
 * Expressive Code highlighting and all.
 *
 * ANCHORS ARE THE RULE'S OWN SLUG
 *
 * Every rule gets `<a id="<slug>">` of its own, so `/docs/rules/security/tainted-data/
 * #tainted-sources` is stable whatever the heading text later becomes — and that is
 * what every `rule:` citation in the prose is rewritten to point at.
 */

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { githubFile, decisionRecord } from '../config/site.mjs'
import { ruleSections, chapterLeads } from '../config/rule-sections.mjs'

const here = path.dirname(fileURLToPath(import.meta.url))
const websiteDir = path.resolve(here, '..')
const repoDir = path.resolve(websiteDir, '..')
const rulesDir = path.join(repoDir, 'docs', 'rules')
const outPagesDir = path.join(websiteDir, 'src', 'content', 'docs', 'docs', 'rules')
const outDataDir = path.join(websiteDir, 'src', 'data')

/** The route the whole rulebook hangs under. */
const ROOT_URL = '/docs/rules'

const fail = (message) => {
  console.error(`sync-rules: ${message}`)
  process.exit(1)
}

// ---------------------------------------------------------------- reading the rulebook

const readJson = (file) => JSON.parse(fs.readFileSync(file, 'utf8'))

if (!fs.existsSync(path.join(rulesDir, '_index.json'))) {
  fail(`no rulebook at ${path.relative(websiteDir, rulesDir)} — is the repository checked out?`)
}

const index = readJson(path.join(rulesDir, '_index.json'))

/** Every rule of every chapter, in the repository's own order. */
const topics = index.topics
  .slice()
  .sort((a, b) => a.order - b.order || a.topic.localeCompare(b.topic))
  .map((entry) => {
    const data = readJson(path.join(rulesDir, `${entry.topic}.json`))
    return {
      topic: entry.topic,
      title: entry.title,
      url: `${ROOT_URL}/${entry.topic}/`,
      rules: data.rules.map((rule) => {
        const slug = rule.id.split('/')[1]
        const bodyFile = path.join(rulesDir, entry.topic, `${slug}.md`)
        return {
          id: rule.id,
          slug,
          title: rule.title,
          status: rule.status,
          because: rule.because ?? [],
          diverges: rule.divergesFromPhp ?? null,
          seeAlso: rule.seeAlso ?? [],
          guardedBy: rule.guardedBy ?? [],
          source: path.posix.join('docs/rules', entry.topic, `${slug}.md`),
          body: fs.existsSync(bodyFile) ? fs.readFileSync(bodyFile, 'utf8').replace(/\s+$/, '') : '',
        }
      }),
    }
  })

// ---------------------------------------------------------------- cutting into sections

/**
 * Split one chapter into its configured sections. The cut is by first-rule slug rather
 * than by index (config/rule-sections.mjs says why), so a `from` that names nothing is
 * a stale cut and fails the sync rather than quietly merging two sections.
 */
function sectionsFor(topic) {
  const configured = ruleSections[topic.topic]
  if (!configured || configured.length === 0) {
    fail(`chapter "${topic.topic}" has no entry in config/rule-sections.mjs`)
  }
  if (configured[0].from !== topic.rules[0].slug) {
    fail(
      `chapter "${topic.topic}": the first section must start at the chapter's first rule ` +
        `("${topic.rules[0].slug}"), and it starts at "${configured[0].from}"`
    )
  }

  const starts = new Map(configured.map((section) => [section.from, section]))
  const single = configured.length === 1
  const sections = []
  for (const rule of topic.rules) {
    const opening = starts.get(rule.slug)
    if (opening) {
      starts.delete(rule.slug)
      sections.push({
        ...opening,
        // A chapter cut into one section has no page of its own: its rules are
        // rendered on the chapter page, so a short chapter costs one click, not two.
        url: single ? topic.url : `${topic.url}${opening.slug}/`,
        rules: [],
      })
    }
    sections[sections.length - 1].rules.push(rule)
  }
  if (starts.size > 0) {
    fail(
      `chapter "${topic.topic}": section${starts.size === 1 ? '' : 's'} cut at ` +
        `${[...starts.keys()].map((s) => `"${s}"`).join(', ')}, which no rule in the chapter is named`
    )
  }
  return sections
}

for (const topic of topics) {
  topic.sections = sectionsFor(topic)
  topic.single = topic.sections.length === 1
  // A one-section chapter leads with that section's own blurb; every other chapter owes
  // a lead of its own, and a missing one is a gap rather than an empty paragraph.
  topic.lead = topic.single ? topic.sections[0].blurb : chapterLeads[topic.topic]
  if (!topic.lead) fail(`chapter "${topic.topic}" has no entry in chapterLeads`)
  for (const section of topic.sections) {
    for (const rule of section.rules) {
      rule.url = section.url
      rule.href = `${section.url}#${rule.slug}`
    }
  }
}

/** Every rule by id, for resolving a `rule:` citation and a `seeAlso` entry. */
const byId = new Map()
for (const topic of topics) {
  for (const rule of topic.rules) byId.set(rule.id, { ...rule, topicTitle: topic.title })
}

// ---------------------------------------------------------------- prose → page markdown

const escapeHtml = (s) =>
  s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')

/** The sliver of inline Markdown the structured fields use: code spans and bold. */
const inlineHtml = (s) =>
  escapeHtml(s)
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')

/** A rule title as a link tooltip: plain text, no markup, no quotes to close early. */
const plainTitle = (s) => s.replace(/[`*"]/g, '')

/**
 * `divergesFromPhp` is written as a clause, because the repository renders it after a
 * comma. Here it stands on its own under a heading, so it starts a sentence — unless it
 * starts with code, where PHP's own spelling of something is the subject and lifting its
 * case would misquote it.
 */
const asSentence = (s) => (/^[a-z]/.test(s) ? s[0].toUpperCase() + s.slice(1) : s)

const dangling = []

/**
 * Rewrite the `rule:<topic>/<slug>` citations in a body to links. Both spellings the
 * repository admits — the bare token and the one an author naturally wraps in
 * backticks — become one link carrying the cited rule's own title as its tooltip.
 * Fenced code is left exactly as written; an inline code span is not, because
 * `` `rule:x/y` `` is how a citation is usually written.
 */
function linkCitations(body, from) {
  const citation = /`rule:([a-z0-9][a-z0-9-]*\/[a-z0-9][a-z0-9-]*)`|rule:([a-z0-9][a-z0-9-]*\/[a-z0-9][a-z0-9-]*)/g
  return body
    .split(/(^```[\s\S]*?^```)/m)
    .map((chunk, i) => {
      if (i % 2 === 1) return chunk // a fenced block, verbatim
      return chunk.replace(citation, (whole, quoted, bare) => {
        const id = quoted ?? bare
        const target = byId.get(id)
        if (!target) {
          dangling.push(`${from} cites rule:${id}, which no chapter declares`)
          return `\`${id}\``
        }
        return `[\`${id}\`](${target.href} "${plainTitle(target.title)}")`
      })
    })
    .join('')
}

const STATUS_LABEL = { shipped: 'Shipped', designed: 'Designed' }

/** One rule, as the Markdown-with-HTML block a generated page carries. */
function renderRule(rule) {
  const out = []
  out.push(`<div class="nv-rule" id="${rule.slug}">`)
  out.push('')
  out.push(`## ${rule.title}`)
  out.push('')
  out.push('<div class="nv-rule-tags">')
  out.push(
    `<span class="nv-rule-status" data-status="${rule.status}">${STATUS_LABEL[rule.status] ?? rule.status}</span>`
  )
  if (rule.diverges) out.push('<span class="nv-rule-flag">Differs from PHP</span>')
  out.push(`<a class="nv-rule-id" href="#${rule.slug}"><code>${escapeHtml(rule.id)}</code></a>`)
  out.push('</div>')
  out.push('')
  if (rule.body) {
    out.push(linkCitations(rule.body, rule.id))
    out.push('')
  }
  if (rule.diverges) {
    out.push('<aside class="nv-rule-diverges">')
    out.push('<p class="nv-rule-diverges-label">Where this differs from PHP</p>')
    out.push(`<p>${inlineHtml(asSentence(rule.diverges))}</p>`)
    out.push('</aside>')
    out.push('')
  }

  const meta = []
  const seeAlso = rule.seeAlso.map((id) => byId.get(id)).filter(Boolean)
  if (seeAlso.length > 0) {
    meta.push(
      `<div class="nv-rule-meta-row"><dt>See also</dt><dd>${seeAlso
        .map((r) => `<a href="${r.href}" title="${escapeHtml(plainTitle(r.title))}"><code>${r.id}</code></a>`)
        .join(' ')}</dd></div>`
    )
  }
  if (rule.because.length > 0) {
    meta.push(
      `<div class="nv-rule-meta-row"><dt>Decided in</dt><dd>${rule.because
        .map((n) => `<a href="${decisionRecord(n)}">record ${n}</a>`)
        .join(' ')}</dd></div>`
    )
  }
  if (rule.guardedBy.length > 0) {
    meta.push(
      `<div class="nv-rule-meta-row"><dt>Guarded by</dt><dd>${rule.guardedBy
        .map((p) => `<a href="${githubFile(p)}"><code>${escapeHtml(p)}</code></a>`)
        .join(' ')}</dd></div>`
    )
  }
  if (meta.length > 0) {
    out.push(`<dl class="nv-rule-meta">${meta.join('')}</dl>`)
    out.push('')
  }
  out.push('</div>')
  return out.join('\n')
}

/** The compact list of what is on this page, above the prose. */
function renderRuleList(rules, { linked }) {
  const items = rules
    .map((rule) => {
      const href = linked ? rule.href : `#${rule.slug}`
      return (
        `<li><a href="${href}">${inlineHtml(rule.title)}</a>` +
        `<span class="nv-rule-list-status" data-status="${rule.status}">${STATUS_LABEL[rule.status] ?? rule.status}</span>` +
        (rule.diverges ? '<span class="nv-rule-list-flag" title="Differs from PHP">PHP</span>' : '') +
        '</li>'
      )
    })
    .join('')
  return `<ol class="nv-rule-list">${items}</ol>`
}

/** Counts a chapter or a section leads with. */
function counts(rules) {
  const shipped = rules.filter((r) => r.status === 'shipped').length
  const diverging = rules.filter((r) => r.diverges).length
  return { total: rules.length, shipped, designed: rules.length - shipped, diverging }
}

function renderCounts(rules) {
  const c = counts(rules)
  const cell = (value, label, kind) =>
    `<div class="nv-count" data-kind="${kind}"><span class="nv-count-value">${value}</span>` +
    `<span class="nv-count-label">${label}</span></div>`
  return (
    '<div class="nv-counts">' +
    cell(c.total, c.total === 1 ? 'rule' : 'rules', 'total') +
    cell(c.shipped, 'shipped', 'shipped') +
    cell(c.designed, 'designed', 'designed') +
    cell(c.diverging, c.diverging === 1 ? 'differs from PHP' : 'differ from PHP', 'php') +
    '</div>'
  )
}

/** Frontmatter for a generated page. */
function frontmatter({ title, description, prev, next }) {
  const lines = [
    '---',
    // `tools/check-links.py` reads the first fifteen lines for this marker and skips the
    // file: a generated page's links are site routes that resolve in Astro's router and
    // never on disk, and the sources they came from are checked in their own spelling.
    '# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.',
    `title: ${JSON.stringify(title)}`,
    `description: ${JSON.stringify(description)}`,
  ]
  lines.push('editUrl: false')
  lines.push('lastUpdated: false')
  // Every page here opens with its own contents list, which carries each rule's status
  // and its PHP flag. Starlight's right-hand table of contents would repeat it as a
  // column of wrapped sentences — a rule title is a whole claim, not a two-word heading.
  lines.push('tableOfContents: false')
  if (prev) lines.push(`prev:\n  link: ${prev.link}\n  label: ${JSON.stringify(prev.label)}`)
  else lines.push('prev: false')
  if (next) lines.push(`next:\n  link: ${next.link}\n  label: ${JSON.stringify(next.label)}`)
  else lines.push('next: false')
  lines.push('---', '')
  return lines.join('\n')
}

/** Markdown → the plain sentence a `description` meta tag carries. */
function summarize(text, limit = 155) {
  const flat = text
    .replace(/`([^`]+)`/g, '$1')
    .replace(/\*\*([^*]+)\*\*/g, '$1')
    .replace(/\s+/g, ' ')
    .trim()
  if (flat.length <= limit) return flat
  return `${flat.slice(0, flat.lastIndexOf(' ', limit - 1))}…`
}

// ---------------------------------------------------------------- writing the pages

/** Every page this run is responsible for, so the prune below knows what is stale. */
const written = new Set()

function writePage(relPath, content) {
  const file = path.join(outPagesDir, relPath)
  fs.mkdirSync(path.dirname(file), { recursive: true })
  const normalized = content.replace(/\r\n/g, '\n').replace(/\n*$/, '\n')
  if (!fs.existsSync(file) || fs.readFileSync(file, 'utf8') !== normalized) {
    fs.writeFileSync(file, normalized)
  }
  written.add(relPath.split(path.sep).join('/'))
}

/** The page before and after this one, in reading order across the whole rulebook. */
const reading = []
for (const topic of topics) {
  reading.push({ link: topic.url, label: topic.title })
  if (!topic.single) for (const s of topic.sections) reading.push({ link: s.url, label: s.title })
}
const neighbours = (link) => {
  const i = reading.findIndex((page) => page.link === link)
  return { prev: reading[i - 1], next: reading[i + 1] }
}

for (const topic of topics) {
  // ------------------------------------------------------------- the chapter page
  const { prev, next } = neighbours(topic.url)
  const body = [
    frontmatter({
      title: topic.title,
      description: summarize(topic.lead),
      prev,
      next,
    }),
    `<p class="nv-section-lead">${inlineHtml(topic.lead)}</p>`,
    '',
    renderCounts(topic.rules),
    '',
  ]

  if (topic.single) {
    body.push(renderRuleList(topic.rules, { linked: false }))
    body.push('')
    for (const rule of topic.rules) {
      body.push(renderRule(rule))
      body.push('')
    }
  } else {
    body.push('<div class="nv-sections">')
    for (const section of topic.sections) {
      body.push('<section class="nv-section">')
      body.push(
        `<h2 class="nv-section-title"><a href="${section.url}">${inlineHtml(section.title)}</a>` +
          `<span class="nv-section-count">${section.rules.length}</span></h2>`
      )
      body.push(`<p class="nv-section-blurb">${inlineHtml(section.blurb)}</p>`)
      body.push(renderRuleList(section.rules, { linked: true }))
      body.push('</section>')
    }
    body.push('</div>')
  }
  writePage(path.join(topic.topic, 'index.md'), body.join('\n'))

  // ------------------------------------------------------------- one page per section
  if (topic.single) continue
  for (const section of topic.sections) {
    const nav = neighbours(section.url)
    const page = [
      frontmatter({
        title: section.title,
        description: summarize(section.blurb),
        prev: nav.prev,
        next: nav.next,
      }),
      `<p class="nv-section-lead">${inlineHtml(section.blurb)}</p>`,
      '',
      renderCounts(section.rules),
      '',
      renderRuleList(section.rules, { linked: false }),
      '',
    ]
    for (const rule of section.rules) {
      page.push(renderRule(rule))
      page.push('')
    }
    writePage(path.join(topic.topic, `${section.slug}.md`), page.join('\n'))
  }
}

// ---------------------------------------------------------------- the index components read

const data = {
  totals: counts(topics.flatMap((t) => t.rules)),
  topics: topics.map((topic) => ({
    topic: topic.topic,
    title: topic.title,
    url: topic.url,
    single: topic.single,
    counts: counts(topic.rules),
    sections: topic.sections.map((section) => ({
      slug: section.slug,
      title: section.title,
      blurb: section.blurb,
      url: section.url,
      counts: counts(section.rules),
      rules: section.rules.map((rule) => ({
        id: rule.id,
        slug: rule.slug,
        title: rule.title,
        status: rule.status,
        href: rule.href,
        diverges: rule.diverges,
      })),
    })),
  })),
}

fs.mkdirSync(outDataDir, { recursive: true })
fs.writeFileSync(path.join(outDataDir, 'rules.json'), `${JSON.stringify(data, null, 2)}\n`)

// ---------------------------------------------------------------- prune what moved away

/** The one handwritten page in the tree; everything else here is this script's. */
const HANDWRITTEN = new Set(['index.mdx'])

function prune(dir, prefix = '') {
  if (!fs.existsSync(dir)) return
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const rel = prefix === '' ? entry.name : `${prefix}/${entry.name}`
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      prune(full, rel)
      if (fs.readdirSync(full).length === 0) fs.rmdirSync(full)
    } else if (!HANDWRITTEN.has(rel) && !written.has(rel)) {
      fs.unlinkSync(full)
      console.log(`sync-rules: removed stale ${rel}`)
    }
  }
}
prune(outPagesDir)

// ---------------------------------------------------------------- report

if (dangling.length > 0) {
  for (const message of dangling) console.error(`sync-rules: ${message}`)
  fail(`${dangling.length} dangling citation${dangling.length === 1 ? '' : 's'}`)
}

const sectionCount = topics.reduce((n, t) => n + t.sections.length, 0)
console.log(
  `sync-rules: ${data.totals.total} rules in ${topics.length} chapters and ${sectionCount} sections ` +
    `→ ${written.size} pages`
)
