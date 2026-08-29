/**
 * `npm run examples:check` — run every website example through the real `nvs`
 * binary and compare its output against the sibling `.out` file.
 *
 * The binary is found via, in order: the NVS_BIN environment variable,
 * ../target/release/nvs(.exe), ../target/debug/nvs(.exe).
 *
 * Outcomes per example:
 *   ok        ran, exit 0, output matches (or no .out file to compare)
 *   MISMATCH  ran, but stdout differs from the .out file
 *   FAIL      non-zero exit
 *   skip      the example's member is not implemented yet (the example starts
 *             with `// requires: unimplemented`), so failure is expected
 *
 * Exit code is non-zero when anything mismatches or fails.
 */

import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const here = path.dirname(fileURLToPath(import.meta.url))
const websiteDir = path.resolve(here, '..')
const repoDir = path.resolve(websiteDir, '..')
const examplesDir = path.join(websiteDir, 'examples')

function findBinary() {
  if (process.env.NVS_BIN && fs.existsSync(process.env.NVS_BIN)) return process.env.NVS_BIN
  const exe = process.platform === 'win32' ? 'nvs.exe' : 'nvs'
  for (const profile of ['release', 'debug']) {
    const candidate = path.join(repoDir, 'target', profile, exe)
    if (fs.existsSync(candidate)) return candidate
  }
  return null
}

const nvs = findBinary()
if (!nvs) {
  console.error('examples:check — no nvs binary found. Build one (`cargo build -p nvs-cli`) or set NVS_BIN.')
  process.exit(1)
}

const files = []
;(function walk(dir) {
  if (!fs.existsSync(dir)) return
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) walk(full)
    else if (entry.name.endsWith('.nvs')) files.push(full)
  }
})(examplesDir)

if (files.length === 0) {
  console.log('examples:check — no examples found under website/examples/.')
  process.exit(0)
}

let ok = 0
let skipped = 0
let failed = 0

for (const file of files.sort()) {
  const rel = path.relative(websiteDir, file).replace(/\\/g, '/')
  const source = fs.readFileSync(file, 'utf8')
  if (/^\/\/\s*requires:\s*unimplemented/m.test(source)) {
    console.log(`  skip      ${rel} (member not implemented yet)`)
    skipped++
    continue
  }

  const result = spawnSync(nvs, ['run', file], { encoding: 'utf8', timeout: 30_000 })
  const outFile = file.replace(/\.nvs$/, '.out')
  const expected = fs.existsSync(outFile) ? fs.readFileSync(outFile, 'utf8') : null

  const normalize = (s) => s.replace(/\r\n/g, '\n').trimEnd()

  if (result.status !== 0) {
    console.error(`  FAIL      ${rel} (exit ${result.status})`)
    if (result.stderr) console.error(result.stderr.trim().split('\n').map((l) => `            ${l}`).join('\n'))
    failed++
  } else if (expected !== null && normalize(result.stdout) !== normalize(expected)) {
    console.error(`  MISMATCH  ${rel}`)
    console.error(`            expected: ${JSON.stringify(normalize(expected))}`)
    console.error(`            got:      ${JSON.stringify(normalize(result.stdout))}`)
    failed++
  } else {
    console.log(`  ok        ${rel}`)
    ok++
  }
}

console.log(`examples:check — ${ok} ok, ${skipped} skipped, ${failed} failed (binary: ${path.relative(repoDir, nvs)})`)
process.exit(failed > 0 ? 1 : 0)
