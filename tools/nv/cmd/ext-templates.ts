// `bun nv ext-templates`: the two `nvs ext new` templates, each written, built, loaded, tested and
// called, as an extension author would.
//
//     bun nv ext-templates --lang rust      the Rust template
//     bun nv ext-templates --lang c         the C template, which needs wasi-sdk and wit-bindgen
//     bun nv ext-templates                  both
//
// For each language this writes the template with `nvs ext new` under `.agent-tmp/ext-templates/`,
// builds it for `wasm32-wasip2` with the commands its README gives, runs `nvs ext build`,
// `nvs ext verify` and `nvs ext test`, and then runs a program that calls the extension from a
// configuration holding the entry `nvs ext pin` prints. It prints `<lang> template: built, verified,
// tested and called` when every step passed, and deletes the scratch folder either way.
//
// `.github/workflows/ci.yml`'s `ext-templates` job runs this on Linux, Windows and macOS, so the job
// and a contributor run one definition. The C leg needs `WASI_SDK_PATH` naming a wasi-sdk and
// `wit-bindgen` on `PATH`: without them it is skipped, and under CI (`CI` set) it fails, so the job
// cannot pass by skipping it. The `nvs` it drives is `NVS_BIN` when that is set, and otherwise the
// pipeline's `covws` build.

