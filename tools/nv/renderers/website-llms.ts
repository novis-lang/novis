// The website's `/llms-full.txt`: `website/public/llms-full.txt`, the one-file reference to everything
// Novis has, which Astro copies into the build as it is. The text is `bun nv reference`'s `build`, from
// `nvs meta --json` and the chapters under `docs/reference/`; that command runs every example in it.
// It reads a built binary, so it is in `bun nv render --website` alone, as `website-core` is, and it
// reads the same binary `website-core` does, so one render publishes one registry.
//
// The file is plain text, so it carries no generated marker: a reader of the file has only the file.

import { build, registry } from "../cmd/reference.ts";
import type { Renderer } from "../lib/render.ts";
import { binary } from "./website-core.ts";

export const LLMS_FULL = "website/public/llms-full.txt";

export const websiteLlms: Renderer = {
  name: "website-llms",
  render: async (root) => [{ path: LLMS_FULL, text: build(await registry(binary(root))) }],
};
