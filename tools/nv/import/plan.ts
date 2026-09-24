// The implementation plan's legacy homes: the `> **Field:**` status block at the head of
// `docs/implementation-plan.md` becomes `data/plan/status.json`, and each row of its milestone table
// `data/plan/milestones/<id>.json`.
//
// A status field is its Markdown with each paragraph unwrapped to one line. A milestone's id is its
// link text, which links `plan/<id in lower case>.md`; its title and estimate are the "What it
// builds" cell split at the parenthesis that ends it, and that file's H1 must say the same. The
// "Carried by" cell is `done`, `ongoing`, `backlog N`, or the goals carrying it: the last is derived
// from the goals' own `milestone`, so it becomes `open` and nothing more.

import { milestone } from "../schema/milestone.ts";
import { planStatus } from "../schema/plan-status.ts";
import { exists, text, unwrap, type Importer, type ImportResult } from "./lib.ts";

const PLAN = "docs/implementation-plan.md";
const FIELDS: Record<string, string> = {
  Status: "status",
  Done: "done",
  "On disk": "onDisk",
  Toolchain: "toolchain",
  "ADR slices landed": "adrSlicesLanded",
  "Open now": "openNow",
  Blocking: "blocking",
};
const TABLE_HEAD = "| Carried by | Milestone | What it builds | Loop-days |";

export const plan: Importer = {
  name: "plan",
  read(root) {
    const out: ImportResult = { records: [], unread: [], files: 1 };
    const lines = text(root, PLAN).split("\n");

    const start = lines.findIndex((l) => l.startsWith("> **"));
    const status: Record<string, string> = {};
    let field: string | null = null;
    let body: string[] = [];
    const flush = () => {
      if (field !== null) status[field] = unwrap(body);
    };
    for (let i = start; start >= 0 && i < lines.length && lines[i]!.startsWith(">"); i++) {
      const line = lines[i]!.replace(/^> ?/, "");
      const m = /^\*\*([^*]+):\*\* (.*)$/.exec(line);
      if (m) {
        flush();
        field = FIELDS[m[1]!] ?? null;
        body = [m[2]!];
        if (field === null) out.unread.push({ path: PLAN, reason: `its status block has a field "${m[1]}", which no record holds` });
      } else {
        body.push(line);
      }
    }
    flush();
    if (start < 0) out.unread.push({ path: PLAN, reason: "it has no `> **Field:**` status block" });
    else out.records.push({ type: planStatus, id: planStatus.name, value: status, from: PLAN });

    const head = lines.indexOf(TABLE_HEAD);
    if (head < 0) {
      out.unread.push({ path: PLAN, reason: `it has no milestone table headed \`${TABLE_HEAD}\`` });
      return out;
    }
    for (let i = head + 2, order = 1; i < lines.length && lines[i]!.startsWith("|"); i++, order++) {
      const cells = lines[i]!.replace(/^\|\s*|\s*\|$/g, "").split(/\s+\|\s+/);
      const link = /^\[([^\]]+)\]\(plan\/([^)]+)\)$/.exec(cells[1] ?? "");
      const split = / \(([^()]+)\)$/.exec(cells[2] ?? "");
      if (cells.length !== 4 || !link || !split) {
        out.unread.push({ path: PLAN, reason: `milestone row ${order} is not \`| carried | [Mn](plan/mn.md) | title (estimate) | days |\`` });
        continue;
      }
      const id = link[1]!;
      const prose = `docs/plan/${link[2]}`;
      if (link[2] !== `${id.toLowerCase()}.md`) out.unread.push({ path: PLAN, reason: `milestone ${id} links ${prose}` });
      const carried = cells[0]!;
      const backlog = /^backlog (\d+)$/.exec(carried);
      const state = carried === "done" || carried === "ongoing" ? carried : "open";
      if (!backlog && state === "open" && !/^goals? `/.test(carried)) {
        out.unread.push({ path: PLAN, reason: `milestone ${id} is carried by "${carried}"` });
      }
      const value: Record<string, unknown> = {
        title: cells[2]!.slice(0, split.index),
        order,
        estimate: split[1],
        loopDays: cells[3],
        state,
        ...(backlog ? { backlog: Number(backlog[1]) } : {}),
      };
      out.records.push({ type: milestone, id, value, from: PLAN });
      if (!exists(root, prose)) {
        out.unread.push({ path: prose, reason: `milestone ${id}'s prose is not on disk` });
        continue;
      }
      out.files++;
      const h1 = text(root, prose).split("\n")[0];
      if (h1 !== `# ${id} — ${cells[2]}`) out.unread.push({ path: prose, reason: `its H1 is not the table's \`# ${id} — ${cells[2]}\`` });
    }
    return out;
  },
};
