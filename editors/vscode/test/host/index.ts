// What `runTests` calls once the extension host is up: every `*.test.js` beside this file, under
// Mocha's `spec` reporter.
//
// The report is written to `.vscode-test/host-report.txt` rather than trusted to stdout. Inside the
// extension host, `console.log` reaches the editor's own output channels and only sometimes the
// terminal that launched it, so `scripts/host.mjs` prints this file instead and the acceptance check
// reads one stream on every platform. The last line is always `host: N passing, M failing`, which is
// the pair `tools/loop.py` greps.

import { readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { format } from "node:util";
import * as Mocha from "mocha";

export async function run(): Promise<void> {
  const here = __dirname;
  const report = process.env.NVS_HOST_REPORT ?? join(here, "..", "..", "..", ".vscode-test", "host-report.txt");

  // Every reporter in Mocha prints through this one function, so replacing it captures the spec
  // output whole — including the epilogue — without reimplementing the reporter or racing the
  // editor for `process.stdout`.
  const lines: string[] = [];
  Mocha.reporters.Base.consoleLog = (...args: unknown[]) => {
    lines.push(format(...args));
  };

  const mocha = new Mocha({ ui: "bdd", reporter: "spec", color: false, timeout: 30000 });
  for (const file of readdirSync(here).filter((name) => name.endsWith(".test.js")).sort()) {
    mocha.addFile(join(here, file));
  }
  await mocha.loadFilesAsync();

  const stats = await new Promise<Mocha.Stats | undefined>((done) => {
    const runner = mocha.run(() => done(runner.stats));
  });
  // A run that ends with no statistics ended some other way than by finishing, which is a failure
  // whatever the tests did.
  const passing = stats?.passes ?? 0;
  const failing = stats?.failures ?? 1;

  lines.push(`host: ${passing} passing, ${failing} failing`);
  writeFileSync(report, `${lines.join("\n")}\n`);
  if (failing > 0) {
    throw new Error(`host: ${failing} failing`);
  }
}
