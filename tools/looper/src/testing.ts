// `test` for every kit test file: bun:test's own, which also writes `(pass) <name>` on stderr once the
// test's body returns. A goal's check reads that line to know the test ran and passed. Bun's reporter
// writes the same line, but omits it when it runs under an AI agent, and that choice does not follow
// an environment variable this file can set. Under a plain console the line appears twice.
//
// Loading this file deletes `LOOPER_ASKER` from `process.env`, so a kit command a test runs asks as
// the test's own `LOOPER_AGENT` or user. A check looper's service runs would pass its `ui` on. A
// `Bun.spawn` without `env` gives its child the process's first environment, so a test passes it.

import { test as bunTest } from "bun:test";

export { expect } from "bun:test";

delete process.env.LOOPER_ASKER;

/** A test named `name`. `timeout` is in ms, bun:test's 5 s without one. */
export function test(name: string, body: () => void | Promise<void>, timeout?: number): void {
  bunTest(
    name,
    async () => {
      await body();
      process.stderr.write(`(pass) ${name}\n`);
    },
    timeout,
  );
}
