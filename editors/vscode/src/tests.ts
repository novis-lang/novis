// The Test Explorer: the two suites `nvs test` runs, in the editor's own Testing API.
//
// `rule:ide/the-extension-builds-no-ui-the-editor-already-has` is what decides the shape. A
// `TestController` is fed and VS Code draws the tree, the gutter icons, the run buttons and the
// failure peek; nothing here renders any of that. Coverage is the same: the Coverage profile runs a
// program with `--coverage-lcov` into a scratch file in the extension's storage directory, and
// each file the tracefile lists becomes a `FileCoverage` whose statements, branches and functions are the runner's own counts
// (`rule:testing/coverage-report`). VS Code draws the gutter and the summary. The `.nvst` corpus
// has no coverage to give, because `nvs test` refuses the flag over a case tree, so under that
// profile a case runs as it does under Run.
//
// **Discovery never runs anything.** `nvs test --list --format=json` is the front end's answer off
// the compiled test table (`crates/nvs-cli/src/runner.rs`, `listing`), so a program whose tests
// fail, hang or `exit` lists exactly as one whose tests pass. It costs one compile per program and
// is asked for lazily — when the Testing view is first opened, and again on its refresh button —
// rather than on activation, because a workspace's programs are not the editor's business until
// somebody asks what tests it holds.
//
// **Two suites, because there are two.** A program's `#[Test]` methods and the `.nvst` corpus
// report differently and share no summary (`rule:testing/nvst-is-separate`), and the CLI refuses a
// machine format over a case tree for exactly that reason — so the corpus half is one process per
// case file and the exit status is the verdict. Nothing here parses a human rendering to recover a
// field the JSON should have carried.
//
// What a document *means* is `src/report.ts`, which imports no `vscode` and is where the headless
// suite tests it. What is left here is the controller, the processes and the editor's run object.

import { execFile } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";

import {
  BranchCoverage,
  CancellationToken,
  DeclarationCoverage,
  ExtensionContext,
  FileCoverage,
  FileCoverageDetail,
  Position,
  Range,
  StatementCoverage,
  TestController,
  TestItem,
  TestMessage,
  TestRun,
  TestRunProfileKind,
  TestRunRequest,
  Uri,
  tests,
  workspace,
} from "vscode";

import { binary, runnable } from "./binary";
import {
  Listed,
  Reported,
  Traced,
  filter,
  listed,
  messages,
  name,
  ran,
  state,
  statements,
  suites,
  traced,
} from "./report";

/** The controller's id, and the `testing` view's grouping key for everything below it. */
export const CONTROLLER = "nvs";

/** The item every `.nvst` case hangs under: the second suite, named as one. */
export const CORPUS = "nvst";

/** Where the two suites are found, and what is never walked looking for them. */
const PROGRAMS = "**/*.nvs";
const CASES = "**/*.nvst";
const EXCLUDED = "**/{node_modules,target,.git}/**";

// A JSON report of a large suite is larger than the program that produced it, and Node caps a
// child process's output at a megabyte by default. The controller holds one listing per program
// for as long as the view is open, which is the memory this feature spends and all of it.
const OUTPUT_CEILING = 64 * 1024 * 1024;

/** What a program's listing said, kept so a run can tell a filtered request from a whole file. */
const known = new Map<string, Listed[]>();

// The details behind each `FileCoverage` a run added, read back when the user opens that file's
// coverage. A run's details live as long as the editor keeps the run's coverage.
const details = new WeakMap<FileCoverage, FileCoverageDetail[]>();

// The extension's own storage directory, where each coverage run writes its tracefile and deletes it.
let storage = "";

/** Create the controller, and let the editor decide when to fill it. */
export function install(context: ExtensionContext): void {
  const controller = tests.createTestController(CONTROLLER, "Novis");
  context.subscriptions.push(controller);
  storage = (context.storageUri ?? context.globalStorageUri).fsPath;
  // Both are the same pass: the editor calls the first when the view opens with nothing resolved,
  // and the second when the user presses refresh.
  controller.resolveHandler = async (item?: TestItem): Promise<void> => {
    if (item === undefined) {
      await discover(controller);
    }
  };
  controller.refreshHandler = (): Promise<void> => discover(controller);
  controller.createRunProfile(
    "Run",
    TestRunProfileKind.Run,
    (request: TestRunRequest, token: CancellationToken) =>
      void perform(controller, request, token, false),
    true,
  );
  const coverage = controller.createRunProfile(
    "Run with Coverage",
    TestRunProfileKind.Coverage,
    (request: TestRunRequest, token: CancellationToken) =>
      void perform(controller, request, token, true),
    true,
  );
  coverage.loadDetailedCoverage = async (_run, file) => details.get(file) ?? [];
}

/**
 * Fill the tree: every program's `#[Test]` table, then the `.nvst` corpus beside it.
 *
 * A program that lists nothing gets no item rather than an empty one — a workspace is mostly files
 * with no tests in them, and a tree of empty nodes is a tree nobody can read.
 */
