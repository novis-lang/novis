/**
 * The example app pages a Core class page links to. An example app page lists the classes it shows in
 * its front matter, `shows:`. An entry matches a class when it is the class's name, or a namespace the
 * class is in: `Novis\Pdf` matches `Novis\Pdf\Document`. ClassOverview renders the list.
 */
import { getCollection } from 'astro:content'
import { withBase } from '~/lib/base'

export interface ExampleApp {
  title: string
  description: string
  url: string
}

const matches = (entry: string, className: string) => className === entry || className.startsWith(`${entry}\\`)

/** Every page whose `shows:` matches `className`, in the order of their paths. */
export async function examplesShowing(className: string): Promise<ExampleApp[]> {
  const pages = await getCollection('docs', (page) => (page.data.shows ?? []).some((entry) => matches(entry, className)))
  return pages
    .sort((a, b) => a.id.localeCompare(b.id))
    .map((page) => ({ title: page.data.title, description: page.data.description ?? '', url: withBase(`/${page.id}/`) }))
}
