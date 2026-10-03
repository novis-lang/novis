/**
 * What a Reference page reads from `docs/examples/`, in place: a feature's
 * `about.md`, its examples and its `nvs.toml`. The site keeps no copy of any.
 *
 * A feature's directory is its `examples` field in core.json or
 * reference.json, which is its feature path in the proofs roster
 * (`core/Time-Date/at`, `config/limits-hard`), so no page splits an id into a
 * path by hand.
 *
 * `about.md` is plain paragraphs and `-` lists with inline code, bold,
 * italics and links, which is all the about pages use. Its last line may be
 * `related: Core\Bytes::length, Core\Str::graphemes`: that line is not shown
 * as prose, each name becomes a link, and a name with no page fails the build.
 */
import changelog from '../data/core-changelog.json'
import { classes } from './core'
import { withBase } from './base'

export interface Change {
  version: string
  change: string
}

/** The member's changelog entries from src/data/core-changelog.json, keyed by member id. */
export function changesFor(id: string): Change[] {
  const entries = (changelog as Record<string, unknown>)[id]
  return Array.isArray(entries) ? (entries as Change[]) : []
}

// The globs are relative to this file, so their keys start with this prefix.
const ROOT = '../../../docs/examples/'

const abouts = import.meta.glob('../../../docs/examples/{core,config,tools}/**/about.md', { query: '?raw', import: 'default', eager: true }) as Record<string, string>
const sources = import.meta.glob('../../../docs/examples/{core,config,tools}/**/*.nvs', { query: '?raw', import: 'default', eager: true }) as Record<string, string>
const outputs = import.meta.glob('../../../docs/examples/{core,config,tools}/**/*.out', { query: '?raw', import: 'default', eager: true }) as Record<string, string>
const tomls = import.meta.glob('../../../docs/examples/{config,tools}/**/nvs.toml', { query: '?raw', import: 'default', eager: true }) as Record<string, string>

/** Anything with a directory under `docs/examples/`: a Core member, a configuration key, a CLI section. */
export interface Feature {
  examples: string
}

const byName = new Map<string, string>()
for (const cls of classes) for (const m of cls.members) byName.set(`${cls.name}::${m.name}`, m.id)

const escapeHtml = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')

export function inlineHtml(md: string): string {
  return md
    .split(/(`[^`]+`)/)
    .map((part) =>
      /^`[^`]+`$/.test(part)
        ? `<code>${escapeHtml(part.slice(1, -1))}</code>`
        : escapeHtml(part)
            .replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, text: string, href: string) => `<a href="${href.startsWith('/') ? withBase(href) : href}">${text}</a>`)
            .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
            .replace(/(^|[^\w*])\*([^*\s][^*]*)\*/g, '$1<em>$2</em>')
    )
    .join('')
}

function blocksHtml(md: string): string[] {
  const out: string[] = []
  let paragraph: string[] = []
  let list: string[] | null = null
  const flush = () => {
    if (paragraph.length > 0) out.push(`<p>${inlineHtml(paragraph.join(' '))}</p>`)
    if (list) out.push(`<ul>${list.map((item) => `<li>${inlineHtml(item)}</li>`).join('')}</ul>`)
    paragraph = []
    list = null
  }
  for (const line of md.split(/\r?\n/)) {
    const trimmed = line.trim()
    if (trimmed === '') {
      flush()
    } else if (/^-\s+/.test(trimmed)) {
      if (paragraph.length > 0) flush()
      list ??= []
      list.push(trimmed.replace(/^-\s+/, ''))
    } else if (list && /^\s+\S/.test(line)) {
      list[list.length - 1] += ` ${trimmed}`
    } else {
      if (list) flush()
      paragraph.push(trimmed)
    }
  }
  flush()
  return out
}

export interface About {
  /** One HTML string per paragraph or list, in order. */
  blocks: string[]
  /** The first paragraph as plain text, for the page's description. */
  summary: string
  /** Member ids in core.json, in the order the `related:` line names them. */
  related: string[]
  /** The repository path of the file. */
  path: string
}

/** The feature's `about.md`, or `null` when it has none. */
export function aboutFor(feature: Feature): About | null {
  const path = `docs/examples/${feature.examples}/about.md`
  const text = abouts[`${ROOT}${feature.examples}/about.md`]
  if (text === undefined) return null
  const lines = text.trimEnd().split(/\r?\n/)
  const last = /^related:\s*(.*)$/.exec(lines[lines.length - 1] ?? '')
  const related: string[] = []
  if (last) {
    lines.pop()
    for (const name of last[1].split(',').map((n) => n.trim()).filter(Boolean)) {
      const id = byName.get(name)
      if (!id) throw new Error(`${path}: the related: line names ${name}, which has no page in the Core reference`)
      related.push(id)
    }
  }
  const prose = lines.join('\n')
  const first = prose.trim().split(/\r?\n\s*\r?\n/)[0] ?? ''
  const summary = first.replace(/\s+/g, ' ').replace(/`/g, '').replace(/\*\*?/g, '').replace(/\[([^\]]*)\]\([^)]*\)/g, '$1').trim()
  return { blocks: blocksHtml(prose), summary, related, path }
}

export interface Example {
  title: string
  code: string
  output: string | undefined
}

/** The `nvs.toml` the feature's examples run with, or `null` when they need none. */
export function configFor(feature: Feature): string | null {
  return tomls[`${ROOT}${feature.examples}/nvs.toml`]?.trimEnd() ?? null
}

/**
 * Every example of the feature, ordered by filename. `NN-some-title.nvs` is
 * one example, and a sibling `NN-some-title.out` is its output, which
 * `bun nv proofs --run` checks. A file in a subdirectory is a companion file,
 * not an example.
 */
export function examplesFor(feature: Feature): Example[] {
  const dir = `${ROOT}${feature.examples}/`
  return Object.keys(sources)
    .filter((p) => p.startsWith(dir) && !p.slice(dir.length).includes('/'))
    .sort()
    .map((p) => {
      const base = p.replace(/\.nvs$/, '')
      const title = base
        .slice(dir.length)
        .replace(/^\d+-/, '')
        .replace(/-/g, ' ')
        .replace(/^./, (c) => c.toUpperCase())
      return { title, code: sources[p].trim(), output: outputs[`${base}.out`]?.trimEnd() }
    })
}