import { existsSync, mkdirSync, rmSync, rmdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { covwsNvs } from "../lib/covws.ts";
import { ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { ArgError, parseArgs } from "../lib/py.ts";

export const summary = "build, load, test and call each `nvs ext new` template: nv ext-templates [--lang rust|c]";

const USAGE = "usage: nv ext-templates [-h] [--lang {rust,c}]";

const LANGS = ["rust", "c"] as const;
type Lang = (typeof LANGS)[number];

/** The program that calls the template's one method, and what it prints. */
const PROGRAM = '<?nvs\nuse Example\\Greeting;\n\necho Greeting::hello("Ada"), "\\n";\n';
const PRINTS = "Hello, Ada!";

// A cold Rust build compiles `wit-bindgen` and its dependencies first.
const BUILD_TIMEOUT_MS = 15 * 60_000;
const STEP_TIMEOUT_MS = 5 * 60_000;

const EXE = process.platform === "win32" ? ".exe" : "";

function help(): string {
  return [
    USAGE,
    "",
    "Write each `nvs ext new` template under .agent-tmp/, build it for wasm32-wasip2, run",
    "`nvs ext build`, `verify` and `test` on it, and call it from a program.",
    "",
    "options:",
    "  --lang {rust,c}  one template; both without it",
    "",
    "The C template needs WASI_SDK_PATH and `wit-bindgen` on PATH. Without them it is",
    "skipped, and under CI it fails.",
  ].join("\n");
}

/** Whether `CI` names a CI run, as GitHub Actions and every other runner set it. */
function underCi(env: Record<string, string | undefined>): boolean {
  const ci = (env.CI ?? "").trim().toLowerCase();
  return ci !== "" && ci !== "0" && ci !== "false";
}

/** What the C leg is missing of wasi-sdk and `wit-bindgen`, one phrase each; empty when nothing is. */
export function cToolsMissing(env: Record<string, string | undefined>): string[] {
  const missing: string[] = [];
  const sdk = env.WASI_SDK_PATH ?? "";
  if (!sdk || !existsSync(join(sdk, "bin", `clang${EXE}`))) missing.push("wasi-sdk (set WASI_SDK_PATH)");
  if (!Bun.which("wit-bindgen", { PATH: env.PATH ?? "" })) missing.push("`wit-bindgen` on PATH");
  return missing;
}

/** One step's failure: the command and what it printed. */
class StepFailed extends Error {}

async function step(argv: string[], cwd: string, timeoutMs = STEP_TIMEOUT_MS): Promise<string> {
  const r = await runProc(argv, { cwd, timeoutMs });
  if (r.code !== 0) {
    const said = `${r.stdout}${r.stderr}`.trim();
    throw new StepFailed(`\`${argv.join(" ")}\` exited ${r.code}${r.timedOut ? " (timed out)" : ""}\n${said}`);
  }
  return r.stdout;
}

/** The commands the template's README gives to build its module, each run in the project. */
function buildSteps(lang: Lang): string[][] {
  if (lang === "rust") return [["cargo", "build", "--release", "--target", "wasm32-wasip2"]];
  const clang = join(process.env.WASI_SDK_PATH!, "bin", `clang${EXE}`);
  return [
    ["wit-bindgen", "c", "wit", "--world", "greeting", "--out-dir", "gen"],
    [
      clang, "--target=wasm32-wasip2", "-mexec-model=reactor", "-O2", "-Igen",
      "-o", "greeting.wasm", "greeting.c", join("gen", "greeting.c"), join("gen", "greeting_component_type.o"),
    ],
  ];
}

/** Every step for `lang`, in `scratch`, which is empty when this starts. */
async function leg(lang: Lang, nvs: string, scratch: string): Promise<void> {
  const project = join(scratch, "greeting");
  await step([nvs, "ext", "new", "--lang", lang, project], scratch);
  for (const argv of buildSteps(lang)) await step(argv, project, BUILD_TIMEOUT_MS);
  await step([nvs, "ext", "build"], project);
  await step([nvs, "ext", "verify", "greeting.nvsx"], project);
  await step([nvs, "ext", "test"], project);
  const app = join(scratch, "app");
  mkdirSync(app);
  writeFileSync(join(app, "nvs.toml"), await step([nvs, "ext", "pin", join(project, "greeting.nvsx")], app));
  writeFileSync(join(app, "main.nvs"), PROGRAM);
  const out = await step([nvs, "run", "main.nvs"], app);
  if (out.trim() !== PRINTS) throw new StepFailed(`the program printed ${JSON.stringify(out)}, not ${JSON.stringify(PRINTS)}`);
}

export async function run(args: string[]): Promise<number> {
  let values: Map<string, string>;
  let flags: Set<string>;
  try {
    ({ flags, values } = parseArgs(args, { flags: [], valued: ["--lang"], order: ["--lang"] }));
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv ext-templates: error: ${e.message}`);
    return 2;
  }
  if (flags.has("--help")) {
    console.log(help());
    return 0;
  }
  const asked = values.get("--lang");
  if (asked !== undefined && !(LANGS as readonly string[]).includes(asked)) {
    console.error(`${USAGE}\nnv ext-templates: error: --lang is rust or c, not ${JSON.stringify(asked)}`);
    return 2;
  }
  const langs: Lang[] = asked ? [asked as Lang] : [...LANGS];
  const nvs = process.env.NVS_BIN || covwsNvs();
  let failed = false;
  for (const lang of langs) {
    if (lang === "c") {
      const missing = cToolsMissing(process.env);
      if (missing.length > 0) {
        if (underCi(process.env)) {
          console.log(`c template: FAILED, CI has no ${missing.join(" and no ")}`);
          failed = true;
        } else console.log(`c template: skipped, no ${missing.join(" and no ")}`);
        continue;
      }
    }
    if (!existsSync(nvs)) {
      console.log(`${lang} template: FAILED, no \`nvs\` at ${nvs}; set NVS_BIN or run \`bun nv verify\``);
      failed = true;
      continue;
    }
    const scratch = join(ROOT, ".agent-tmp", "ext-templates", `${lang}-${process.pid}`);
    rmSync(scratch, { recursive: true, force: true });
    mkdirSync(scratch, { recursive: true });
    try {
      await leg(lang, nvs, scratch);
      console.log(`${lang} template: built, verified, tested and called`);
    } catch (e) {
      if (!(e instanceof StepFailed)) throw e;
      console.log(`${lang} template: FAILED, ${e.message}`);
      failed = true;
    } finally {
      rmSync(scratch, { recursive: true, force: true });
      try {
        rmdirSync(dirname(scratch));
      } catch {
        // Another run's folder is still in it.
      }
    }
  }
  return failed ? 1 : 0;
}
