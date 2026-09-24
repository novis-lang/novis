// `bun nv plan`: the read-only modes over the implementation plan. `--show M8` prints a milestone's
// scope file whole, `--show M8:lead` its first paragraph and `--show M8:verify` its `**Verify:**`
// paragraph, its acceptance test. `--get <Field>` prints one field of the status block, unwrapped.
//
// The roster and the status block are read from the `milestone` and `plan_status` records under
// `data/plan/`. A milestone's scope is prose, `docs/plan/<id>.md`, and its H1 is where the body starts.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { load, pathOf } from "../lib/store.ts";
import { milestone as milestoneType } from "../schema/milestone.ts";
import { planStatus } from "../schema/plan-status.ts";

export const summary = "the plan, read-only: nv plan --show M8[:lead|:verify], nv plan --get <Field>";

/** A milestone file's H1: `# M4S — The `Core` API contract and its pure half (~5 weeks)`. */
const H1 = /^#\s+(M\d+[A-Z]?)\s*—\s*(.*)$/;

/** The status block's field names as the plan writes them, and each one's key in the record. */
const FIELDS: [string, string][] = [
  ["Status", "status"],
  ["Done", "done"],
  ["On disk", "onDisk"],
  ["Toolchain", "toolchain"],
  ["ADR slices landed", "adrSlicesLanded"],
  ["Open now", "openNow"],
  ["Blocking", "blocking"],
];

interface Entry {
  id: string;
  rel: string;
  record: string;
}

/** Every milestone, in the table's order. */
function milestones(): Entry[] {
  return load(milestoneType)
    .sort((a, b) => a.value.order - b.value.order)
    .map((m) => ({ id: m.id, rel: `docs/plan/${m.id.toLowerCase()}.md`, record: pathOf(milestoneType, m.id) }));
}

/** `'x'`, the way Python's `repr` prints a string with no quote in it. */
function repr(s: string): string {
  return s.includes("'") && !s.includes('"') ? `"${s}"` : `'${s}'`;
}

/** A milestone file's text below its H1, with the blank lines at either end cut off. */
function bodyOf(entry: Entry): string {
  const text = readFileSync(join(ROOT, entry.rel), "utf8").replace(/\r\n/g, "\n");
  const lines = text.split("\n");
  const at = lines.findIndex((l) => H1.test(l));
  return (at < 0 ? text : lines.slice(at + 1).join("\n")).replace(/^\n+|\n+$/g, "");
}

/** The first paragraph: what the milestone is, before the detail. */
function leadParagraph(entry: Entry): string {
  const buf: string[] = [];
  for (const raw of bodyOf(entry).split("\n")) {
    if (!raw.trim()) {
      if (buf.length > 0) break;
      continue;
    }
    buf.push(raw.trim());
  }
  return buf.join(" ");
}

/** The `**Verify:**` or `**Verified:**` paragraph, or `""` when the milestone has none. */
function verifyParagraph(entry: Entry): string {
  const lines = bodyOf(entry).split("\n");
  const at = lines.findIndex((l) => l.startsWith("**Verify:**") || l.startsWith("**Verified:**"));
  if (at < 0) return "";
  const buf: string[] = [];
  for (const line of lines.slice(at)) {
    if (!line.trim()) break;
    buf.push(line.trim());
  }
  return buf.join(" ");
}

function show(spec: string): number {
  const colon = spec.indexOf(":");
  const id = (colon < 0 ? spec : spec.slice(0, colon)).trim().toUpperCase();
  const part = colon < 0 ? "" : spec.slice(colon + 1);
  const index = milestones();
  const entry = index.find((m) => m.id === id);
  if (!entry) {
    console.log(`nv plan: no milestone ${repr(colon < 0 ? spec : spec.slice(0, colon))}. The index has: ` +
      index.map((m) => m.id).join(", "));
    return 1;
  }
  if (!existsSync(join(ROOT, entry.rel))) {
    console.log(`nv plan: ${entry.rel} does not exist (the record is ${entry.record})`);
    return 1;
  }
  if (part === "" || part === "all") {
    const text = readFileSync(join(ROOT, entry.rel), "utf8").replace(/\r\n/g, "\n");
    console.log(`-- ${entry.rel}\n\n${text.replace(/\n+$/, "")}`);
  } else if (part === "lead") {
    console.log(leadParagraph(entry));
  } else if (part === "verify") {
    const got = verifyParagraph(entry);
    if (!got) {
      console.log(`nv plan: ${entry.id} has no \`**Verify:**\` paragraph`);
      return 1;
    }
    console.log(got);
  } else {
    console.log(`nv plan: unknown part ${repr(part)} -- use \`:lead\`, \`:verify\`, or no suffix`);
    return 2;
  }
  return 0;
}

function get(name: string): number {
  const status = load(planStatus)[0]!.value as Record<string, string>;
  const hit = FIELDS.find(([field]) => field.toLowerCase() === name.toLowerCase());
  if (!hit) {
    console.log(`nv plan: no field ${repr(name)}. The block has: ${FIELDS.map(([f]) => f).join(", ")}`);
    return 1;
  }
  console.log(status[hit[1]]);
  return 0;
}

export async function run(args: string[]): Promise<number> {
  const [flag, value, ...rest] = args;
  if ((flag === "--show" || flag === "--get") && value !== undefined && rest.length === 0) {
    return flag === "--show" ? show(value) : get(value);
  }
  console.error("usage: bun nv plan --show M8[:lead|:verify] | --get <Field>\n" +
    "  the plan's other modes are still `python tools/plan.py`'s");
  return 2;
}
