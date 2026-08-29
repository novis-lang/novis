/**
 * `npm run sync:core` — update the Core function reference from the repository.
 *
 * Reads   ../docs/spec/01-core-library.md      (authoritative for every signature)
 *         ../crates/nvs-stdlib/src/*.rs        (which members are implemented)
 *         scripts/spec-overrides.mjs           (hand-maintained parse corrections)
 *
 * Writes  src/data/core.json                   (tool-owned; regenerated every run)
 *         src/content/docs/docs/core/**.mdx    (created ONLY when missing — these
 *                                               pages hold human prose and the tool
 *                                               never overwrites an existing one)
 *
 * The report at the end is part of the contract: rows the spec parser could
 * not read, member pages whose member no longer exists, and members that
 * gained or lost their implementation all show up there.
 */

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { parseSpec } from './lib/spec.mjs'
import { scanRegistry } from './lib/registry.mjs'
import { blocksHtml, inlineHtml } from './lib/md.mjs'
import { overrides } from './spec-overrides.mjs'

const here = path.dirname(fileURLToPath(import.meta.url))
const websiteDir = path.resolve(here, '..')
const repoDir = path.resolve(websiteDir, '..')
const specPath = path.join(repoDir, 'docs', 'spec', '01-core-library.md')
const stdlibDir = path.join(repoDir, 'crates', 'nvs-stdlib', 'src')
const outDataDir = path.join(websiteDir, 'src', 'data')
const outPagesDir = path.join(websiteDir, 'src', 'content', 'docs', 'docs', 'core')

const specText = fs.readFileSync(specPath, 'utf8')
const { classes, warnings } = parseSpec(specText, overrides)
const { implemented, warnings: regWarnings } = scanRegistry(stdlibDir)
warnings.push(...regWarnings)

// ---------------------------------------------------------------- enrich with implementation status

for (const cls of classes) {
  const members = implemented[cls.name] ?? []
  cls.implemented = cls.name in implemented
  for (const m of cls.members) {
    m.implemented = members.includes(m.name)
  }
}

// ---------------------------------------------------------------- slugs and urls

const classSlug = (cls) => cls.id.toLowerCase().replace(/\./g, '-')
const methodSlug = (m) => m.name.toLowerCase()

for (const cls of classes) {
  cls.slug = classSlug(cls)
  cls.url = `/docs/core/${cls.slug}/`
  // Pre-rendered HTML for the spec excerpts class pages embed.
  cls.summaryHtml = cls.summary ? blocksHtml(cls.summary, 'docs/spec') : ''
  cls.surfaceHtml = cls.surface ? inlineHtml(cls.surface, 'docs/spec') : ''
  for (const m of cls.members) {
    m.slug = methodSlug(m)
    m.url = `/docs/core/${cls.slug}/${m.slug}/`
  }
}

// ---------------------------------------------------------------- data file

fs.mkdirSync(outDataDir, { recursive: true })
fs.writeFileSync(path.join(outDataDir, 'core.json'), JSON.stringify({ classes }, null, 2) + '\n')

// Changelog store: created once, then human/tool appended via release tooling.
const changelogPath = path.join(outDataDir, 'core-changelog.json')
if (!fs.existsSync(changelogPath)) {
  fs.writeFileSync(
    changelogPath,
    JSON.stringify(
      {
        '//': 'Per-member changelog. Key: member id (e.g. "Str.length"). Value: array of {version, change}. Empty until the first release that changes a member.',
      },
      null,
      2
    ) + '\n'
  )
}

// ---------------------------------------------------------------- page stubs

fs.mkdirSync(outPagesDir, { recursive: true })

let createdPages = 0
const yaml = (s) => JSON.stringify(s)

/** Human-readable phrase for the Replaces cell, or ''. Keeps inline-code
 * ticks — they are MDX-safe and protect characters like `<=>`. */
function replacesPhrase(replaces) {
  if (!replaces) return ''
  const clean = replaces.replace(/\s*\(([^)]*)\)/g, '').trim()
  if (!clean || /^nothing\b/i.test(clean)) return ''
  return clean
}

function classDir(cls) {
  return path.join(outPagesDir, cls.slug)
}

