// `test` for every kit test file: bun:test's own, which also writes `(pass) <name>` on stderr once the
// test's body returns. A goal's check reads that line to know the test ran and passed. Bun's reporter
// writes the same line, but omits it when it runs under an AI agent, and that choice does not follow
// an environment variable this file can set. Under a plain console the line appears twice.

import { test as bunTest } from "bun:test";

export { expect } from "bun:test";

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
