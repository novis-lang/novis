/**
 * Parser for docs/spec/01-core-library.md — the file the repo declares
 * authoritative for every `Core` signature.
 *
 * The spec is written for humans, so this parser is deliberately layered:
 *
 *  1. Well-formed member tables (`| Member | Signature | … |`) whose signature
 *     cell parses as `name(params): return` become full member records.
 *  2. Roster tables (`| Class/Type | Surface/Owns/Members | … |`) and
 *     bullet-roster sections become *surface* class records — a class the
 *     sidebar shows and a page describes, without per-member pages yet.
 *  3. Everything else lands in `unparsed` with a warning, so a spec edit that
 *     this parser cannot read is a visible sync-time report, never silence.
 *
 * Hand-maintained corrections live in ../spec-overrides.mjs, NOT here.
 */

// ---------------------------------------------------------------- utilities

/** Unescape the markdown-table escapes the spec uses inside cells. */
function unescapeCell(s) {
  return s.replace(/\\\|/g, '|').trim()
}

/** Split a markdown table row into cells, honoring `\|` escapes. */
function splitRow(line) {
  const cells = []
  let cur = ''
  for (let i = 0; i < line.length; i++) {
    const ch = line[i]
    if (ch === '\\' && line[i + 1] === '|') {
      cur += '|'
      i++
    } else if (ch === '|') {
      cells.push(cur)
      cur = ''
    } else {
      cur += ch
    }
  }
  cells.push(cur)
  // A well-formed row is `| a | b |` -> ['', ' a ', ' b ', ''].
  return cells.slice(1, -1).map((c) => c.trim())
}

/** Split `s` at top-level commas (ignoring commas inside (), {}, <>, []). */
function splitTopLevel(s, sep = ',') {
  const parts = []
  let depth = 0
  let cur = ''
  for (const ch of s) {
    if ('({<['.includes(ch)) depth++
    else if (')}>]'.includes(ch)) depth--
    if (ch === sep && depth === 0) {
      parts.push(cur)
      cur = ''
    } else {
      cur += ch
    }
  }
  if (cur.trim() !== '') parts.push(cur)
  return parts.map((p) => p.trim())
}

/** Find the index of the `)` matching the `(` at `open`. -1 when unbalanced. */
function matchParen(s, open) {
  let depth = 0
  for (let i = open; i < s.length; i++) {
    if (s[i] === '(') depth++
    else if (s[i] === ')') {
      depth--
      if (depth === 0) return i
    }
  }
  return -1
}

// ------------------------------------------------------- signature parsing

/**
 * Parse one options-bag literal `{a?: int, b?: bool}` or `{hour?, minute?}`.
 * @returns {{name: string, type: string}[]}
 */
function parseOptions(inner) {
  if (inner.trim() === '') return []
  return splitTopLevel(inner).map((part) => {
    const m = /^([A-Za-z_][A-Za-z0-9_]*)\??\s*(?::\s*(.+))?$/.exec(part.trim())
    if (!m) return { name: part.trim(), type: '' }
    return { name: m[1], type: (m[2] ?? '').trim() }
  })
}

/**
 * Parse one parameter: `type $name`, `type $name = default`, `T ...$values`,
 * or an options bag `{…}`.
 */
function parseParam(raw) {
  const s = raw.trim()
  // A bare `{…}` is an options bag; `{…} $name` is a shape-typed parameter.
  if (s.startsWith('{') && !/\}\s*\$/.test(s)) {
    const inner = s.slice(1, s.lastIndexOf('}'))
    return { kind: 'options', options: parseOptions(inner) }
  }
  let rest = s
  let def = null
  const eq = rest.indexOf('=')
  if (eq !== -1) {
    def = rest.slice(eq + 1).trim()
    rest = rest.slice(0, eq).trim()
  }
  const variadic = rest.includes('...')
  rest = rest.replace('...', ' ').replace(/\s+/g, ' ').trim()
  const dollar = rest.lastIndexOf('$')
  if (dollar === -1) return null
  const name = rest.slice(dollar + 1).trim()
  const type = rest.slice(0, dollar).trim()
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) return null
  return { kind: 'param', name, type, default: def, optional: def !== null, variadic }
}

