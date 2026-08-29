/**
 * `npm run sync:core` — update the Core function reference from the repository.
 *
 * Reads   ../docs/spec/01-core-library.md      (authoritative for the surface:
 *                                               every member, planned or not —
 *                                               but only *implemented* members
 *                                               are published to the site)
 *         ../crates/nvs-stdlib/src/*.rs        (which members are implemented)
 *         `nvs meta --json`                    (registry-carried documentation,
 *                                               ADR 0117 — optional until the
 *                                               toolchain provides it)
 *         scripts/spec-overrides.mjs           (hand-maintained parse corrections)
 *
 * Writes  src/data/core.json                   (tool-owned; regenerated every run)
 *         src/content/docs/docs/core/**.mdx    (ownership is per page, decided by
 *                                               its `draft` flag — see below)
 *
 * Page ownership (the whole design hangs on this): a page carrying
 * `novis.draft: true` is TOOL-OWNED and regenerated on every run, so its
 * defaults can never go stale. Removing the flag hands the page to humans
 * forever — from then on this script never touches it. The stubs themselves
 * contain no generated prose: every default renders at build time from
 * core.json through the Method* components, so even a human-owned page keeps
 * following the repository wherever it kept a component.
 *
 * The report at the end is part of the contract: rows the spec parser could
 * not read, member pages whose member no longer exists, and where the
 * documentation came from all show up there.
 */

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { parseSpec } from './lib/spec.mjs'
import { scanRegistry } from './lib/registry.mjs'
import { loadRegistryDocs } from './lib/meta.mjs'
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

// ---------------------------------------------------------------- registry docs (ADR 0117)

// Field-wise precedence: a doc field the registry carries wins; a field it
// lacks falls back to the spec-derived default. The Method* components apply
// that rule at build time — here we only attach what the toolchain reported.
const { docs: registryDocs, note: metaNote } = loadRegistryDocs(repoDir)

const asHtml = (s) => inlineHtml(String(s), 'docs/spec')

for (const cls of classes) {
  for (const m of cls.members) {
    m.notesHtml = m.notes ? asHtml(m.notes) : ''
    const doc = registryDocs.get(`${cls.name}::${m.name}`)
    if (!doc) continue
    m.doc = {
      ...(doc.short ? { shortHtml: asHtml(doc.short) } : {}),
      ...(Array.isArray(doc.params)
        ? {
            params: doc.params.map((p) => ({
              name: String(p.name ?? ''),
              descHtml: p.desc ? asHtml(p.desc) : '',
              ...(Array.isArray(p.shape)
                ? {
                    shape: p.shape.map((k) => ({
                      key: String(k.key ?? ''),
                      type: String(k.type ?? ''),
                      descHtml: k.desc ? asHtml(k.desc) : '',
                    })),
                  }
                : {}),
            })),
          }
        : {}),
      ...(doc.return ? { returnHtml: asHtml(doc.return) } : {}),
      ...(Array.isArray(doc.errors)
        ? { errors: doc.errors.map((e) => ({ error: String(e.error ?? ''), descHtml: e.desc ? asHtml(e.desc) : '' })) }
        : {}),
    }
    for (const p of m.doc.params ?? []) {
      const known = m.params.some((sp) => sp.name === p.name) || m.options.some((o) => o.name === p.name)
      if (!known) warnings.push(`registry documents parameter "${p.name}" on ${cls.name}::${m.name}, which the spec does not declare`)
    }
  }
}

// ---------------------------------------------------------------- publish only what is implemented

// The site shows what the toolchain has: a member not in the registry, and a
// class with no registered member, are not published. The full spec surface
// still parses above, so drift warnings keep seeing all of it.
const parsedClassCount = classes.length
const parsedMemberCount = classes.reduce((n, c) => n + c.members.length, 0)
for (const cls of classes) {
  cls.members = cls.members.filter((m) => m.implemented)
  cls.unparsed = []
}
const published = classes.filter((c) => c.members.length > 0)

// ---------------------------------------------------------------- slugs and urls

const classSlug = (cls) => cls.id.toLowerCase().replace(/\./g, '-')
const methodSlug = (m) => m.name.toLowerCase()

