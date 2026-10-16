/**
 * /search/names.json — every page a reader can reach by its name: each Core
 * class and member, each `nvs.toml` key and block, each CLI page, each type. The search
 * dialog (src/components/Search.astro) loads it the first time it opens and
 * matches the typed words against the names on every keystroke, so a name
 * search needs no request after that one. Pagefind still searches the text.
 *
 * Each entry is `[name, url, summary, kind]`. The name is what the reader
 * types and sees; the summary is the first sentence of the page's `about.md`,
 * or of the registry card when there is none.
 */
import type { APIRoute } from 'astro'
import { classes } from '~/lib/core'
import { pages } from '~/lib/reference'
import { aboutFor } from '~/lib/feature'

type Kind = 'class' | 'method' | 'setting' | 'command' | 'type'
type Entry = [name: string, url: string, summary: string, kind: Kind]

const plain = (html: string) =>
  html.replace(/<[^>]+>/g, '').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&amp;/g, '&')

/** The first sentence of a summary, so one line of the list fits it. */
const firstSentence = (text: string) => text.replace(/\s+/g, ' ').trim().split(/(?<=[.!?])\s/)[0] ?? ''

export const GET: APIRoute = () => {
  const entries: Entry[] = []
  for (const cls of classes) {
    const count = cls.members.length
    entries.push([cls.name, cls.url, `${count} ${count === 1 ? 'method' : 'methods'}`, 'class'])
    for (const member of cls.members) {
      const summary = aboutFor(member)?.summary || plain(member.doc?.shortHtml ?? '')
      entries.push([`${cls.name}::${member.name}`, member.url, firstSentence(summary), 'method'])
    }
  }
  for (const page of pages) {
    const intro = page.intro ? aboutFor(page.intro) : null
    const kind: Kind = page.area === 'config' ? 'setting' : page.area === 'types' ? 'type' : 'command'
    // A block with a key of its own name, like `[limits]` and `limits`, is listed once, under the key.
    const block = page.title.replace(/[^a-z0-9.]/gi, '')
    const own = page.keys.find((key) => key.key === block)
    const count = `${page.keys.length} ${page.keys.length === 1 ? 'setting' : 'settings'}`
    const summary = intro ? firstSentence(intro.summary) : page.keys.length > 0 ? count : ''
    if (!own) entries.push([page.title, page.url, summary, kind])
    for (const key of page.keys) {
      const slug = key.key.replace(/[^a-z0-9]+/gi, '-').toLowerCase()
      const url = key === own ? page.url : `${page.url}#${slug}`
      entries.push([key.key, url, firstSentence(aboutFor(key)?.summary ?? ''), kind])
    }
  }
  return new Response(JSON.stringify(entries), { headers: { 'Content-Type': 'application/json; charset=utf-8' } })
}
