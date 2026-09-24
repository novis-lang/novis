// `bun nv class-cards`: which `Core` classes still owe the card `rule:core-api/reference-card` gives a
// class.
//
//     bun nv class-cards            one line per class still owing, then the count
//     bun nv class-cards --check    the same, exit 1 while any class owes one
//
// The one home of the answer is `CLASSES_STILL_OWING_A_CARD` in `crates/nvs-stdlib/src/registry.rs`:
// the registry test `every_registry_row_carries_a_reference_card` lets exactly those classes ship
// without a `ClassDoc`, and fails naming any of them the session it gains one, so the list only ever
// shrinks. This command reads that list and prints it, so a session can see what is left without a
// build, and goal `core-class-cards`'s acceptance check is this command printing that nothing is.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";

export const summary = "which Core classes still owe their card: nv class-cards [--check]";

const REGISTRY = "crates/nvs-stdlib/src/registry.rs";
const LIST = /const CLASSES_STILL_OWING_A_CARD: &\[&str\] = &\[([\s\S]*?)\];/;

export async function run(args: string[]): Promise<number> {
  const match = LIST.exec(readFileSync(join(ROOT, REGISTRY), "utf8"));
  if (!match) {
    console.error(`nv class-cards: ${REGISTRY} has no CLASSES_STILL_OWING_A_CARD`);
    return 1;
  }
  const names = [...match[1]!.matchAll(/r"([^"]+)"/g)].map((m) => m[1]!);
  for (const name of names) console.log(`  ${name} still owes its card`);
  if (names.length > 0) {
    console.log(`${names.length} Core class(es) still owe a card`);
    return args.includes("--check") ? 1 : 0;
  }
  console.log("every Core class carries its card");
  return 0;
}
