/**
 * Typed access to the generated Configuration and CLI reference data
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
  class: string
  apply: string
}

export interface RefPage {
  area: 'config' | 'cli'
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
export const CHANGED_BY: Record<string, string> = {
  System: 'Only `nvs.toml` sets this key. `Core\\Config::set` returns `false` for it.',
  Runtime: '`nvs.toml` sets the value that each request starts with. A request may change its own value with `Core\\Config::set`, up to the ceiling in `[limits.hard]`.',
  RuntimeTighten: '`nvs.toml` sets the value that each request starts with. A request may make its own value stricter with `Core\\Config::set`. A change that loosens it returns `false`.',
}

/** What a change to a configuration key in the file needs before it is in force. */
export const APPLIED_AT: Record<string, string> = {
  Reload: 'A change in the file takes effect a few seconds after you save it. The server does not restart.',
  Boot: 'A change in the file takes effect only when the server starts again.',
}