/**
 * Parse a signature like:
 *   `length(string $s): uint`
 *   `$i->in(Zone $zone): DateTime`
 *   `Uri::parse(string $uri): Uri`
 *   `decodeAs<T>(string $json, {maxDepth?: uint}): T`
 * @returns {null | {
 *   name: string, classPrefix: string|null, receiver: string|null,
 *   generics: string|null, params: any[], options: any[], returnType: string,
 *   signature: string,
 * }}
 */
export function parseSignature(raw) {
  let s = unescapeCell(raw).replace(/`/g, '').trim()
  // Trailing prose after the closing paren+return, e.g. "(and the rest)" — cut
  // anything after the return type that starts a new sentence in parens.
  let receiver = null
  let classPrefix = null

  let m = /^\$([A-Za-z_][A-Za-z0-9_]*)->/.exec(s)
  if (m) {
    receiver = m[1]
    s = s.slice(m[0].length)
  } else {
    m = /^([A-Za-z_][A-Za-z0-9_\\]*)::/.exec(s)
    if (m) {
      classPrefix = m[1]
      s = s.slice(m[0].length)
    }
  }

  const head = /^([A-Za-z_][A-Za-z0-9_]*)\s*(<[^>]*>)?\s*\(/.exec(s)
  if (!head) return null
  const name = head[1]
  const generics = head[2] ?? null
  const open = s.indexOf('(', head[1].length)
  const close = matchParen(s, open)
  if (close === -1) return null
  const paramsRaw = s.slice(open + 1, close)
  const after = s.slice(close + 1).trim()
  const ret = /^:\s*([^—]+?)(?:\s*[—(].*)?$/.exec(after)
  const returnType = ret ? ret[1].trim() : after === '' ? 'void' : null
  if (returnType === null) return null

  const params = []
  let options = []
  for (const part of splitTopLevel(paramsRaw)) {
    if (part === '') continue
    const p = parseParam(part)
    if (p === null) return null
    if (p.kind === 'options') options = p.options
    else params.push(p)
  }

  const sigParams = splitTopLevel(paramsRaw).join(', ')
  return {
    name,
    classPrefix,
    receiver,
    generics,
    params,
    options,
    returnType,
    signature: `${name}${generics ?? ''}(${sigParams}): ${returnType}`,
  }
}

// ------------------------------------------------------------ spec walking

/** All `Core\X[\Y]` tokens in a line, longest form first. */
function coreNamesIn(line) {
  return [...line.matchAll(/Core\\[A-Za-z][A-Za-z0-9_]*(?:\\[A-Za-z][A-Za-z0-9_]*)?/g)].map((m) => m[0])
}

function classIdOf(name) {
  return name.replace(/^Core\\/, '').replace(/\\/g, '.')
}

/**
 * @param {string} specText the raw markdown of 01-core-library.md
 * @param {object} overrides from spec-overrides.mjs
 */
export function parseSpec(specText, overrides) {
  const warnings = []
  /** @type {Map<string, any>} */
  const classes = new Map()

  function classRecord(name) {
    const clean = name.replace(/<[^>]*>/, '') // ObjectMap<K, V> -> ObjectMap
    const full = clean.startsWith('Core\\') ? clean : `Core\\${clean}`
    if (!classes.has(full)) {
      classes.set(full, {
        id: classIdOf(full),
        name: full,
        section: '',
        summary: '',
        surface: '',
        adrs: [],
        enums: [],
        constants: [],
        members: [],
        unparsed: [],
      })
    }
    return classes.get(full)
  }

  /** Resolve a bare prefix like `Uri`, `Duration`, `Zone::` against the
   * current section namespace, e.g. inside `Core\Time` -> `Core\Time\Duration`. */
  function resolvePrefix(prefix, nsStack) {
    if (prefix.startsWith('Core\\')) return prefix
    for (const ns of nsStack) {
      const candidate = `${ns}\\${prefix}`
      if (classes.has(candidate)) return candidate
    }
    // `Core\<prefix>` referenced from a section listing it in its heading.
    if (classes.has(`Core\\${prefix}`)) return `Core\\${prefix}`
    // Namespace-local new class (e.g. Core\Db\Rows on first sight): resolve
    // against the section's own namespace, never a sub-heading's class.
    const sectionNs = nsStack.length >= 2 ? nsStack[nsStack.length - 2] : nsStack[0]
    if (sectionNs && sectionNs !== 'Core') return `${sectionNs}\\${prefix}`
    return `Core\\${prefix}`
  }

  const lines = specText.split(/\r?\n/)
  let currentClasses = [] // classes named by the innermost heading
  let currentSection = ''
  let nsStack = [] // namespaces for prefix resolution, innermost first
  let summaryTarget = null
  let collectingSummary = false

  const rowOverrides = overrides.rows ?? {}

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]

    // ---- headings drive class context
    const h = /^(#{1,3})\s+(.*)$/.exec(line)
    if (h) {
      const [, hashes, text] = h
      const named = coreNamesIn(text)
      if (hashes === '##') {
        currentSection = text.trim()
        currentClasses = named.map((n) => classRecord(n))
        for (const c of currentClasses) if (!c.section) c.section = currentSection
        nsStack = named.length > 0 ? [named[0], 'Core'] : ['Core']
        // Only start summary collection for single-class sections; for
        // multi-class ones a shared summary would mislead.
        summaryTarget = currentClasses.length >= 1 ? currentClasses[0] : null
        collectingSummary = summaryTarget !== null && summaryTarget.summary === ''
      } else if (hashes === '###') {
        if (named.length > 0) {
          currentClasses = named.map((n) => classRecord(n))
          for (const c of currentClasses) if (!c.section) c.section = currentSection
          nsStack = [named[0], ...nsStack.filter((n) => n !== named[0])]
        }
        collectingSummary = false
      } else {
        collectingSummary = false
      }
      continue
    }

    // ---- summary: prose between a class's `##` heading and its first table/subheading
    if (collectingSummary && summaryTarget) {
      if (line.startsWith('|') || line.startsWith('```')) {
        collectingSummary = false
      } else {
        summaryTarget.summary += line + '\n'
      }
    }

    // ---- enum / constant roster lines
    const enumLine = /^Enums:\s*(.*)$/.exec(line)
    if (enumLine && currentClasses[0]) {
      let acc = enumLine[1]
      while (!/\.\s*$/.test(acc) && i + 1 < lines.length && lines[i + 1].trim() !== '') {
        acc += ' ' + lines[++i].trim()
      }
      for (const em of acc.matchAll(/`([A-Za-z\\]+)\s*\{\s*([^}]*)\}`/g)) {
        const cases = em[2].split(',').map((c) => c.trim().replace(/….*$/, '…'))
        const ownerName = overrides.enumOwners?.[em[1]]
        const owner = ownerName ? classRecord(ownerName) : currentClasses[0]
        owner.enums.push({ name: em[1], cases, raw: em[0].replace(/`/g, '') })
      }
      continue
    }
    const constLine = /^Constants?:\s*(.*)$/.exec(line)
    if (constLine && currentClasses[0]) {
      let acc = constLine[1]
      while (!/[.—]\s*$/.test(acc) && i + 1 < lines.length && lines[i + 1].trim() !== '' && !lines[i + 1].startsWith('#')) {
        acc += ' ' + lines[++i].trim()
      }
      const stop = acc.search(/—|\breplacing\b/)
      const rosterText = stop === -1 ? acc : acc.slice(0, stop)
      for (const cm of rosterText.matchAll(/`([A-Za-z_][A-Za-z0-9_:\\]*)`/g)) {
        currentClasses[0].constants.push(cm[1].replace(/^.*::/, ''))
      }
      continue
    }

    // ---- bullet rosters: `- \`Core\Request\`: everything the class covers …`
    const bullet = /^-\s+`(Core\\[A-Za-z][A-Za-z0-9_\\]*)`:\s*(.*)$/.exec(line)
    if (bullet) {
      const rec = classRecord(bullet[1])
      if (!rec.section) rec.section = currentSection
      let text = bullet[2]
      while (i + 1 < lines.length && /^\s+\S/.test(lines[i + 1])) {
        text += ' ' + lines[++i].trim()
      }
      if (!rec.surface) rec.surface = text.trim()
      continue
    }

    // ---- tables
    if (!line.startsWith('|')) continue
    const header = splitRow(line).map((c) => c.replace(/\*/g, '').trim())
    if (!lines[i + 1] || !/^\|[\s:-]*-/.test(lines[i + 1])) continue // not a table header
    const rows = []
    let j = i + 2
    for (; j < lines.length && lines[j].startsWith('|'); j++) rows.push(splitRow(lines[j]))
    i = j - 1

    const kind0 = header[0]?.toLowerCase()
    const isMemberTable = kind0 === 'member'
    const isRosterTable = kind0 === 'class' || kind0 === 'type'
    if (!isMemberTable && !isRosterTable) continue // e.g. the strtotime mapping table

    if (isRosterTable) {
      for (const row of rows) {
        const cellNames = coreNamesIn(row[0])
        const names = cellNames.length > 0 ? cellNames : [resolvePrefix(row[0].replace(/`/g, '').replace(/<[^>]*>/g, '').trim(), nsStack)]
        for (const n of names) {
          const rec = classRecord(n)
          if (!rec.section) rec.section = currentSection
          if (!rec.surface) rec.surface = unescapeCell(row[1] ?? '')
          const refCell = row[2] ?? ''
          for (const am of refCell.matchAll(/\b(\d{4})\b/g)) {
            if (!rec.adrs.includes(am[1])) rec.adrs.push(am[1])
          }
        }
      }
      continue
    }

    // Member table. Columns: Member | Signature | (Replaces|Notes|Answers) | Q?
    const col2 = (header[2] ?? '').toLowerCase()
    for (const row of rows) {
      const memberCellRaw = row[0] ?? ''
      const sigCell = row[1] ?? ''
      const replaces = col2 === 'replaces' ? unescapeCell(row[2] ?? '') : ''
      const notes = col2 !== 'replaces' ? unescapeCell(row[2] ?? '') : ''
      const qualifier = unescapeCell(row[3] ?? (col2 === 'q' ? row[2] : '') ?? '').replace(/\*/g, '')

      const key = memberCellRaw.replace(/`/g, '').trim()
      const rowOv = rowOverrides[key]
      if (rowOv?.skip) continue

      // Names in the member cell: `a` / `b` or `a` `b` `c`
      const names = [...memberCellRaw.matchAll(/`([^`]+)`/g)].map((m) => m[1])
      if (names.length === 0) names.push(key)

      const parsed = parseSignature(sigCell)
      if (!parsed) {
        const target = currentClasses[0] ?? classRecord('Core\\Unknown')
        target.unparsed.push({ memberCell: key, signatureCell: unescapeCell(sigCell), section: currentSection })
        if (!rowOv?.silent) warnings.push(`unparsed row in "${currentSection}": ${key} | ${sigCell.slice(0, 80)}`)
        continue
      }

      // Which class does this member belong to?
      let owner
      const firstName = names[0]
      const prefixed = /^([A-Za-z\\]+)::/.exec(firstName)
      if (rowOv?.class) owner = classRecord(rowOv.class)
      else if (parsed.classPrefix) owner = classRecord(resolvePrefix(parsed.classPrefix, nsStack))
      else if (prefixed) owner = classRecord(resolvePrefix(prefixed[1], nsStack))
      else if (parsed.receiver && overrides.receivers?.[parsed.receiver]) owner = classRecord(overrides.receivers[parsed.receiver])
      else if (currentClasses.length === 1) owner = currentClasses[0]
      else if (currentClasses.length > 1) {
        warnings.push(`ambiguous class for "${firstName}" in "${currentSection}" — add a row override`)
        continue
      } else {
        warnings.push(`no class context for "${firstName}" in "${currentSection}"`)
        continue
      }

      const isStatic = parsed.receiver === null

      const push = (name, sig) => {
        if (owner.members.some((mm) => mm.name === name)) return
        owner.members.push({
          id: `${owner.id}.${name}`,
          name,
          static: isStatic,
          receiver: sig.receiver,
          generics: sig.generics,
          signature: sig.signature.replace(new RegExp(`^${sig.name}`), name),
          params: sig.params,
          options: sig.options,
          returnType: sig.returnType,
          replaces,
          notes,
          qualifier,
          section: currentSection,
        })
      }

      // First name always gets the parsed signature.
      const bare = (n) => n.replace(/^.*::/, '').replace(/^\$[A-Za-z0-9_]*->/, '')
      push(bare(firstName), parsed)

      // Further names: clone when the cell is space-separated (`sin` `cos` `tan`)
      // or explicitly allowed; otherwise they need an override member.
      const spaceSeparated = !memberCellRaw.includes('/')
      for (const extra of names.slice(1)) {
        const extraBare = bare(extra)
        if (spaceSeparated || rowOv?.cloneAlso?.includes(extraBare)) {
          push(extraBare, parsed)
        } else if (!rowOv?.silent && !(overrides.members ?? []).some((om) => om.class === owner.name && om.name === extraBare)) {
          warnings.push(`"${key}" names ${extraBare} but only ${bare(firstName)}'s signature is stated — add an override member or cloneAlso`)
        }
      }
    }
  }

  // ---- hand-maintained additional members
  for (const om of overrides.members ?? []) {
    const owner = classRecord(om.class)
    const parsed = parseSignature(om.signature)
    if (!parsed) {
      warnings.push(`override member ${om.class}::${om.name}: signature does not parse: ${om.signature}`)
      continue
    }
    if (owner.members.some((mm) => mm.name === parsed.name)) continue
    owner.members.push({
      id: `${owner.id}.${parsed.name}`,
      name: parsed.name,
      static: om.static ?? parsed.receiver === null,
      receiver: parsed.receiver,
      generics: parsed.generics,
      signature: parsed.signature,
      params: parsed.params,
      options: parsed.options,
      returnType: parsed.returnType,
      replaces: om.replaces ?? '',
      notes: om.notes ?? '',
      qualifier: om.qualifier ?? '',
      section: om.section ?? owner.section,
      fromOverride: true,
    })
  }

  // ---- class-level extras from overrides
  for (const [name, extra] of Object.entries(overrides.classes ?? {})) {
    const rec = classRecord(name)
    if (extra.summary && !rec.summary.trim()) rec.summary = extra.summary
    if (extra.hide) rec.hidden = true
    for (const c of extra.constants ?? []) {
      if (!rec.constants.includes(c)) rec.constants.push(c)
    }
    for (const e of extra.enums ?? []) {
      if (!rec.enums.some((x) => x.name === e.name)) rec.enums.push(e)
    }
  }

  // Trim summaries.
  for (const rec of classes.values()) {
    rec.summary = rec.summary.trim()
  }

  const list = [...classes.values()].filter((c) => !c.hidden && c.name !== 'Core\\Unknown')
  list.sort((a, b) => a.id.localeCompare(b.id))
  return { classes: list, warnings }
}
