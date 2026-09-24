// Every importer `bun nv import` runs, in one list. Each reads the legacy homes of one or more record
// types; a type no importer here covers is named by `nv import --check` as not imported yet.

import { decisions } from "./decisions.ts";
import { gaps } from "./gaps.ts";
import { goals } from "./goals.ts";
import type { Importer } from "./lib.ts";
import { plan } from "./plan.ts";
import { playbook } from "./playbook.ts";
import { reference } from "./reference.ts";
import { rules } from "./rules.ts";

export const IMPORTERS: Importer[] = [rules, decisions, reference, playbook, goals, plan, gaps];
