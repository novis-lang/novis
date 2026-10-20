/**
 * What every diagram under components/diagrams/ shares: the tones it may draw
 * in, the `{count}` holes a label may have, and the scroll range an animated
 * part of it runs in.
 *
 * A diagram's data is a JSON file under src/data/diagrams/, so the words in it
 * are prose that `bun nv site --check prose` reads like a page's. Each data
 * file is typed by the component that draws it.
 */
import { counts, shortCount, type Count } from '~/lib/numbers'

/** The six tones custom.css defines as `--nv-tone-<name>`. */
export const TONES = ['violet', 'crimson', 'orange', 'gold', 'green', 'blue'] as const
export type Tone = (typeof TONES)[number]

let made = 0

/** An id no other diagram in the build has, for an `aria-labelledby` or a radio group. */
export const uid = (prefix: string) => `${prefix}-${++made}`

/** The CSS colour of a tone, for a `--tone` custom property. */
export const tone = (t: Tone) => `var(--nv-tone-${t})`

/**
 * A label with every `{count}` hole filled from the site's counts:
 * `Attack programs · {attacks}` is `Attack programs · 1,400+`. A hole that
 * names no count fails the build, so a renamed count cannot print a brace.
 */
export function label(text: string): string {
  return text.replace(/\{(\w+)\}/g, (_, key: string) => {
    if (!(key in counts)) throw new Error(`diagram label \`${text}\` has a hole \`{${key}}\` that names no count`)
    return shortCount(key as Count)
  })
}

/**
 * The slice of a diagram's scroll-in where its part `at` (0 at the top, 1 at
 * the bottom) animates, as the two ends of an `animation-range` inside
 * `entry`. A part starts a little after its own place in the diagram comes in
 * and takes a fifth of the entry; the last parts end at the entry's end.
 */
export function entryRange(at: number, delay = 0): string {
  const start = Math.min(Math.round((at * 0.75 + delay) * 100), 80)
  return `--from: ${start}%; --to: ${Math.min(start + 20, 100)}%`
}
