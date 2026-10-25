// The profile viewer, copied into the package: speedscope's built `dist/release/` and its licence,
// into `out/speedscope/`. `bundle` runs this first, so `vsce package --no-dependencies` ships the
// viewer without shipping `node_modules/` (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`).
//
// The source is the `speedscope` devDependency, and `package-lock.json` pins the exact build. No
// source build runs and no copy of the viewer is committed to the tree.

import { copyFileSync, cpSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const SOURCE = join(ROOT, "node_modules", "speedscope");
const TARGET = join(ROOT, "out", "speedscope");

rmSync(TARGET, { recursive: true, force: true });
cpSync(join(SOURCE, "dist", "release"), TARGET, {
  recursive: true,
  filter: (path) => !path.endsWith(".map"),
});
copyFileSync(join(SOURCE, "LICENSE"), join(TARGET, "LICENSE"));
