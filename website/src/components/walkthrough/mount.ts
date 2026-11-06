/**
 * Starts walkthrough stages. A stage names its scene module in `data-scene` (the file name under
 * `scenes/`, without `.ts`), and the module is loaded only when its stage starts, as its own chunk.
 * So a page pays for the engine and a scene only where a stage is on it, and the front page's
 * carousel pays for one only when a visitor presses play.
 *
 * - `mountAll()` starts every stage in the document that is not `data-lazy`. Stage.astro calls it.
 * - `mount(root)` starts one stage and returns it once its first frame is drawn.
 */
import type { Stage } from './engine/stage'

const scenes = import.meta.glob<{ start(root: HTMLElement): void }>('./scenes/*.ts')

export async function mount(root: HTMLElement): Promise<Stage> {
  const name = root.dataset.scene
  const load = scenes[`./scenes/${name}.ts`]
  if (!load) throw new Error(`walkthrough: no scene module scenes/${name}.ts`)
  const scene = await load()
  scene.start(root)
  return (root as HTMLElement & { wt: Stage }).wt
}

export function mountAll(): void {
  for (const root of document.querySelectorAll<HTMLElement>('.wt[data-scene]:not([data-lazy])')) void mount(root)
}
