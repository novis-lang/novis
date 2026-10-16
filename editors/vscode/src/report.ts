// The two documents `nvs test` prints, and the tree the explorer builds out of them.
//
// Discovery reads `nvs test --list --format=json` and a run reads `nvs test --format=json`, both
// written by `crates/nvs-cli/src/runner.rs`. They are two readings of one schema and the key that
// carries the data says which is which: a listing has `listed` and no summary, because a summary of
// zeros would be a report of a run where nothing passed. A listing is `schemaVersion: 2`; a run is
// 2, or 3 when a coverage flag was passed (`rule:testing/report-formats`). Nothing here re-derives a
// field either document should have carried — a client feature whose CLI surface is missing is the
// CLI's problem, never a parse of the human rendering.
//
// The Coverage profile's third document is the lcov file `--coverage-lcov` writes
// (`rule:testing/coverage-report`), and `traced` is its reader: lines, functions and branches per
// file, as the counts the runner wrote and nothing this client adds up.
//
// Nothing here imports `vscode`, for the reason `src/nodes.ts` gives: the headless tier runs it in
// plain Node, and a decision whose test needs a display is a decision nobody tests. What is left in
// `src/tests.ts` is the controller, the processes and the editor's own run object.

/** Where a test is written, as version 2 of the schema reports it — `null` in all three when the compiler could not place it. */
export interface Located {
  file: string | null;
  line: number | null;
  column: number | null;
}

/** One `#[Test]` call discovery found. It carries no verdict, because nothing ran. */
export interface Listed extends Located {
  class: string;
  method: string;
}

/** One `#[Test]` call a run reported, with everything that verdict brought with it. */
export interface Reported extends Listed {
  verdict: "passed" | "failed" | "skipped" | "flaky" | "exited";
  durationMs: number;
  reason?: string;
  failures?: string[];
  attempts?: number;
  exitCode?: number;
  /** Version 3 only: each file the test reached, mapped to the sorted lines it reached there. */
  coverage?: Record<string, number[]>;
}

/** A whole run: the summary line the terminal prints, and a record per test. */
export interface Run {
  summary: {
    total: number;
    passed: number;
    failed: number;
    skipped: number;
    flaky: number;
    durationMs: number;
  };
  tests: Reported[];
}

/** The tests of one class, in the order the document listed them. */
export interface Suite<T extends Listed> {
  class: string;
  tests: T[];
}

/** One file of an lcov tracefile, with every count as the runner wrote it. */
export interface Traced {
  /** The `SF` name: relative to the directory the run started in, or absolute outside it. */
  file: string;
  /** One per `DA` line, one-based. */
  lines: { line: number; count: number }[];
  /** One per `FN` line, with its `FNDA` count, or `0` when the file gave none. */
  functions: { name: string; line: number; count: number }[];
  /** One per `BRDA` line. `side` is `0` for true and `1` for false; `count` is `0` for a `-`. */
  branches: { line: number; block: number; side: number; count: number }[];
}

/** The version a listing and a run without a coverage flag carry at their root. */
const SCHEMA = 2;

/** The version a run under a coverage flag carries, whose records also have `coverage`. */
const COVERED = 3;

/**
 * The listing `nvs test --list --format=json` printed, or nothing when it is not one.
 *
 * A run document is refused here rather than accepted for its records: it has `tests` and a
 * summary, and reading one as a listing would show the explorer a tree built out of a run that
 * already happened.
 */
export function listed(text: string): Listed[] | undefined {
  const document = parse(text, [SCHEMA]);
  if (document === undefined || !Array.isArray(document.listed)) {
    return undefined;
  }
  return document.listed.every(located) ? (document.listed as Listed[]) : undefined;
}

/**
 * The report `nvs test --format=json` printed, or nothing when it is not one.
 *
 * The summary is required rather than recomputed from the records: it is the runner's own count,
 * and a client that added its own up would disagree with the terminal the day a verdict is added.
 */
