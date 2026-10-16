// A scratch repository root for one test, under `.cache/nv-test/`, which is inside the project and
// git-ignored. `cleanup` deletes it.

import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { CACHE } from "../lib/paths.ts";

export interface Scratch {
  root: string;
  put(path: string, text: string): void;
  cleanup(): void;
}

export function scratch(): Scratch {
  const root = join(CACHE, "nv-test", crypto.randomUUID());
  mkdirSync(root, { recursive: true });
  return {
    root,
    put(path, text) {
      const full = join(root, path);
      mkdirSync(dirname(full), { recursive: true });
      writeFileSync(full, text);
    },
    cleanup() {
      rmSync(root, { recursive: true, force: true });
    },
  };
}
