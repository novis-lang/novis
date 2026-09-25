// The extension-host tier, as one command: a pinned VS Code build, downloaded once into
// `.vscode-test/`, running `out/test/host/index.js` inside the real extension host. That is the only
// place activation on `.nvs`, the Tasks, the status item, the AST panel and the token legend can be
// observed rather than inferred.
//
// Isolation is what makes it safe to run on the machine a developer is working on, and no flag here is
// optional (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`): the build is the
// downloaded one rather than the editor they installed, the profile and extensions directory are
// throwaway, every other extension is off, and the workspace is a copy of `test/host/fixture/` rather
// than this repository. Without them a second instance attaches to the window already open and exits
// with no results, and a test that writes a setting writes it into that developer's own `settings.json`.
//
// `bun nv loop`'s acceptance check greps this output for `host:` and `0 failing`, and what it reads is
// the report the in-host index wrote, printed here after the editor exits. The launcher's own stdout is
// the same on Windows, macOS and Linux; Electron's is not.

import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { runTests } from "@vscode/test-electron";

const HERE = dirname(fileURLToPath(import.meta.url));
const PACKAGE = resolve(HERE, "..");
const ROOT = resolve(PACKAGE, "..", "..");

// The build every machine runs, pinned rather than `stable`: a verdict the unattended loop records has
// to mean the same thing the next morning, and the colour tier asserts through `_workbench
// .captureSyntaxTokens`, a private command a release is free to move. Bumping this is a deliberate edit
// with the suite run behind it, and the fallback when a pinned stable will not start beside the
// developer's own is a pinned Insiders build — never a dropped isolation flag.
const VERSION = "1.136.2";

const CACHE = join(PACKAGE, ".vscode-test");
const PROFILE = join(CACHE, "user-data");
const EXTENSIONS = join(CACHE, "extensions");
const WORKSPACE = join(CACHE, "workspace");
const REPORT = join(CACHE, "host-report.txt");
const FIXTURE = join(PACKAGE, "test", "host", "fixture");

// Which `nvs` the throwaway profile is pointed at: `--nvs <path>` from the acceptance check's argv
// first, then the lookup `test/protocol/session.ts` uses, so a developer running this by hand tests the
// tree they are editing rather than an older install. An empty string is a real answer and is what the
// `nvs.path` setting documents as "look it up on PATH".
function binary() {
  const flag = process.argv.indexOf("--nvs");
  const given = flag < 0 ? undefined : process.argv[flag + 1];
  if (given !== undefined && given.length > 0) {
    return resolve(given);
  }
  const named = process.env.NVS_BIN;
  if (named !== undefined && named.length > 0) {
    return resolve(named);
  }
  const executable = process.platform === "win32" ? "nvs.exe" : "nvs";
  for (const profile of ["debug", "release"]) {
    const built = join(ROOT, "target", profile, executable);
    if (existsSync(built)) {
      return built;
    }
  }
  return "";
}

// The profile and the workspace are built from scratch every run. A profile that survives a run
// remembers what the last one did to it, which is the same stale-green this tier exists to rule out.
function prepare(nvs) {
  rmSync(PROFILE, { recursive: true, force: true });
  rmSync(WORKSPACE, { recursive: true, force: true });
  rmSync(REPORT, { force: true });
  cpSync(FIXTURE, WORKSPACE, { recursive: true });
  const user = join(PROFILE, "User");
  mkdirSync(user, { recursive: true });
  mkdirSync(EXTENSIONS, { recursive: true });
  // `nvs.path` reaches this file and no other. The server starts off because "before the server
  // answers" is a state `colour.test.ts` asserts in, and withholding it is the only way to observe that
  // state rather than race it; the test that needs a server turns the setting on, which respawns. The
  // rest are what stop a fresh profile from opening a welcome page, trusting nothing, or reaching the
  // network on its own account.
  const settings = {
    "nvs.path": nvs,
    "nvs.lsp.enable": false,
    "security.workspace.trust.enabled": false,
    "telemetry.telemetryLevel": "off",
    "update.mode": "none",
    "extensions.autoCheckUpdates": false,
    "workbench.startupEditor": "none"
  };
  writeFileSync(join(user, "settings.json"), `${JSON.stringify(settings, null, 2)}\n`);
}

// A shell that itself runs inside an editor's extension host -- an agent session in VS Code is one
// -- carries `ELECTRON_RUN_AS_NODE`, and the pinned editor inherits it: Electron then runs as plain
// Node, takes the workspace path as a script to execute, and fails on `Cannot find module` before
// any test starts. The variable means nothing to this launcher, so it is dropped rather than worked
// around by every caller.
delete process.env.ELECTRON_RUN_AS_NODE;

const nvs = binary();
prepare(nvs);

let launched = true;
try {
  await runTests({
    version: VERSION,
    cachePath: CACHE,
    extensionDevelopmentPath: PACKAGE,
    extensionTestsPath: join(PACKAGE, "out", "test", "host", "index.js"),
    // What the isolation test compares against: the suite asserts that the editor it is running in is
    // the one this launcher set up, rather than asserting that these flags were passed.
    extensionTestsEnv: {
      NVS_HOST_CACHE: CACHE,
      NVS_HOST_PROFILE: PROFILE,
      NVS_HOST_WORKSPACE: WORKSPACE,
      NVS_HOST_NVS: nvs,
      NVS_HOST_REPORT: REPORT
    },
    launchArgs: [
      WORKSPACE,
      `--user-data-dir=${PROFILE}`,
      `--extensions-dir=${EXTENSIONS}`,
      "--disable-extensions",
      "--disable-workspace-trust",
      "--skip-welcome",
      "--skip-release-notes",
      "--disable-gpu"
    ]
  });
} catch {
  // A failing test and an editor that never started both land here. The report tells them apart, so
  // the verdict is printed from it rather than from this exception.
  launched = false;
}

if (!existsSync(REPORT)) {
  console.log("host: no report under .vscode-test -- the editor never reached the test runner");
  process.exit(1);
}
process.stdout.write(readFileSync(REPORT, "utf8"));
process.exit(launched ? 0 : 1);
