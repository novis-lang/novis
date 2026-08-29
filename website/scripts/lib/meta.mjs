/**
 * The registry-documentation seam (ADR 0117): ask the `nvs` binary for its own
 * registry as JSON via `nvs meta --json`, and fail soft — a missing binary or a
 * toolchain that predates the subcommand simply means "no registry docs yet",
 * and every consumer falls back to the spec, field by field.
 *
 * Expected shape (the command owns the contract; unknown fields are ignored):
 *
 *   { "classes": [ { "name": "Core\\Str", "members": [ { "name": "length",
 *       "doc": {
 *         "short":  "one or two sentences, inline markdown allowed",
 *         "params": [ { "name": "s", "desc": "…",
 *                       "shape": [ { "key": "pretty", "type": "bool", "desc": "…" } ] } ],
 *         "return": "…",
 *         "errors": [ { "error": "Core\\Error\\…", "desc": "…" } ]
 *       } } ] } ] }
 */

import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

function findBinary(repoDir) {
  if (process.env.NVS_BIN && fs.existsSync(process.env.NVS_BIN)) return process.env.NVS_BIN
  const exe = process.platform === 'win32' ? 'nvs.exe' : 'nvs'
  for (const profile of ['release', 'debug']) {
    const candidate = path.join(repoDir, 'target', profile, exe)
    if (fs.existsSync(candidate)) return candidate
  }
  return null
}

/**
 * @returns {{ docs: Map<string, object>, note: string }} — `docs` maps
 * "Core\\Str::length" to that member's `doc` object; `note` says where the
 * data came from (or why there is none), for the sync report.
 */
export function loadRegistryDocs(repoDir) {
  const docs = new Map()
  const nvs = findBinary(repoDir)
  if (!nvs) return { docs, note: 'no nvs binary — spec only' }

  const run = spawnSync(nvs, ['meta', '--json'], { encoding: 'utf8', timeout: 30_000 })
  if (run.error || run.status !== 0) {
    return { docs, note: `nvs meta not available in this toolchain — spec only` }
  }

  let parsed
  try {
    parsed = JSON.parse(run.stdout)
  } catch {
    return { docs, note: 'nvs meta produced unparseable output — spec only' }
  }

  for (const cls of parsed?.classes ?? []) {
    for (const m of cls?.members ?? []) {
      if (m?.doc && typeof m.name === 'string') docs.set(`${cls.name}::${m.name}`, m.doc)
    }
  }
  return { docs, note: `nvs meta — ${docs.size} member(s) carry registry docs` }
}
