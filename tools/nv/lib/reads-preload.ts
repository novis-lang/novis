// `bun test --preload` for a recorded `bun nv selftest`: when `NV_READS_LOG` names a log, what the tests
// and the tools modules they load read of the tree is recorded into it, as `main.ts` records a command
// (`reads.ts`). It is loaded before any test file, so every module a test loads is rewritten. `bun test`
// fires no exit handler, so the line is written by a hook after the last test.

import { afterAll } from "bun:test";
import { ENV, flush, install } from "./reads.ts";

const log = process.env[ENV];
if (log) {
  install(log);
  afterAll(() => flush(log));
}
