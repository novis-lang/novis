/**
 * The files of the extension the guide *Write an extension in Rust* builds,
 * read in place at build time from `website/guides/highlight-extension/`, the way
 * src/lib/apps.ts reads `apps/`. A page shows exactly the code that
 * `bun nv ext-templates --example` builds, tests and calls.
 *
 * Left out: the build output under `target/`, the world's WIT under
 * `wit/deps/`, which `nvs ext new` writes and the reader never changes, and
 * `demo/nvs.toml`, which a reader writes from their own pin.
 */

// The glob is relative to this file, so its keys start with this prefix.
const ROOT = '../../guides/highlight-extension/'

const files = import.meta.glob(
  [
    '../../guides/highlight-extension/**/*.{rs,toml,wit,nvs,md}',
    '!../../guides/highlight-extension/target/**',
    '!../../guides/highlight-extension/wit/deps/**',
    '!../../guides/highlight-extension/demo/nvs.toml',
  ],
  { query: '?raw', import: 'default', eager: true },
) as Record<string, string>

export interface ExampleFile {
  /** The path inside the project, such as `src/lib.rs`. */
  path: string
  code: string
  /** The language the code block highlights it as. */
  lang: string
}

const LANGS: Record<string, string> = { rs: 'rust', toml: 'toml', wit: 'wit', nvs: 'novis', md: 'md' }

/** The order a reader meets the files in: the guide's steps. */
const ORDER = ['wit/highlight.wit', 'Cargo.toml', 'src/lib.rs', 'nvsx.toml', 'nvs/Code.nvs', 'tests/HighlightTest.nvs', 'demo/index.nvs', 'README.md']

const rank = (path: string) => {
  const at = ORDER.indexOf(path)
  return at === -1 ? ORDER.length : at
}

/** Every file of the example, in the order of the guide's steps. */
export function exampleFiles(): ExampleFile[] {
  const found = Object.entries(files)
    .map(([key, code]) => {
      const path = key.slice(ROOT.length)
      return { path, code: code.trimEnd(), lang: LANGS[path.split('.').pop()!] ?? 'txt' }
    })
    .sort((a, b) => rank(a.path) - rank(b.path) || a.path.localeCompare(b.path))
  if (found.length === 0) throw new Error('exampleFiles: no files under website/guides/highlight-extension/')
  return found
}

/** One file of the example. A path the project does not have fails the build. */
export function exampleFile(path: string): ExampleFile {
  const file = exampleFiles().find((f) => f.path === path)
  if (!file) throw new Error(`exampleFile: website/guides/highlight-extension/${path} is not a file of the example`)
  return file
}
