// Every renderer `bun nv render` runs, in one list. Each turns records into the committed files that
// carry the generated marker. A renderer is added in the same change as the records it reads.
// `WEBSITE_RENDERERS` is the list `bun nv render --website` runs. `website-core` is not in `RENDERERS`,
// because it reads a built binary's registry and CI's docs job runs `nv render --check` without one.
// CI's `reference` job builds the binary and runs `nv render --website --check`.

import type { Renderer } from "../lib/render.ts";
import { goalPlan } from "./goal-plan.ts";
import { recordSchemas } from "./record-schemas.ts";
import { websiteCore } from "./website-core.ts";
import { websiteGrammar } from "./website-grammar.ts";

export const RENDERERS: Renderer[] = [websiteGrammar, goalPlan, recordSchemas];

export const WEBSITE_RENDERERS: Renderer[] = [websiteGrammar, websiteCore];
