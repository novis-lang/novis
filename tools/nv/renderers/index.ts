// Every renderer `bun nv render` runs, in one list. Each turns records into the committed files that
// carry the generated marker. A renderer is added in the same change as the records it reads.
// `WEBSITE_RENDERERS` is the part of the list `bun nv render --website` narrows to.

import type { Renderer } from "../lib/render.ts";
import { websiteRules } from "./website-rules.ts";

export const WEBSITE_RENDERERS: Renderer[] = [websiteRules];

export const RENDERERS: Renderer[] = [...WEBSITE_RENDERERS];
