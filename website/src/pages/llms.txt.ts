/**
 * /llms.txt — the agent-facing site inventory (https://llmstxt.org/ format):
 * what this site is, and a link to every page with its one-line description.
 * Built from the same content collection as the pages themselves, so it can
 * never drift.
 */
import type { APIRoute } from 'astro'
import { getCollection } from 'astro:content'
import { SITE_URL, SITE_DESCRIPTION, AREAS, INSTALL } from '../../config/site.mjs'
import { withBase } from '~/lib/base'

export const GET: APIRoute = async () => {
  const docs = await getCollection('docs')
  const url = (id: string) =>
    id === 'index' || id === '' ? `${SITE_URL}${withBase('/')}` : `${SITE_URL}${withBase(`/${id.replace(/\/index$/, '')}/`)}`

  const line = (d: (typeof docs)[number]) =>
    `- [${d.data.title}](${url(d.id)})${d.data.description ? `: ${String(d.data.description).replace(/\s+/g, ' ')}` : ''}`

  const section = (title: string, filter: (id: string) => boolean) => {
    const pages = docs.filter((d) => filter(d.id)).sort((a, b) => a.id.localeCompare(b.id))
    if (pages.length === 0) return ''
    return `\n## ${title}\n\n${pages.map(line).join('\n')}\n`
  }

  const body =
    `# Novis\n\n> ${SITE_DESCRIPTION}\n\n` +
    `This file lists every page of the Novis website. Install shows how to get Novis.\n` +
    `Guides has examples and shows how to use Novis. Syntax shows how to write it.\n` +
    `Reference has a page for every class and function. In-Depth explains the decisions\n` +
    `and ideas behind the language.\n` +
    section('Start here', (id) => !id.includes('/')) +
    [INSTALL, ...AREAS]
      .map(({ label, href }) => {
        const prefix = href.replace(/^\/|\/$/g, '')
        return section(label, (id) => id === prefix || id.startsWith(`${prefix}/`))
      })
      .join('')

  return new Response(body, { headers: { 'Content-Type': 'text/plain; charset=utf-8' } })
}