export function ran(text: string): Run | undefined {
  const document = parse(text, [SCHEMA, COVERED]);
  if (document === undefined || !Array.isArray(document.tests)) {
    return undefined;
  }
  const summary = document.summary;
  if (typeof summary !== "object" || summary === null) {
    return undefined;
  }
  const reported = document.tests.every(
    (test) => located(test) && typeof (test as Reported).verdict === "string",
  );
  return reported ? ({ summary, tests: document.tests } as Run) : undefined;
}

/**
 * What the report calls a test, and the id the explorer hangs it off.
 *
 * `Class::method` is the runner's own spelling — it is what `--filter` matches and what the
 * plaintext report prints — so a record and the item it belongs to meet on a name neither side
 * invented.
 */
export function name(test: Listed): string {
  return `${test.class}::${test.method}`;
}

/**
 * The listed tests grouped under their class, both in the document's own order.
 *
 * The class is the only grouping the schema offers and the one a reader already has in front of
 * them in the file; a deeper tree would be this client inventing structure the compiler does not
 * report.
 */
export function suites<T extends Listed>(tests: T[]): Suite<T>[] {
  const grouped: Suite<T>[] = [];
  for (const test of tests) {
    const suite = grouped.find((held) => held.class === test.class);
    if (suite === undefined) {
      grouped.push({ class: test.class, tests: [test] });
    } else {
      suite.tests.push(test);
    }
  }
  return grouped;
}

/**
 * The narrowest `--filter` that reaches `wanted` out of `all`, or nothing when the whole file is
 * wanted and there is nothing to narrow.
 *
 * `--filter` is a substring of `Class::method` (`crates/nvs-cli/src/runner.rs`, `selected`), which
 * is one flag rather than a set, so only three requests can be expressed: everything, one class,
 * and one test. Anything else runs the file whole and the run reports the items that were asked
 * for, which is the same verdicts either way — a filter is what a run costs, never what it means.
 *
 * A single test's filter is a substring and so may select a method whose name starts the same way.
 * That extra test runs and its record belongs to no queued item, which is why a run maps records
 * onto items it already has rather than creating one for each record it reads.
 */
export function filter(wanted: Listed[], all: Listed[]): string | undefined {
  if (wanted.length === 0 || wanted.length === all.length) {
    return undefined;
  }
  if (wanted.length === 1) {
    return name(wanted[0]);
  }
  const only = wanted[0].class;
  const whole =
    wanted.every((test) => test.class === only) &&
    all.filter((test) => test.class === only).length === wanted.length;
  return whole ? `${only}::` : undefined;
}

/**
 * What a verdict reads as in the editor's run, which has four states where the runner has five.
 *
 * `flaky` is the one that has to be decided rather than mapped: the runner reports a test that
 * passed on a retry as flaky and never as green (`rule:testing/test-attribute` § 20), and there is
 * no third state on a `TestRun` to put it in — so it is failed, carrying the attempt count, rather
 * than a green tick the terminal did not give it. `exited` is errored because a test that called
 * `exit` returned no verdict at all.
 */
export function state(test: Reported): "passed" | "failed" | "skipped" | "errored" {
  switch (test.verdict) {
    case "passed":
      return "passed";
    case "skipped":
      return "skipped";
    case "exited":
      return "errored";
    default:
      return "failed";
  }
}

/**
 * The lines a failed, flaky or exited test puts on its item, in the order the runner reported them.
 *
 * Every failed assertion is reported and not the first (`crates/nvs-cli/src/runner.rs`), so this is
 * a list; a passing or skipped test has nothing to say and gets an empty one.
 */
export function messages(test: Reported): string[] {
  const said = test.failures ?? [];
  switch (test.verdict) {
    case "flaky":
      return [`it passed on attempt ${test.attempts ?? 1}, having failed with:`, ...said];
    case "exited":
      return [`the test called exit(${test.exitCode ?? 0})`];
    default:
      return said;
  }
}

/**
 * The files an lcov tracefile lists, or nothing when a record in it is not one.
 *
 * Only the keys `nvs test` writes are read: `SF`, `DA`, `FN`, `FNDA` and `BRDA`. The totals
 * (`LF`, `FNH`, `BRF` and the rest) are skipped, because the editor counts the details it is
 * given. A `BRDA` count of `-` means the branch's line never ran, so both sides read `0`.
 */
