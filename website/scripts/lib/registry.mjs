/**
 * Scanner over crates/nvs-stdlib/src/*.rs answering one question per member:
 * is it registered in the compiler's `Core` registry yet?
 *
 * It reads the `CoreClass { name: …, methods: &[ CoreMethod { name: "y", … } ] }`
 * declarations textually. That shape is load-bearing in the crate (its own
 * tests iterate `CLASSES`), so a textual scan is stable; if the shape ever
 * changes, the sync run reports zero implemented classes, which is loud.
 * (`nvs meta --json` — ADR 0117 — will make this scan obsolete once the
 * registry reports itself.)
 *
 * A class writes its name one of three ways, all resolved here:
 *   name: r"Core\Bytes",                  — inline raw string
 *   name: NAME,                           — a same-file `const NAME: &str = …`
 *   name: nvs_runtime::CARRIER_CLI_TEXT,  — a qualified const from nvs-runtime
 */

import fs from 'node:fs'
import path from 'node:path'

/** All `const X: &str = "Core\…"` declarations in one file, raw or escaped. */
function constMap(text) {
  /** @type {Record<string, string>} */
  const map = {}
  for (const c of text.matchAll(/const\s+([A-Z_][A-Z0-9_]*)\s*:\s*&str\s*=\s*(r?)"([^"]+)"/g)) {
    const value = c[2] ? c[3] : c[3].replace(/\\\\/g, '\\')
    if (value.startsWith('Core\\')) map[c[1]] = value
  }
  return map
}

/**
 * @param {string} stdlibSrcDir absolute path to crates/nvs-stdlib/src
 * @returns {{ implemented: Record<string, string[]>, warnings: string[] }}
 *          class name (`Core\Math`) -> registered member names
 */
export function scanRegistry(stdlibSrcDir) {
  const warnings = []
  /** @type {Record<string, string[]>} */
  const implemented = {}

  // Consts a class name may reference across crates (`nvs_runtime::…`).
  /** @type {Record<string, string>} */
  const externConsts = {}
  const runtimeSrcDir = path.resolve(stdlibSrcDir, '..', '..', 'nvs-runtime', 'src')
  if (fs.existsSync(runtimeSrcDir)) {
    for (const file of fs.readdirSync(runtimeSrcDir)) {
      if (!file.endsWith('.rs')) continue
      Object.assign(externConsts, constMap(fs.readFileSync(path.join(runtimeSrcDir, file), 'utf8')))
    }
  }

  for (const file of fs.readdirSync(stdlibSrcDir)) {
    if (!file.endsWith('.rs')) continue
    const text = fs.readFileSync(path.join(stdlibSrcDir, file), 'utf8')
    const fileConsts = constMap(text)
    // A file-local const may itself alias a qualified one:
    // `const NAME: &str = nvs_runtime::CARRIER_CLI_TEXT;`
    for (const a of text.matchAll(/const\s+([A-Z_][A-Z0-9_]*)\s*:\s*&str\s*=\s*([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z0-9_]+)+)\s*;/g)) {
      const target = externConsts[a[2].split('::').at(-1)]
      if (target && !fileConsts[a[1]]) fileConsts[a[1]] = target
    }

    let from = 0
    for (;;) {
      const at = text.indexOf('CoreClass {', from)
      if (at === -1) break
      // Balance braces from the `{` of `CoreClass {`.
      const open = text.indexOf('{', at)
      let depth = 0
      let end = -1
      for (let i = open; i < text.length; i++) {
        if (text[i] === '{') depth++
        else if (text[i] === '}') {
          depth--
          if (depth === 0) {
            end = i
            break
          }
        }
      }
      if (end === -1) {
        warnings.push(`${file}: unbalanced CoreClass block at ${at}`)
        break
      }
      const block = text.slice(open, end)
      from = end

      let className
      const inline = /name:\s*r"(Core\\[^"]+)"/.exec(block)
      if (inline) {
        className = inline[1]
      } else {
        const ident = /name:\s*([A-Za-z_][A-Za-z0-9_:]*)\s*,/.exec(block)
        if (ident) {
          const bare = ident[1].split('::').at(-1)
          className = ident[1].includes('::') ? externConsts[bare] : fileConsts[bare]
          if (!className) {
            warnings.push(`${file}: CoreClass writes name via ${ident[1]}, which resolves to no "Core\\…" const`)
          }
        }
      }
      if (!className) continue
      const members = implemented[className] ?? (implemented[className] = [])

      // Method names: every `name: "x"` directly followed by `params:` is a
      // member row (static or instance roster alike).
      for (const m of block.matchAll(/name:\s*"([A-Za-z0-9_]+)"\s*,\s*\n?\s*params:/g)) {
        if (!members.includes(m[1])) members.push(m[1])
      }
    }
  }

  if (Object.keys(implemented).length === 0) {
    warnings.push('registry scan found no CoreClass declarations — did the crate layout change?')
  }
  return { implemented, warnings }
}
