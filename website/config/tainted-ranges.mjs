/**
 * The dotted underline and the tooltip on each tainted value in a snippet.
 *
 * The grammar that colours a snippet cannot know which values are tainted, so
 * the ranges come from the compiler's language server: `bun nv site --bless`
 * writes a `.tainted` file beside each snippet that has a tainted value, and
 * `tools/nv/cmd/site.ts` § *A snippet* is the home of that file and its format,
 * one `line:column+length type text` row per token. `src/components/Snippet.astro`
 * passes the file's text to `<Code>` as the `tainted` prop, and this plugin
 * gives each range the class `nv-tainted` and a `title` tooltip. The text keeps
 * its colour, and the underline is drawn in that same colour, so it reads in
 * the light and the dark theme alike.
 *
 * `ec.config.mjs` registers the plugin, not `astro.config.ts`: the `<Code>`
 * component reads the options in the Astro config as JSON, and a plugin is
 * code. Nothing here imports Expressive Code, because an annotation may be a
 * plain object, so `tools/nv/test/tainted-ranges.test.ts` runs without the
 * website's packages.
 */

export const TAINTED_CLASS = 'nv-tainted'

/** The tooltip on a tainted token whose semantic-token type is `type`. */
export const taintTitle = (type) =>
  `This ${type === 'property' ? 'property' : 'variable'} is tainted: its value comes from outside the program.`

/**
 * The ranges a `.tainted` file lists, as `{ lineIndex, columnStart, columnEnd,
 * title }` with indexes from 0, checked against `lines`, the lines of the code
 * the page shows. A row that does not cover the text it names throws, so a
 * file the code has moved away from fails the build instead of underlining
 * the wrong characters. `where` names the code block in that error.
 */
export function taintRanges(text, lines, where) {
  const ranges = []
  for (const row of text.replace(/\r\n?/g, '\n').split('\n')) {
    if (row.trim() === '') continue
    const m = /^(\d+):(\d+)\+(\d+) (\S+) (.+)$/.exec(row)
    if (!m) throw new Error(`${where}: \`${row}\` is not a tainted range; run \`bun nv site --bless\` on the snippet`)
    const lineIndex = Number(m[1]) - 1
    const columnStart = Number(m[2]) - 1
    const columnEnd = columnStart + Number(m[3])
    const covered = lines[lineIndex]?.slice(columnStart, columnEnd)
    if (covered !== m[5]) {
      throw new Error(`${where}: the tainted range \`${row}\` covers \`${covered ?? ''}\`; run \`bun nv site --bless\` on the snippet`)
    }
    ranges.push({ lineIndex, columnStart, columnEnd, title: taintTitle(m[4]) })
  }
  return ranges
}

/**
 * One node of a tainted range, given the class and the tooltip. An element,
 * which is a coloured token, takes them itself, so its underline is drawn in
 * its own colour. Bare text is wrapped in a `<span>`.
 */
export function taintNode(node, title) {
  if (node.type === 'element') {
    const had = node.properties?.className
    const className = [...(Array.isArray(had) ? had : had ? [had] : []), TAINTED_CLASS]
    return { ...node, properties: { ...node.properties, className, title } }
  }
  return { type: 'element', tagName: 'span', properties: { className: [TAINTED_CLASS], title }, children: [node] }
}

/** The annotation for one range, as the plain object Expressive Code accepts in place of a class. */
export function taintAnnotation({ columnStart, columnEnd, title }) {
  return {
    name: TAINTED_CLASS,
    inlineRange: { columnStart, columnEnd },
    render: ({ nodesToTransform }) => nodesToTransform.map((node) => taintNode(node, title)),
  }
}

/** The Expressive Code plugin: each code block with a `tainted` prop gets one annotation per range. */
export function taintedRanges() {
  return {
    name: 'Novis tainted ranges',
    baseStyles: `.${TAINTED_CLASS} {
      text-decoration-line: underline;
      text-decoration-style: dotted;
      text-decoration-color: currentColor;
      text-decoration-thickness: 0.1em;
      text-underline-offset: 0.25em;
      cursor: help;
    }`,
    hooks: {
      annotateCode: ({ codeBlock }) => {
        const text = codeBlock.props.tainted
        if (typeof text !== 'string' || text.trim() === '') return
        const lines = codeBlock.getLines()
        const where = codeBlock.parentDocument?.sourceFilePath ?? 'a code block'
        for (const range of taintRanges(text, lines.map((line) => line.text), where)) {
          lines[range.lineIndex].addAnnotation(taintAnnotation(range))
        }
      },
    },
  }
}
