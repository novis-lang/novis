import { describe, expect, test } from "bun:test";
import type { Check } from "../driver/accept.ts";
import { commandEnv } from "../driver/runner.ts";
import { covwsNvs } from "../lib/covws.ts";

const check = (over: Partial<Check>): Check => ({ id: "c", kind: "command", stage: 1, ...over });
const EXE = "/built/covws/nvs";

describe("commandEnv", () => {
  test("a command that starts `nvs` itself, as the editor's headless suites do, finds the pipeline's build in NVS_BIN", () => {
    // Without it the suite falls back to `target/debug/nvs`, which the pipeline never builds.
    const headless = check({ argv: ["npm", "run", "--silent", "test:headless"], cwd: "editors/vscode" });
    expect(commandEnv(headless, headless.argv!, EXE)).toEqual({ NVS_BIN: EXE });
  });

  test("an `{nvs}` command runs with the compile cache off and the same build in NVS_BIN", () => {
    const argv = [covwsNvs(), "queue", "migrate", "--config", "examples/queue-sqlite.toml"];
    expect(commandEnv(check({ argv: ["{nvs}", ...argv.slice(1)] }), argv, EXE)).toEqual({ NOVIS_NO_FILE_CACHE: "1", NVS_BIN: EXE });
  });

  test("a command that measures the release CLI, another kind of check, or a failed build gets no NVS_BIN", () => {
    const bench = check({ argv: ["bun", "nv", "bench", "--guard"] });
    expect(commandEnv(bench, bench.argv!, EXE)).toEqual({});
    const cargo = check({ kind: "cargo-named", args: ["test", "-p", "nvs-lsp"] });
    expect(commandEnv(cargo, ["cargo", "test", "-p", "nvs-lsp"], EXE)).toEqual({});
    const headless = check({ argv: ["npm", "run", "--silent", "test:headless"] });
    expect(commandEnv(headless, headless.argv!, null)).toEqual({});
  });
});