async function discover(controller: TestController): Promise<void> {
  known.clear();
  controller.items.replace([]);
  for (const uri of await workspace.findFiles(PROGRAMS, EXCLUDED)) {
    const document = await cli(["test", "--list", "--format=json", uri.fsPath]);
    const declared = document === undefined ? undefined : listed(document.stdout);
    if (declared === undefined || declared.length === 0) {
      continue;
    }
    known.set(uri.toString(), declared);
    controller.items.add(populate(controller, uri, declared));
  }
  const cases = await workspace.findFiles(CASES, EXCLUDED);
  if (cases.length > 0) {
    const corpus = controller.createTestItem(CORPUS, ".nvst cases");
    for (const uri of cases) {
      corpus.children.add(controller.createTestItem(uri.toString(), basename(uri), uri));
    }
    controller.items.add(corpus);
  }
}

/**
 * One program's item: the file, its classes, and a leaf per listed test at the line it is written
 * on.
 *
 * The class is the only grouping the document offers, and `report.ts` owns why that is the whole
 * tree. A record with no location keeps its leaf and gets no range, because a test the compiler
 * could not place still runs.
 */
function populate(controller: TestController, uri: Uri, tests: Listed[]): TestItem {
  const file = controller.createTestItem(uri.toString(), basename(uri), uri);
  for (const suite of suites(tests)) {
    const held = controller.createTestItem(`${uri.toString()}::${suite.class}`, suite.class, uri);
    for (const test of suite.tests) {
      const leaf = controller.createTestItem(
        `${uri.toString()}::${name(test)}`,
        test.method,
        uri,
      );
      leaf.range = located(test);
      held.children.add(leaf);
    }
    file.children.add(held);
  }
  return file;
}

/**
 * Run what was asked for, one process per file, and report each verdict onto the item it belongs
 * to.
 *
 * The request's items are expanded to leaves first, because the editor sends whatever node the
 * user pressed and the runner reports per test. A leaf whose record never arrived is left
 * unreported rather than guessed at: it is the one state that says the process did not answer for
 * it.
 */
async function perform(
  controller: TestController,
  request: TestRunRequest,
  token: CancellationToken,
  coverage: boolean,
): Promise<void> {
  const run = controller.createTestRun(request);
  const queued = leaves(controller, request);
  for (const item of queued) {
    run.enqueued(item);
  }
  for (const [file, items] of byFile(queued)) {
    if (token.isCancellationRequested) {
      break;
    }
    if (file === CORPUS) {
      await corpus(run, items);
    } else {
      await program(run, Uri.parse(file), items, coverage);
    }
  }
  run.end();
}

/**
 * One program's run: the JSON report, mapped back onto the items that asked for it, and under the
 * Coverage profile the lcov file the same run wrote.
 */
async function program(run: TestRun, uri: Uri, items: TestItem[], coverage: boolean): Promise<void> {
  const wanted = known.get(uri.toString()) ?? [];
  const narrowed = filter(
    wanted.filter((test) => items.some((item) => item.id.endsWith(`::${name(test)}`))),
    wanted,
  );
  for (const item of items) {
    run.started(item);
  }
  const argv = ["test", "--format=json", uri.fsPath];
  const asked = narrowed === undefined ? argv : [...argv, "--filter", narrowed];
  if (!coverage) {
    answer(run, items, await cli(asked));
    return;
  }
  // The tracefile names each file relative to the directory the run starts in, so the run starts
  // in the program's workspace folder and every name is resolved against that folder.
  const folder = workspace.getWorkspaceFolder(uri)?.uri.fsPath ?? dirname(uri.fsPath);
  await mkdir(storage, { recursive: true });
  const scratch = await mkdtemp(join(storage, "coverage-"));
  try {
    const tracefile = join(scratch, "coverage.lcov");
    answer(run, items, await cli([...asked, "--coverage-lcov", tracefile], folder));
    const files = traced(await readFile(tracefile, "utf8").catch(() => ""));
    for (const trace of files ?? []) {
      run.addCoverage(covered(Uri.file(resolve(folder, trace.file)), trace));
    }
  } finally {
    await rm(scratch, { recursive: true, force: true });
  }
}

/** One traced file as the editor's `FileCoverage`, with its details kept for when it is opened. */
function covered(uri: Uri, trace: Traced): FileCoverage {
  const found: FileCoverageDetail[] = statements(trace).map(
    (line) =>
      new StatementCoverage(
        line.count,
        new Position(line.line - 1, 0),
        line.branches.map((branch) => new BranchCoverage(branch.count, undefined, branch.label)),
      ),
  );
  for (const declared of trace.functions) {
    found.push(
      new DeclarationCoverage(declared.name, declared.count, new Position(declared.line - 1, 0)),
    );
  }
  const file = FileCoverage.fromDetails(uri, found);
  details.set(file, found);
  return file;
}

