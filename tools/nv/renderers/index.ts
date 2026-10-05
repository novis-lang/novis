// Every renderer `bun nv render` runs, in one list. Most turn records into the committed files that
// carry the generated marker, and are added in the same change as the records they read. `spec-tables`
// runs the other way: it renders the spec records from the spec chapters' Markdown tables.
// `WEBSITE_RENDERERS` is the list `bun nv render --website` runs. `website-core` is not in `RENDERERS`,
// because it reads a built binary's registry and CI's docs job runs `nv render --check` without one.
// CI's `reference` job builds the binary and runs `nv render --website --check`.

import type { Renderer } from "../lib/render.ts";
import { goalPlan } from "./goal-plan.ts";
import { recordSchemas } from "./record-schemas.ts";
import { specTables } from "./spec-tables.ts";
import { websiteCore } from "./website-core.ts";
import { websiteGrammar } from "./website-grammar.ts";

export const RENDERERS: Renderer[] = [specTables, websiteGrammar, goalPlan, recordSchemas];

export const WEBSITE_RENDERERS: Renderer[] = [websiteGrammar, websiteCore];
