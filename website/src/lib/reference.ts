/**
 * Typed access to the generated Configuration, CLI and Types reference data
 * (src/data/reference.json), which `bun nv render --website` writes from the
 * proofs roster. tools/nv/renderers/website-reference.ts says how the pages
 * are grouped.
 */
import data from '../data/reference.json'
import { withBase } from './base'

export interface RefFeature {
  id: string
  title: string
  examples: string
}

export interface RefDirective extends RefFeature {
  key: string
  /** How a request may change this key: `up-to-ceiling`, `off-only`, or empty when it may not. */
  how: string
  /** The keys under this one a request may change, relative to it. */
  settable: string[]
  apply: string
}

export interface RefPage {
  area: 'config' | 'cli' | 'types'
  slug: string
  url: string
  title: string
  group: string
  intro: RefFeature | null
  keys: RefDirective[]
}

export const pages = (data as { pages: RefPage[] }).pages
for (const page of pages) page.url = withBase(page.url)

/** Who may change a configuration key, in the words its page shows. */
export function changedBy(key: RefDirective): string {
  if (key.how === 'up-to-ceiling') {
    return '`nvs.toml` sets the value that each request starts with. A request may change its own value with `Core\\Config::set`, up to the ceiling where the key has one.'
  }
  if (key.how === 'off-only') {
    return '`nvs.toml` sets the value that each request starts with. A request may set its own value to `false` with `Core\\Config::set`, and to nothing else.'
  }
  if (key.settable.length > 0) {
    const names = key.settable.map((k) => `\`${k}\``).join(', ')
    return `A request may change these keys in this block with \`Core\\Config::set\`: ${names}. Only \`nvs.toml\` sets the others.`
  }
  return 'Only `nvs.toml` sets this key. A request may not change it.'
}

/** What a change to a configuration key in the file needs before it is in force. */
export const APPLIED_AT: Record<string, string> = {
  Reload: 'A change in the file takes effect a few seconds after you save it. The server does not restart.',
  Boot: 'A change in the file takes effect only when the server starts again.',
}
