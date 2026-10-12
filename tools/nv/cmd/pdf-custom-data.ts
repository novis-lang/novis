// `bun nv pdf-custom-data`: VS Code's HTML and CSS custom data for `Novis\Pdf`, from the subset
// table (`rule:core-classes/pdf-css-subset-table`).
//
//     bun nv pdf-custom-data            write editors/vscode/data/pdf.html-data.json and pdf.css-data.json
//     bun nv pdf-custom-data --check    exit 1 naming each file that differs from what it would write
//
// What goes in the files is `editors/vscode/scripts/customData.ts`'s, which the contributions suite
// runs too, so this command only reads `extensions/pdf/subset.json` and writes or compares.

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { customData, type Subset } from "../../../editors/vscode/scripts/customData.ts";
import { ROOT } from "../lib/paths.ts";

export const summary = "the editor's PDF custom data from extensions/pdf/subset.json: nv pdf-custom-data [--check]";

const SUBSET = "extensions/pdf/subset.json";
const HTML = "editors/vscode/data/pdf.html-data.json";
const CSS = "editors/vscode/data/pdf.css-data.json";

export async function run(args: string[]): Promise<number> {
  const check = args.includes("--check");
  const unknown = args.filter((a) => a !== "--check");
  if (unknown.length > 0) {
    console.error(`nv pdf-custom-data: unknown argument ${unknown[0]}`);
    return 2;
  }
  const data = customData(JSON.parse(readFileSync(join(ROOT, SUBSET), "utf8")) as Subset);
  const files: [string, string][] = [[HTML, data.html], [CSS, data.css]];
  let stale = 0;
  for (const [path, text] of files) {
    const full = join(ROOT, path);
    const current = existsSync(full) ? readFileSync(full, "utf8") : undefined;
    if (current === text) {
      continue;
    }
    if (check) {
      console.error(`${path} is stale: run bun nv pdf-custom-data`);
      stale++;
    } else {
      writeFileSync(full, text);
      console.log(`wrote ${path}`);
    }
  }
  return stale > 0 ? 1 : 0;
}