for (const cls of classes) {
  fs.mkdirSync(classDir(cls), { recursive: true })

  // ---- class page (human-owned after creation)
  const clsIndex = path.join(classDir(cls), 'index.mdx')
  if (!fs.existsSync(clsIndex)) {
    createdPages++
    const shortName = cls.name
    const page = `---
title: ${yaml(shortName)}
description: ${yaml(`The ${shortName} class of the Novis Core library — every member, with signatures and status.`)}
sidebar:
  label: ${yaml(cls.id.replace(/\./g, '\\'))}
  order: 0
novis:
  kind: class
  id: ${yaml(cls.id)}
  draft: true
---

import ClassOverview from '@components/ClassOverview.astro'

{/* HUMAN-OWNED PAGE. \`npm run sync:core\` created this stub once and will never
    overwrite it. The prose here is yours; the member table below renders from
    the generated data and stays current on its own. */}

{/* Group note: if this class needs an important note at the top (like the
    UTF-8 note on Str), write it here as a normal Starlight aside. */}

<ClassOverview id=${yaml(cls.id)} />
`
    fs.writeFileSync(clsIndex, page)
  }

  // ---- method pages (human-owned after creation)
  for (const m of cls.members) {
    const file = path.join(classDir(cls), `${m.slug}.mdx`)
    if (fs.existsSync(file)) continue
    createdPages++

    const fullName = `${cls.name}::${m.name}`
    const repl = replacesPhrase(m.replaces)
    const lead = repl ? `Novis's replacement for PHP's ${repl}.` : `A member of ${cls.name}.`
    const leadPlain = repl
      ? `Novis's replacement for PHP's ${repl.replace(/`/g, '')}.`
      : `A member of ${cls.name}.`

    const paramLines = m.params
      .map((p) => `- **\`$${p.name}\`** (\`${p.type || 'mixed'}\`${p.default !== null ? `, default \`${p.default}\`` : ''}${p.variadic ? ', variadic' : ''}) — *to be documented.*`)
      .join('\n')
    const optionLines = m.options
      .map((o) => `- **\`${o.name}\`**${o.type ? ` (\`${o.type}\`)` : ''} — *to be documented.*`)
      .join('\n')

    const page = `---
title: ${yaml(fullName)}
description: ${yaml(leadPlain)}
sidebar:
  label: ${yaml(m.name)}
novis:
  kind: method
  id: ${yaml(m.id)}
  draft: true
---

import MethodSignature from '@components/MethodSignature.astro'
import MethodChangelog from '@components/MethodChangelog.astro'
import MethodExamples from '@components/MethodExamples.astro'
import SeeAlso from '@components/SeeAlso.astro'

{/* HUMAN-OWNED PAGE. \`npm run sync:core\` created this stub once and will never
    overwrite it. Edit every prose section freely; the signature block, the
    changelog and the examples render from generated data and files. Remove
    \`draft: true\` above once the prose has been reviewed. */}

${lead}

<MethodSignature id=${yaml(m.id)} />

## Description

*To be documented${m.notes ? ` — the spec notes: ${m.notes.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/\*/g, '')}` : ''}.*
${
  paramLines
    ? `
### Parameters

${paramLines}
`
    : ''
}${
      optionLines
        ? `
### Options

${optionLines}
`
        : ''
    }
## Return value

Returns \`${m.returnType}\`. *To be documented.*

## Errors

*To be documented.*

<MethodChangelog id=${yaml(m.id)} />

{/* Tips & tricks: add a "## Tips" section here when there is something worth
    saying. The section is simply absent until then. */}

<MethodExamples id=${yaml(m.id)} />

{/* Related members: list ids like "Str.isEmpty". Renders nothing while empty. */}
<SeeAlso ids={[]} />
`
    fs.writeFileSync(file, page)
  }
}

// ---------------------------------------------------------------- orphan report

const known = new Set()
known.add('index.mdx') // the handwritten Core reference landing page
for (const cls of classes) {
  known.add(path.join(cls.slug, 'index.mdx'))
  for (const m of cls.members) known.add(path.join(cls.slug, `${m.slug}.mdx`))
}
const orphans = []
function walk(dir, rel = '') {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const relPath = path.join(rel, entry.name)
    if (entry.isDirectory()) walk(path.join(dir, entry.name), relPath)
    else if (entry.name.endsWith('.mdx') && !known.has(relPath)) orphans.push(relPath)
  }
}
walk(outPagesDir)

// ---------------------------------------------------------------- report

const memberCount = classes.reduce((n, c) => n + c.members.length, 0)
const implCount = classes.reduce((n, c) => n + c.members.filter((m) => m.implemented).length, 0)
console.log(
  `sync:core — ${classes.length} classes, ${memberCount} members (${implCount} implemented), ${createdPages} page(s) created, ${warnings.length} warning(s)`
)
for (const w of warnings) console.warn(`  warn: ${w}`)
if (orphans.length > 0) {
  console.warn('  orphan pages (member no longer in the spec — review and delete by hand):')
  for (const o of orphans) console.warn(`    src/content/docs/docs/core/${o.replace(/\\/g, '/')}`)
}
