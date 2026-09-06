/**
 * /llms.txt — the agent-facing site inventory (https://llmstxt.org/ format):
 * what this site is, and a link to every page with its one-line description.
 * Built from the same content collection as the pages themselves, so it can
 * never drift.
 */
import type { APIRoute } from 'astro'
import { getCollection } from 'astro:content'
import { SITE_URL, SITE_DESCRIPTION } from '../../config/site.mjs'
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
    `This file lists every page of the Novis website. The rulebook states every rule the\n` +
    `language holds itself to, chapter by chapter, each marked shipped or designed; the\n` +
    `Core reference documents the standard library, one page per member, generated from\n` +
    `the language's own specification.\n` +
    section('Start here', (id) => !id.includes('/')) +
    section('Getting started', (id) => id === 'docs' || id.startsWith('docs/getting-started') || id.startsWith('docs/release-notes')) +
    section('The rulebook', (id) => id.startsWith('docs/rules')) +
    section('Core reference', (id) => id.startsWith('docs/core'))

  return new Response(body, { headers: { 'Content-Type': 'text/plain; charset=utf-8' } })
}