/** The verdicts one program's process reported, put on the items that asked for them. */
function answer(
  run: TestRun,
  items: TestItem[],
  document: { stdout: string; stderr: string } | undefined,
): void {
  const report = document === undefined ? undefined : ran(document.stdout);
  if (report === undefined) {
    const said = document?.stderr.trim() ?? `${binary()} did not run`;
    for (const item of items) {
      run.errored(item, new TestMessage(said || `${binary()} test printed no report`));
    }
    return;
  }
  run.appendOutput(document?.stderr.replace(/\n/g, "\r\n") ?? "");
  for (const test of report.tests) {
    const item = items.find((held) => held.id.endsWith(`::${name(test)}`));
    if (item !== undefined) {
      verdict(run, item, test);
    }
  }
}

/** What the editor is told about one reported test. */
function verdict(run: TestRun, item: TestItem, test: Reported): void {
  const said = messages(test).map((line) => new TestMessage(line));
  switch (state(test)) {
    case "passed":
      run.passed(item, test.durationMs);
      break;
    case "skipped":
      run.skipped(item);
      break;
    case "errored":
      run.errored(item, said, test.durationMs);
      break;
    default:
      run.failed(item, said, test.durationMs);
      break;
  }
}

/**
 * The corpus half: one process per case file, and the exit status is the verdict.
 *
 * There is no document to read. `nvs test` refuses `--format` over a `.nvst` tree because the two
 * suites share no summary (`crates/nvs-cli/src/main.rs`), so what a failing case has to say is its
 * own output, handed to the editor as the message rather than parsed for a field.
 */
async function corpus(run: TestRun, items: TestItem[]): Promise<void> {
  for (const item of items) {
    run.started(item);
    const started = Date.now();
    const outcome = await cli(["test", Uri.parse(item.id).fsPath]);
    const elapsed = Date.now() - started;
    if (outcome === undefined) {
      run.errored(item, new TestMessage(`${binary()} did not run`), elapsed);
    } else if (outcome.code === 0) {
      run.passed(item, elapsed);
    } else {
      run.failed(item, new TestMessage(outcome.stdout || outcome.stderr), elapsed);
    }
  }
}

/** Every leaf the request covers, in the tree's own order, with no node counted twice. */
function leaves(controller: TestController, request: TestRunRequest): TestItem[] {
  const roots: TestItem[] = [];
  if (request.include === undefined) {
    controller.items.forEach((item) => roots.push(item));
  } else {
    roots.push(...request.include);
  }
  const found: TestItem[] = [];
  const excluded = new Set(request.exclude?.map((item) => item.id));
  const walk = (item: TestItem): void => {
    if (excluded.has(item.id)) {
      return;
    }
    if (item.children.size === 0) {
      if (!found.some((held) => held.id === item.id)) {
        found.push(item);
      }
      return;
    }
    item.children.forEach(walk);
  };
  roots.forEach(walk);
  return found;
}

/**
 * The queued leaves grouped by the process that will answer for them.
 *
 * A program's leaves share their file's uri; every `.nvst` case is its own file and is grouped
 * under the corpus instead, because the run below spawns one process per case either way.
 */
function byFile(items: TestItem[]): Map<string, TestItem[]> {
  const grouped = new Map<string, TestItem[]>();
  for (const item of items) {
    const key = item.parent?.id === CORPUS ? CORPUS : item.id.split("::").slice(0, -2).join("::");
    grouped.set(key, [...(grouped.get(key) ?? []), item]);
  }
  return grouped;
}

/** What `nvs` printed and what it exited with, or nothing when it did not run at all. */
async function cli(
  args: string[],
  cwd?: string,
): Promise<{ stdout: string; stderr: string; code: number } | undefined> {
  const { command } = await runnable();
  return new Promise((resolve) => {
    execFile(
      command,
      args,
      { cwd, maxBuffer: OUTPUT_CEILING, env: { ...process.env, NO_COLOR: "1" } },
      (failure, stdout, stderr) => {
        if (failure !== null && (failure as NodeJS.ErrnoException).code === "ENOENT") {
          resolve(undefined);
          return;
        }
        const code = failure === null ? 0 : ((failure as { code?: number }).code ?? 1);
        resolve({ stdout, stderr, code });
      },
    );
  });
}

/** A test's declaration as a range, or nothing when the document could not place it. */
function located(test: Listed): Range | undefined {
  if (test.line === null) {
    return undefined;
  }
  // The document counts lines and columns from one, and the editor counts both from zero.
  const at = new Position(test.line - 1, Math.max((test.column ?? 1) - 1, 0));
  return new Range(at, at);
}

/** The last segment of a uri's path, which is what a file's item is labelled with. */
function basename(uri: Uri): string {
  return uri.path.split("/").pop() ?? uri.path;
}
