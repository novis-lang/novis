/**
 * `npm run sync:examples` — copy the repository's example tree into the site.
 *
 * The examples live in `../docs/examples/` and not here, because the same sweep that tests a
 * feature writes them: `python tools/dossier.py` audits, runs and blesses them, and ADR 0134 is
 * why they exist at all. `../docs/examples/README.md` owns what an example is.
 *
 * So `website/examples/` is a **tool-owned mirror** — the same rule the ADR pages already follow.
 * It is emptied and rewritten on every sync, and a file edited here is lost on the next one; edit
 * `../docs/examples/<the same path>` instead. Only `.nvs` programs, their `.out` files and each
 * feature's `about.md` description are copied; anything else in the source tree (a README) stays
 * in the repository.
 *
 * `npm run examples:check` still runs whatever is in `website/examples/` through the real binary,
 * so a stale mirror fails here rather than shipping.
 */

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const here = path.dirname(fileURLToPath(import.meta.url))
const websiteDir = path.resolve(here, '..')
const repoDir = path.resolve(websiteDir, '..')
const source = path.join(repoDir, 'docs', 'examples')
const destination = path.join(websiteDir, 'examples')

if (!fs.existsSync(source)) {
  console.error(`sync:examples — ${path.relative(repoDir, source)} does not exist.`)
  process.exit(1)
}

const wanted = new Set(['.nvs', '.out'])
const copied = []

function walk(dir, relative = '') {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    const full = path.join(dir, entry.name)
    const rel = relative ? path.join(relative, entry.name) : entry.name
    if (entry.isDirectory()) {
      walk(full, rel)
    } else if (wanted.has(path.extname(entry.name)) || entry.name === 'about.md') {
      const target = path.join(destination, rel)
      fs.mkdirSync(path.dirname(target), { recursive: true })
      fs.copyFileSync(full, target)
      copied.push(rel.replace(/\\/g, '/'))
    }
  }
}

// Rewritten from scratch, so an example deleted in the repository leaves the site with it.
fs.rmSync(destination, { recursive: true, force: true })
walk(source)

const features = new Set(copied.filter((f) => f.endsWith('.nvs')).map((f) => path.dirname(f)))
console.log(
  `sync:examples — ${copied.filter((f) => f.endsWith('.nvs')).length} examples across ` +
    `${features.size} features copied from docs/examples/ into website/examples/.`,
)
