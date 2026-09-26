// Every renderer `bun nv render` runs, in one list. Each turns records into the committed files that
// carry the generated marker. A renderer is added in the same change as the records it reads.
// `WEBSITE_RENDERERS` is the list `bun nv render --website` runs. `website-core` is in it alone,
// because it reads a built binary's registry and CI's docs job runs `nv render --check` without one.

import type { Renderer } from "../lib/render.ts";
import { goalPlan } from "./goal-plan.ts";
import { websiteCore } from "./website-core.ts";
import { websiteRules } from "./website-rules.ts";

export const RENDERERS: Renderer[] = [websiteRules, goalPlan];

export const WEBSITE_RENDERERS: Renderer[] = [websiteRules, websiteCore];