export function traced(text: string): Traced[] | undefined {
  const files: Traced[] = [];
  let current: Traced | undefined;
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    const colon = line.indexOf(":");
    const key = colon < 0 ? line : line.slice(0, colon);
    const rest = colon < 0 ? "" : line.slice(colon + 1);
    if (key === "SF") {
      current = { file: rest, lines: [], functions: [], branches: [] };
      files.push(current);
      continue;
    }
    if (key === "end_of_record") {
      current = undefined;
      continue;
    }
    if (!["DA", "FN", "FNDA", "BRDA"].includes(key)) {
      continue;
    }
    if (current === undefined) {
      return undefined;
    }
    const fields = rest.split(",");
    if (key === "DA") {
      const [at, count] = [whole(fields[0]), whole(fields[1])];
      if (at === undefined || count === undefined) {
        return undefined;
      }
      current.lines.push({ line: at, count });
    } else if (key === "BRDA") {
      const [at, block, side] = fields.slice(0, 3).map(whole);
      const count = fields[3] === "-" ? 0 : whole(fields[3]);
      if (at === undefined || block === undefined || side === undefined || count === undefined) {
        return undefined;
      }
      current.branches.push({ line: at, block, side, count });
    } else {
      // A function's name is everything after the first comma, so a name may contain one.
      const first = whole(fields[0]);
      const named = fields.slice(1).join(",");
      if (first === undefined || named === "") {
        return undefined;
      }
      if (key === "FN") {
        current.functions.push({ name: named, line: first, count: 0 });
      } else {
        const declared = current.functions.find((held) => held.name === named);
        if (declared === undefined) {
          return undefined;
        }
        declared.count = first;
      }
    }
  }
  return files;
}

/** One line of a file's coverage as the editor shows it: its count, and its branches' sides. */
export interface Covered {
  line: number;
  count: number;
  branches: { count: number; label: string }[];
}

/**
 * A traced file's lines, each with the branch sides whose condition starts on it, in line order.
 *
 * A branch is listed at the line its condition starts on, which is nearly always a line a statement
 * starts on too. A condition that starts on a line no statement does still gets that line, and its
 * count is how often the condition ran: the two sides of its first branch, added together.
 */
export function statements(trace: Traced): Covered[] {
  const covered = new Map<number, Covered>();
  for (const { line, count } of trace.lines) {
    covered.set(line, { line, count, branches: [] });
  }
  const many = (line: number): boolean =>
    trace.branches.some((branch) => branch.line === line && branch.block > 0);
  for (const branch of trace.branches) {
    let held = covered.get(branch.line);
    if (held === undefined) {
      const ran = trace.branches
        .filter((other) => other.line === branch.line && other.block === branch.block)
        .reduce((sum, other) => sum + other.count, 0);
      held = { line: branch.line, count: ran, branches: [] };
      covered.set(branch.line, held);
    }
    const side = branch.side === 0 ? "true" : "false";
    const label = many(branch.line) ? `condition ${branch.block + 1}: ${side}` : side;
    held.branches.push({ count: branch.count, label });
  }
  return [...covered.values()].sort((a, b) => a.line - b.line);
}

/** A field that is a whole number of zero or more, or nothing when it is not one. */
function whole(field: string | undefined): number | undefined {
  return field !== undefined && /^\d+$/.test(field) ? Number(field) : undefined;
}

/** A document of one of `versions`, as an object, or nothing when it is not. */
function parse(text: string, versions: number[]): Record<string, unknown> | undefined {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return undefined;
  }
  if (typeof value !== "object" || value === null) {
    return undefined;
  }
  const document = value as Record<string, unknown>;
  return versions.includes(document.schemaVersion as number) ? document : undefined;
}

/** Whether a record names a test and carries the three location keys, whatever they hold. */
function located(value: unknown): boolean {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const test = value as Partial<Listed>;
  return (
    typeof test.class === "string" &&
    typeof test.method === "string" &&
    "file" in test &&
    "line" in test &&
    "column" in test
  );
}
