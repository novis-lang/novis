// The viewer's build: `src/main.tsx` and the CSS it imports, bundled by Bun into one JavaScript file and
// one CSS file. `bun nv viewer` writes them to `dist/`, and `nvs-cli` embeds them
// (`crates/nvs-cli/src/viewer.rs`).
//
// Bun's own JSX transform has only the React-style runtimes, so the plugin below runs every `.tsx`
// file through Babel with Solid's preset, which compiles JSX to DOM calls, and TypeScript's, which
// strips the types. A `.ts` file has no JSX and Bun reads it directly.
//
// The output is minified with no source map, so it names no path of the machine that built it, and two
// builds of one tree give the same bytes (`tools/nv/test/viewer.test.ts`).

import { transformAsync } from "@babel/core";
import solid from "babel-preset-solid";
import typescript from "@babel/preset-typescript";
import type { BunPlugin } from "bun";
import { join } from "node:path";

/** The viewer's folder. */
export const VIEWER = import.meta.dir;
/** The one entry point. */
export const ENTRY = join(VIEWER, "src", "main.tsx");
/** Where the two committed files are. */
export const DIST = join(VIEWER, "dist");

/** The two files a build gives, by their name under `dist/`. */
export interface Bundle {
  "viewer.js": Uint8Array;
  "viewer.css": Uint8Array;
}

const solidJsx: BunPlugin = {
  name: "solid-jsx",
  setup(build) {
    build.onLoad({ filter: /\.tsx$/ }, async ({ path }) => {
      const out = await transformAsync(await Bun.file(path).text(), {
        filename: path,
        babelrc: false,
        configFile: false,
        sourceMaps: false,
        presets: [[solid, { generate: "dom" }], [typescript, { onlyRemoveTypeImports: true }]],
      });
      if (!out?.code) throw new Error(`viewer: Babel gave no code for ${path}`);
      return { contents: out.code, loader: "js" };
    });
  },
};

/** Builds the bundle in memory. `minify` is off only for `--dev`, where a readable stack helps. */
export async function build(minify = true): Promise<Bundle> {
  const result = await Bun.build({
    entrypoints: [ENTRY],
    target: "browser",
    format: "esm",
    minify,
    sourcemap: "none",
    plugins: [solidJsx],
    throw: false,
  });
  if (!result.success) throw new Error(`viewer: the build failed\n${result.logs.map(String).join("\n")}`);
  const pick = async (ext: string) => {
    const found = result.outputs.filter((o) => o.path.endsWith(ext));
    if (found.length !== 1) throw new Error(`viewer: the build gave ${found.length} ${ext} file(s), want 1`);
    return new Uint8Array(await found[0]!.arrayBuffer());
  };
  return { "viewer.js": await pick(".js"), "viewer.css": await pick(".css") };
}
