/**
 * The registry-documentation seam (ADR 0117): ask the `nvs` binary for its own
 * registry as JSON via `nvs meta --json`, and fail soft — a missing binary or a
 * toolchain that predates the subcommand simply means "no registry docs yet",
 * and every consumer falls back to the spec, field by field.
 *
 * Expected shape (the command owns the contract; unknown fields are ignored,
 * and a field with nothing written is absent rather than empty):
 *
 *   { "classes": [ { "name": "Core\\Str",
 *       "members": [ { "name": "length",
 *         "doc": {
 *           "short":  "one or two sentences, inline markdown allowed",
 *           "params": [ { "name": "s", "desc": "…",
 *                         "shape": [ { "key": "pretty", "type": "bool", "desc": "…" } ] } ],
 *           "return": "…",
 *           "errors": [ { "error": "Core\\Error\\…", "desc": "…" } ]
 *         } } ],
 *       "constants": [ { "name": "PI", "doc": "one sentence" } ] } ],
 *     "enums": [ { "name": "Core\\Order",
 *       "doc": { "short": "…", "cases": [ { "name": "Asc", "desc": "…" } ] } } ] }
 *
 * `enums` is top-level because the registry's roster is — an enum has no owner
 * class there; the spec's `Enums:` lines and `spec-overrides.mjs`'s
 * `enumOwners` are what attach one to a class page.
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
 * @returns {{ docs: Map<string, object>, enumDocs: Map<string, { short: string, cases: Map<string, string> }>,
 *             constDocs: Map<string, string>, note: string }}
 * — `docs` maps "Core\\Str::length" to that member's `doc` object; `enumDocs`
 * maps "Core\\Order" to its short description and its cases' descriptions keyed
 * by case name; `constDocs` maps "Core\\Math::PI" to its one-sentence card;
 * `note` says where the data came from (or why there is none), for the sync
 * report.
 */
export function loadRegistryDocs(repoDir) {
  const docs = new Map()
  const enumDocs = new Map()
  const constDocs = new Map()
  const none = (note) => ({ docs, enumDocs, constDocs, note })
  const nvs = findBinary(repoDir)
  if (!nvs) return none('no nvs binary — spec only')

  const run = spawnSync(nvs, ['meta', '--json'], { encoding: 'utf8', timeout: 30_000 })
  if (run.error || run.status !== 0) {
    return none('nvs meta not available in this toolchain — spec only')
  }

  let parsed
  try {
    parsed = JSON.parse(run.stdout)
  } catch {
    return none('nvs meta produced unparseable output — spec only')
  }

  for (const cls of parsed?.classes ?? []) {
    for (const m of cls?.members ?? []) {
      if (m?.doc && typeof m.name === 'string') docs.set(`${cls.name}::${m.name}`, m.doc)
    }
    for (const c of cls?.constants ?? []) {
      if (typeof c?.doc === 'string' && c.doc && typeof c.name === 'string') {
        constDocs.set(`${cls.name}::${c.name}`, c.doc)
      }
    }
  }
  for (const e of parsed?.enums ?? []) {
    if (!e?.doc || typeof e.name !== 'string') continue
    const cases = new Map()
    for (const c of Array.isArray(e.doc.cases) ? e.doc.cases : []) {
      if (typeof c?.name === 'string' && typeof c.desc === 'string' && c.desc) cases.set(c.name, c.desc)
    }
    enumDocs.set(e.name, { short: typeof e.doc.short === 'string' ? e.doc.short : '', cases })
  }
  return none(
    `nvs meta — ${docs.size} member(s), ${enumDocs.size} enum(s), ${constDocs.size} constant(s) carry registry docs`,
  )
}
