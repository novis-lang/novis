// Every renderer `bun nv render` runs, in one list. Each turns records or sources into the committed
// files that carry the generated marker, and is added in the same change as the records it reads.
// `core-classes` renders the `Core` class table from the rank each class's const states.
// `WEBSITE_RENDERERS` is the list `bun nv render --website` runs. `website-core` and `website-llms`
// are not in `RENDERERS`, because they read a built binary's registry and CI's docs job runs
// `nv render --check` without one. CI's `reference` job builds the binary and runs
// `nv render --website --check`.

import type { Renderer } from "../lib/render.ts";
import { coreClasses } from "./core-classes.ts";
import { langChapters } from "./lang-chapters.ts";
import { recordSchemas } from "./record-schemas.ts";
import { websiteCompare } from "./website-compare.ts";
import { websiteCore } from "./website-core.ts";
import { websiteGrammar } from "./website-grammar.ts";
import { websiteLlms } from "./website-llms.ts";
import { websiteNumbers } from "./website-numbers.ts";

export const RENDERERS: Renderer[] = [websiteGrammar, websiteCompare, websiteNumbers, recordSchemas, langChapters, coreClasses];

export const WEBSITE_RENDERERS: Renderer[] = [websiteGrammar, websiteCompare, websiteNumbers, websiteCore, websiteLlms];
