// `bun nv render [--website] [--check]`: writes every rendered file from the records, and deletes a
// file in a renderer's own directory that it no longer writes. `--website` narrows the run to the
// website's pages and data. `--check` writes nothing and exits 1 naming each rendered file whose text
// on disk is not what the records give, and each file that would be deleted.

import { apply, orphans, removeOrphans, type Output } from "../lib/render.ts";
import { ROOT } from "../lib/paths.ts";
import { RENDERERS, WEBSITE_RENDERERS } from "../renderers/index.ts";

export const summary = "write the rendered files from the records, or --check that they are current; --website for the website's alone";

export async function run(args: string[]): Promise<number> {
  const check = args.includes("--check");
  const website = args.includes("--website");
  const unknown = args.filter((a) => a !== "--check" && a !== "--website");
  if (unknown.length > 0) {
    console.error(`nv render: unknown argument ${unknown.join(" ")}`);
    return 2;
  }
  const renderers = website ? WEBSITE_RENDERERS : RENDERERS;
  const outputs: Output[] = [];
  for (const r of renderers) {
    try {
      outputs.push(...(await r.render(ROOT)));
    } catch (e) {
      console.error(`nv render: ${r.name}: ${e instanceof Error ? e.message : String(e)}`);
      return 1;
    }
  }
  const owned = renderers.flatMap((r) => (r.owns ? [r.owns] : []));
  const gone = orphans(owned, outputs);
  const { stale, unchanged } = apply(outputs, { check });
  if (!check) removeOrphans(gone, owned.map((o) => o.dir));
  for (const path of stale) console.log(check ? `${path}: is not what the records render` : `${path}: written`);
  for (const path of gone) console.log(check ? `${path}: no renderer writes it any more` : `${path}: deleted`);
  const behind = stale.length + gone.length;
  if (check) {
    if (behind === 0) console.log(`render: every generated file is current (${unchanged} file(s) from ${renderers.length} renderer(s))`);
    else console.log(`nv render --check: ${unchanged} current, ${stale.length} stale, ${gone.length} to delete`);
    return behind > 0 ? 1 : 0;
  }
  console.log(`nv render: ${stale.length} written, ${gone.length} deleted, ${unchanged} already current`);
  return 0;
}
