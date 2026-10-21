// `bun nv ext-templates`: the two `nvs ext new` templates, each written, built, loaded, tested and
// called, as an extension author would, and the Markdown example the website's extension guide
// walks through.
//
//     bun nv ext-templates --lang rust      the Rust template
//     bun nv ext-templates --lang c         the C template, which needs wasi-sdk and wit-bindgen
//     bun nv ext-templates --example        the example under `website/guides/markdown-extension/`
//     bun nv ext-templates                  all three
//
// For each language this writes the template with `nvs ext new` under `.agent-tmp/ext-templates/`,
// builds it for `wasm32-wasip2` with the commands its README gives, runs `nvs ext build`,
// `nvs ext verify` and `nvs ext test`, and then runs a program that calls the extension from a
// configuration holding the entry `nvs ext pin` prints. It prints `<lang> template: built, verified,
// tested and called` when every step passed, and deletes the scratch folder either way.
//
// The example leg copies the committed project to the scratch folder, so no build output lands in
// the tree, and holds its `wit/deps/` to the files `nvs ext new` writes today: the guide tells a
// reader to start from `nvs ext new`, and a world that moved on would leave the example behind.
// Then it takes the guide's steps: the build, `nvs ext build`, `verify` and `test`, the pin into
// `demo/nvs.toml`, `nvs check` of the demo page, and a program beside it that calls the class the
// package carries.
//
// `.github/workflows/ci.yml`'s `ext-templates` job runs this on Linux, Windows and macOS, so the job
// and a contributor run one definition. The C leg needs `WASI_SDK_PATH` naming a wasi-sdk and
// `wit-bindgen` on `PATH`: without them it is skipped, and under CI (`CI` set) it fails, so the job
// cannot pass by skipping it. The `nvs` it drives is `NVS_BIN` when that is set, and otherwise the
// pipeline's `covws` build.

import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, rmdirSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { covwsNvs } from "../lib/covws.ts";
import { ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { ArgError, parseArgs } from "../lib/py.ts";

export const summary = "build, load, test and call each `nvs ext new` template and the guide's example: nv ext-templates [--lang rust|c | --example]";

const USAGE = "usage: nv ext-templates [-h] [--lang {rust,c} | --example]";

const LANGS = ["rust", "c"] as const;
type Lang = (typeof LANGS)[number];
/** One leg: a template, or the guide's example. */
type Leg = Lang | "example";

/** The program that calls the template's one method, and what it prints. */
const PROGRAM = '<?nvs\nuse Example\\Greeting;\n\necho Greeting::hello("Ada"), "\\n";\n';
const PRINTS = "Hello, Ada!";

/** The example the website's guide walks through, repository-relative. */
export const EXAMPLE = "website/guides/markdown-extension";
/** The file `nvs ext build` writes for the example: its class's last name, lower-cased. */
const EXAMPLE_NVSX = "parser.nvsx";
/** The program the example leg runs beside the demo page, and what it prints. It calls the class
 * the package carries, which calls both of the component's functions. */
const EXAMPLE_PROGRAM =
  '<?nvs\nuse Example\\Markdown\\Document;\n\necho Document::outline("# Tea\\n## Green"), Document::render("**strong**");\n';
const EXAMPLE_PRINTS = "<ul><li>Tea</li><li>Green</li></ul><p><strong>strong</strong></p>";

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
    "  --lang {rust,c}  one template",
    "  --example        the Markdown example under website/guides/markdown-extension/",
    "",
    "Without an option, both templates and the example.",
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

/** Every file under `dir`, relative to it with `/` separators, sorted. */
function filesIn(dir: string): string[] {
  return readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile())
    .map((e) => relative(dir, join(e.parentPath, e.name)).replaceAll("\\", "/"))
    .sort();
}

/** Whether a path of the example, relative to it, is the project's own: not build output, and not
 * the `demo/nvs.toml` a reader writes from their own pin. */
export function exampleOwns(path: string): boolean {
  const parts = path.replaceAll("\\", "/").split("/");
  return parts[0] !== "target" && !path.endsWith(".nvsx") && parts.join("/") !== "demo/nvs.toml";
}

/** Every step of the guide's example, in `scratch`, which is empty when this starts. */
async function exampleLeg(nvs: string, scratch: string): Promise<void> {
  const source = join(ROOT, EXAMPLE);
  const project = join(scratch, "markdown-extension");
  cpSync(source, project, { recursive: true, filter: (from) => exampleOwns(relative(source, from)) });
  const fresh = join(scratch, "fresh");
  await step([nvs, "ext", "new", "--lang", "rust", fresh], scratch);
  const deps = join("wit", "deps");
  const want = filesIn(join(fresh, deps));
  const have = filesIn(join(project, deps));
  const stale = [...new Set([...want, ...have])].filter(
    (f) => !want.includes(f) || !have.includes(f) || !readFileSync(join(fresh, deps, f)).equals(readFileSync(join(project, deps, f))),
  );
  if (stale.length > 0) {
    throw new StepFailed(
      `${EXAMPLE}/wit/deps/ is not what \`nvs ext new\` writes now: ${stale.join(", ")}. Copy \`wit/deps/\` from a new \`nvs ext new --lang rust\` project`,
    );
  }
  await step(["cargo", "build", "--release", "--target", "wasm32-wasip2"], project, BUILD_TIMEOUT_MS);
  await step([nvs, "ext", "build"], project);
  await step([nvs, "ext", "verify", EXAMPLE_NVSX], project);
  await step([nvs, "ext", "test"], project);
  const demo = join(project, "demo");
  writeFileSync(join(demo, "nvs.toml"), await step([nvs, "ext", "pin", join(project, EXAMPLE_NVSX)], demo));
  await step([nvs, "check", "index.nvs"], demo);
  writeFileSync(join(demo, "main.nvs"), EXAMPLE_PROGRAM);
  const out = await step([nvs, "run", "main.nvs"], demo);
  if (out.trim() !== EXAMPLE_PRINTS) {
    throw new StepFailed(`the program printed ${JSON.stringify(out)}, not ${JSON.stringify(EXAMPLE_PRINTS)}`);
  }
}

export async function run(args: string[]): Promise<number> {
  let values: Map<string, string>;
  let flags: Set<string>;
  try {
    ({ flags, values } = parseArgs(args, { flags: ["--example"], valued: ["--lang"], order: ["--lang", "--example"] }));
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
  if (asked !== undefined && flags.has("--example")) {
    console.error(`${USAGE}\nnv ext-templates: error: --lang and --example name one leg each; give one of them`);
    return 2;
  }
  const legs: Leg[] = asked ? [asked as Lang] : flags.has("--example") ? ["example"] : [...LANGS, "example"];
  const nvs = process.env.NVS_BIN || covwsNvs();
  let failed = false;
  for (const lang of legs) {
    const name = lang === "example" ? "markdown example" : `${lang} template`;
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
      console.log(`${name}: FAILED, no \`nvs\` at ${nvs}; set NVS_BIN or run \`bun nv verify\``);
      failed = true;
      continue;
    }
    const scratch = join(ROOT, ".agent-tmp", "ext-templates", `${lang}-${process.pid}`);
    rmSync(scratch, { recursive: true, force: true });
    mkdirSync(scratch, { recursive: true });
    try {
      if (lang === "example") await exampleLeg(nvs, scratch);
      else await leg(lang, nvs, scratch);
      console.log(`${name}: built, verified, tested and called`);
    } catch (e) {
      if (!(e instanceof StepFailed)) throw e;
      console.log(`${name}: FAILED, ${e.message}`);
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