for (const cls of published) {
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
fs.writeFileSync(path.join(outDataDir, 'core.json'), JSON.stringify({ classes: published }, null, 2) + '\n')

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
let regeneratedPages = 0
let humanPages = 0
const yaml = (s) => JSON.stringify(s)

/** True when the page still carries `draft: true` in its frontmatter — i.e.
 * no human has taken ownership of it, so the tool may regenerate it. */
function isToolOwned(file) {
  const fm = /^---\r?\n([\s\S]*?)\r?\n---/.exec(fs.readFileSync(file, 'utf8'))
  return fm ? /\bdraft:\s*true\b/.test(fm[1]) : false
}

/** Write a stub: create when missing, regenerate while tool-owned, keep once human-owned. */
function writeStub(file, content) {
  if (!fs.existsSync(file)) {
    createdPages++
  } else if (isToolOwned(file)) {
    regeneratedPages++
  } else {
    humanPages++
    return
  }
  fs.writeFileSync(file, content)
}

/** Markdown → plain text for the frontmatter `description` (meta tags). */
function plain(s) {
  return s
    .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/`/g, '')
    .replace(/\*\*?/g, '')
    .trim()
}

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

const ownershipComment = `{/* OWNERSHIP: while \`draft: true\` stands above, this page is TOOL-OWNED and
    \`npm run sync:core\` REGENERATES it on every run — edits here will be lost.
    To take ownership, remove \`draft: true\`; the tool then never touches this
    file again. Keep a <Method...> component wherever its default (rendered
    from the repository's own data) is good enough; replace one with your own
    prose where it is not. */}`

for (const cls of published) {
  fs.mkdirSync(classDir(cls), { recursive: true })

  // ---- class page
  const shortName = cls.name
  const clsPage = `---
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

${ownershipComment}

{/* Group note: if this class needs an important note at the top (like the
    UTF-8 note on Str), write it here as a normal Starlight aside. */}

<ClassOverview id=${yaml(cls.id)} />
`
  writeStub(path.join(classDir(cls), 'index.mdx'), clsPage)

  // ---- method pages
  for (const m of cls.members) {
    const fullName = `${cls.name}::${m.name}`
    const repl = replacesPhrase(m.replaces)
    const leadPlain = m.doc?.shortHtml
      ? plain(String(registryDocs.get(`${cls.name}::${m.name}`)?.short ?? ''))
      : repl
        ? `Novis's replacement for PHP's ${plain(repl)}.`
        : `A member of ${cls.name}.`

    const hasParams = m.params.length > 0
    const hasOptions = m.options.length > 0

    const imports = [
      `import MethodLead from '@components/MethodLead.astro'`,
      `import MethodSignature from '@components/MethodSignature.astro'`,
      `import MethodDescription from '@components/MethodDescription.astro'`,
      ...(hasParams || hasOptions ? [`import ParamDocs from '@components/ParamDocs.astro'`] : []),
      `import MethodReturn from '@components/MethodReturn.astro'`,
      `import MethodErrors from '@components/MethodErrors.astro'`,
      `import MethodChangelog from '@components/MethodChangelog.astro'`,
      `import MethodExamples from '@components/MethodExamples.astro'`,
      `import SeeAlso from '@components/SeeAlso.astro'`,
    ].join('\n')

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

${imports}

${ownershipComment}

<MethodLead id=${yaml(m.id)} />

<MethodSignature id=${yaml(m.id)} />

## Description

<MethodDescription id=${yaml(m.id)} />
${
  hasParams
    ? `
### Parameters

<ParamDocs id=${yaml(m.id)} kind="params" />
`
    : ''
}${
      hasOptions
        ? `
### Options

<ParamDocs id=${yaml(m.id)} kind="options" />
`
        : ''
    }
## Return value

<MethodReturn id=${yaml(m.id)} />

## Errors

<MethodErrors id=${yaml(m.id)} />

<MethodChangelog id=${yaml(m.id)} />

{/* Tips & tricks: add a "## Tips" section here when there is something worth
    saying. The section is simply absent until then. */}

<MethodExamples id=${yaml(m.id)} />

{/* Related members: list ids like "Str.isEmpty". Renders nothing while empty. */}
<SeeAlso ids={[]} />
`
    writeStub(path.join(classDir(cls), `${m.slug}.mdx`), page)
  }
}

// ---------------------------------------------------------------- orphan report

const known = new Set()
known.add('index.mdx') // the handwritten Core reference landing page
for (const cls of published) {
  known.add(path.join(cls.slug, 'index.mdx'))
  for (const m of cls.members) known.add(path.join(cls.slug, `${m.slug}.mdx`))
}
// A tool-owned orphan (still `draft: true`) is deleted outright — its member
// left the published set, and nothing in it was human-written. A human-owned
// orphan is only ever reported.
const orphans = []
let deletedPages = 0
function walk(dir, rel = '') {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name)
    const relPath = path.join(rel, entry.name)
    if (entry.isDirectory()) {
      walk(full, relPath)
      if (fs.readdirSync(full).length === 0) fs.rmdirSync(full)
    } else if (entry.name.endsWith('.mdx') && !known.has(relPath)) {
      if (isToolOwned(full)) {
        fs.unlinkSync(full)
        deletedPages++
      } else {
        orphans.push(relPath)
      }
    }
  }
}
walk(outPagesDir)

// ---------------------------------------------------------------- report

const memberCount = published.reduce((n, c) => n + c.members.length, 0)
console.log(
  `sync:core — spec surface ${parsedClassCount} classes / ${parsedMemberCount} members; ` +
    `published ${published.length} classes / ${memberCount} members (implemented only); ` +
    `${createdPages} page(s) created, ${regeneratedPages} draft page(s) regenerated, ${deletedPages} deleted, ` +
    `${humanPages} human-owned page(s) untouched, ${warnings.length} warning(s)`
)
console.log(`  registry docs: ${metaNote}`)
for (const w of warnings) console.warn(`  warn: ${w}`)
if (orphans.length > 0) {
  console.warn('  orphan pages (member no longer in the spec — review and delete by hand):')
  for (const o of orphans) console.warn(`    src/content/docs/docs/core/${o.replace(/\\/g, '/')}`)
}
