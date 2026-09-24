// The headless tier, as one command: every suite under `out/test/` that runs without an editor, in
// order, each reporting its own line so a failing ledger entry says which one broke. The host tier's
// suite lives under the same tree and is skipped here; `scripts/host.mjs` is what runs it.
//
// `tools/loop.py`'s acceptance check greps this output for `grammar:`, `contributions:`,
// `protocol:` and `0 failing`, and `nv verify` greps it for `N passing`. A suite that does
// not exist yet prints no line and the check stays red, which is the state the goal's later stages
// are in — the runner never invents a green for a directory nobody has written.
//
// Nothing here needs an editor, a display or a network.

import { existsSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import Mocha from "mocha";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = resolve(HERE, "..", "out", "test");

// The order a reader wants them in: colour before contributions before the wire. A suite not
// named here runs after these, alphabetically.
const ORDER = ["grammar", "contributions", "protocol"];

// `host` is the other tier and imports `vscode`, which resolves to nothing outside the extension host.
// `scripts/host.mjs` is what runs it.
const ELSEWHERE = ["host"];

function suites() {
  if (!existsSync(OUT)) {
    return [];
  }
  const found = readdirSync(OUT, { withFileTypes: true })
    .filter((e) => e.isDirectory())
    .map((e) => e.name)
    .filter((name) => !ELSEWHERE.includes(name))
    .filter((name) => cases(name).length > 0);
  return found.sort((a, b) => {
    const ai = ORDER.indexOf(a);
    const bi = ORDER.indexOf(b);
    if (ai !== bi) {
      return (ai < 0 ? ORDER.length : ai) - (bi < 0 ? ORDER.length : bi);
    }
    return a.localeCompare(b);
  });
}

function cases(name) {
  const dir = join(OUT, name);
  return readdirSync(dir)
    .filter((f) => f.endsWith(".test.js"))
    .sort()
    .map((f) => join(dir, f));
}

async function run(name) {
  const mocha = new Mocha({ ui: "bdd", reporter: "spec", color: false });
  for (const file of cases(name)) {
    mocha.addFile(file);
  }
  await mocha.loadFilesAsync();
  const stats = await new Promise((done) => {
    const runner = mocha.run(() => done(runner.stats));
  });
  console.log(`${name}: ${stats.passes} passing, ${stats.failures} failing`);
  return stats;
}

const names = suites();
if (names.length === 0) {
  console.log("headless: no suites under out/test -- did `tsc -p ./` run?");
  process.exit(1);
}

let passing = 0;
let failing = 0;
for (const name of names) {
  const stats = await run(name);
  passing += stats.passes;
  failing += stats.failures;
}
console.log(`headless: ${passing} passing, ${failing} failing`);
process.exit(failing === 0 ? 0 : 1);
