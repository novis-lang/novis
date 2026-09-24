// `bun nv render [--check]`: writes every rendered file from the records, or with `--check` writes
// nothing and exits 1 naming each rendered file whose text on disk is not what the records give.

import { apply, type Output } from "../lib/render.ts";
import { ROOT } from "../lib/paths.ts";
import { RENDERERS } from "../renderers/index.ts";

export const summary = "write the rendered files from the records, or --check that they are current";

export async function run(args: string[]): Promise<number> {
  const check = args.includes("--check");
  const unknown = args.filter((a) => a !== "--check");
  if (unknown.length > 0) {
    console.error(`nv render: unknown argument ${unknown.join(" ")}`);
    return 2;
  }
  const outputs: Output[] = [];
  for (const r of RENDERERS) outputs.push(...(await r.render(ROOT)));
  const { stale, unchanged } = apply(outputs, { check });
  for (const path of stale) console.log(check ? `${path}: is not what the records render` : `${path}: written`);
  if (check) {
    if (stale.length === 0) console.log(`render: every generated file is current (${unchanged} of ${RENDERERS.length} renderer(s))`);
    else console.log(`nv render --check: ${unchanged} current, ${stale.length} stale`);
    return stale.length > 0 ? 1 : 0;
  }
  console.log(`nv render: ${stale.length} written, ${unchanged} already current`);
  return 0;
}
