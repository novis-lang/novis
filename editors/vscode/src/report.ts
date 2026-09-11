// The two documents `nvs test` prints, and the tree the explorer builds out of them.
//
// Discovery reads `nvs test --list --format=json` and a run reads `nvs test --format=json`, both
// `schemaVersion: 2` and both written by `crates/nvs-cli/src/runner.rs`. They are two readings of
// one schema and the key that carries the data says which is which: a listing has `listed` and no
// summary, because a summary of zeros would be a report of a run where nothing passed. Nothing
// here re-derives a field either document should have carried — a client feature whose CLI surface
// is missing is the CLI's problem, never a parse of the human rendering.
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

/** The version both documents carry at their root, and the first key a consumer reads. */
const SCHEMA = 2;

/**
 * The listing `nvs test --list --format=json` printed, or nothing when it is not one.
 *
 * A run document is refused here rather than accepted for its records: it has `tests` and a
 * summary, and reading one as a listing would show the explorer a tree built out of a run that
 * already happened.
 */
export function listed(text: string): Listed[] | undefined {
  const document = parse(text);
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
  const document = parse(text);
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

/** A document of this schema, as an object, or nothing when it is neither. */
function parse(text: string): Record<string, unknown> | undefined {
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
  return document.schemaVersion === SCHEMA ? document : undefined;
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
