/**
 * How a replay's completion list narrows as letters are typed.
 *
 * The language server sends a list once, where the editor asks for it, and
 * the editor narrows that list itself as more letters are typed. A replay
 * records only the server's lists, so the page imitates the narrowing, and
 * `bun nv site` uses this same function to check that an `accept LABEL` step
 * accepts the row the page shows as selected. `tools/nv/cmd/site.ts`
 * § *A snippet* is the home of the replay format.
 *
 * A row stays in the list when the typed letters appear in its label in the
 * same order, upper or lower case alike. Rows whose label starts with the
 * typed text come first, then rows whose first matched letter is earlier,
 * then rows whose matched letters are closer together. Rows that tie keep the
 * server's order. The first row is the selected one. This is close to what
 * VS Code does, and is not its exact behaviour.
 *
 * Nothing here imports a package, so `tools/nv/test/` runs it without the
 * website's packages.
 */

/**
 * Where each letter of `typed` is in `label`, from the left, or null when the
 * letters do not all appear in order.
 */
export function matchedLetters(label, typed) {
  const lower = label.toLowerCase()
  const at = []
  let from = 0
  for (const letter of typed.toLowerCase()) {
    const i = lower.indexOf(letter, from)
    if (i < 0) return null
    at.push(i)
    from = i + 1
  }
  return at
}

/**
 * The rows of a list whose labels are `labels` that stay when `typed` has
 * been typed, best first, as `{ index, matched }`: the row's place in
 * `labels`, and where each typed letter is in its label.
 */
export function narrow(labels, typed) {
  const kept = []
  labels.forEach((label, index) => {
    const matched = matchedLetters(label, typed)
    if (matched !== null) kept.push({ index, matched, prefix: label.toLowerCase().startsWith(typed.toLowerCase()) })
  })
  const first = (row) => (row.matched.length ? row.matched[0] : 0)
  const span = (row) => (row.matched.length ? row.matched[row.matched.length - 1] - row.matched[0] : 0)
  kept.sort((a, b) => Number(b.prefix) - Number(a.prefix) || first(a) - first(b) || span(a) - span(b) || a.index - b.index)
  return kept.map(({ index, matched }) => ({ index, matched }))
}
