// Every renderer `bun nv render` runs, in one list. Each turns records into the committed files that
// carry the generated marker. A renderer is added in the same change as the records it reads.

import type { Renderer } from "../lib/render.ts";

export const RENDERERS: Renderer[] = [];
