/**
 * Scanner over crates/nvs-stdlib/src/*.rs answering one question per member:
 * is it registered in the compiler's `Core` registry yet?
 *
 * It reads the `CoreClass { name: r"Core\X", methods: &[ CoreMethod { name: "y", … } ] }`
 * declarations textually. That shape is load-bearing in the crate (its own
 * tests iterate `CLASSES`), so a textual scan is stable; if the shape ever
 * changes, the sync run reports zero implemented classes, which is loud.
 */

import fs from 'node:fs'
import path from 'node:path'

/**
 * @param {string} stdlibSrcDir absolute path to crates/nvs-stdlib/src
 * @returns {{ implemented: Record<string, string[]>, warnings: string[] }}
 *          class name (`Core\Math`) -> registered member names
 */
export function scanRegistry(stdlibSrcDir) {
  const warnings = []
  /** @type {Record<string, string[]>} */
  const implemented = {}

  for (const file of fs.readdirSync(stdlibSrcDir)) {
    if (!file.endsWith('.rs')) continue
    const text = fs.readFileSync(path.join(stdlibSrcDir, file), 'utf8')

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

      const nameMatch = /name:\s*r"(Core\\[^"]+)"/.exec(block)
      if (!nameMatch) continue
      const className = nameMatch[1]
      const members = implemented[className] ?? (implemented[className] = [])

      // Method names: every `name: "x"` inside the block (class name uses r"…",
      // so plain-quoted names are member rows and options; member rows are the
      // ones directly followed by `params:`.
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
