/**
 * The files of each example app under `apps/`, read in place at build time the
 * way src/lib/feature.ts reads `docs/examples/`, so a page shows exactly the
 * code that `bun nv site --check apps` runs. A file the glob does not match,
 * such as `.gitignore`, is left out.
 */

// The glob is relative to this file, so its keys start with this prefix.
const ROOT = '../../../apps/'

const files = import.meta.glob('../../../apps/*/**/*.{nvs,toml,md,svg}', { query: '?raw', import: 'default', eager: true }) as Record<string, string>

export interface AppFile {
  /** The path inside the app, such as `src/App.nvs`. */
  path: string
  code: string
  /** The language the code block highlights it as. */
  lang: string
}

const LANGS: Record<string, string> = { nvs: 'novis', toml: 'toml', md: 'md', svg: 'xml' }

/** The order of an app's top-level entries: the configuration, the entry file, the classes, then the rest. */
const FIRST = ['nvs.toml', 'public', 'src', 'assets', 'README.md']

const rank = (path: string) => {
  const at = FIRST.indexOf(path.split('/')[0]!)
  return at === -1 ? FIRST.length : at
}

/** Every file of the app in the folder `apps/<app>`, in the order a reader meets them. */
export function appFiles(app: string): AppFile[] {
  const prefix = `${ROOT}${app}/`
  const found = Object.entries(files)
    .filter(([key]) => key.startsWith(prefix))
    .map(([key, code]) => {
      const path = key.slice(prefix.length)
      return { path, code: code.trimEnd(), lang: LANGS[path.split('.').pop()!] ?? 'txt' }
    })
    .sort((a, b) => rank(a.path) - rank(b.path) || a.path.localeCompare(b.path))
  if (found.length === 0) throw new Error(`appFiles: no files under apps/${app}/`)
  return found
}
