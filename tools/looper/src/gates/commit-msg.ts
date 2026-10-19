// The commit message check `hook commit-msg` runs, as `crates/looper/src/cmd/hook.rs` makes it. A
// message breaks the shape `docs/agent/conventions.md` § *A commit message* gives when:
//
// - its subject is not `type(scope): what is now true`, with a type and a length that `[commits]`
//   allows (`subjectProblems`);
// - a body is not parted from the subject by a blank line;
// - a line is a trailer. A trailer is a line whose key is one of `TRAILER_KEYS`, a line such as
//   `Generated with [Tool](url)`, or any last paragraph whose every line is `Key: value`, which is the
//   trailer block git itself reads.
//
// Lines that start with `#` are git's comments and are skipped. The subject of the loop's own sweep
// commit, `SWEEP_SUBJECT` and the session's number, skips the subject check. The rest of its message
// is checked.

import type { Commits } from "../goal/config.ts";
import { lines as textLines } from "../goal/model.ts";

/** The trailer keys refused wherever they stand, compared without case. */
export const TRAILER_KEYS = [
  "co-authored-by",
  "signed-off-by",
  "generated-by",
  "generated-with",
  "assisted-by",
  "authored-by",
  "reviewed-by",
  "on-behalf-of",
  "committed-by",
  "created-by",
  "made-with",
];

/** The subject of the sweep commit, before ` session ` and the session's number. */
export const SWEEP_SUBJECT = "wip(loop): the unfinished slice of";

const KEY_VALUE = /^[A-Za-z0-9][A-Za-z0-9-]*:\s+\S/;
const PROSE = /^[^a-z`]*(generated|created|made)\s+(with|by)\s+\[/i;

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** Every problem with the subject line `subject` under `rules`, one line each. */
export function subjectProblems(subject: string, rules: Commits): string[] {
  const shape = new RegExp(`^(${rules.types.map(escapeRegex).join("|")})(\\([a-z0-9-]+\\))?: \\S.*$`);
  const problems: string[] = [];
  if (!shape.test(subject)) {
    problems.push(`the subject \`${subject}\` is not \`type(scope): what is now true\`, with a type of ${rules.types.join(", ")}`);
  }
  const length = Array.from(subject).length;
  if (length > rules.subject_max) {
    problems.push(`the subject is ${length} characters, and may be at most ${rules.subject_max}`);
  }
  return problems;
}

/** Whether `subject` is the one the sweep commit writes: `SWEEP_SUBJECT`, ` session ` and a number of at least four digits. */
function isSweep(subject: string): boolean {
  const prefix = `${SWEEP_SUBJECT} session `;
  return subject.startsWith(prefix) && /^[0-9]{4,}$/.test(subject.slice(prefix.length));
}

/** Every problem with the commit message `text` under `rules`, one line each. */
export function check(text: string, rules: Commits): string[] {
  const lines = textLines(text)
    .map((l, i): [number, string] => [i + 1, l.trimEnd()])
    .filter(([, l]) => !l.startsWith("#"));
  const first = lines[0];
  if (first === undefined) return ["the message is empty"];
  const subject = first[1];
  const problems = isSweep(subject) ? [] : subjectProblems(subject, rules);
  const second = lines[1];
  if (second !== undefined && second[1] !== "") problems.push("the second line is not blank");
  const body = lines.slice(1);
  for (const [n, line] of body) {
    const colon = line.indexOf(":");
    const key = colon < 0 ? undefined : line.slice(0, colon).trim().toLowerCase();
    if ((key !== undefined && TRAILER_KEYS.includes(key)) || PROSE.test(line)) {
      problems.push(`line ${n} is a trailer: ${line}`);
    }
  }
  // The last paragraph of the body: the lines after its last blank one.
  let end = body.length;
  while (end > 0 && body[end - 1]?.[1] === "") end--;
  let start = end;
  while (start > 0 && body[start - 1]?.[1] !== "") start--;
  const last = body.slice(start, end);
  const startsAfterBlank = (last[0]?.[0] ?? 0) > 2;
  if (startsAfterBlank && last.every(([, l]) => KEY_VALUE.test(l))) {
    for (const [n, line] of last) {
      const named = `line ${n} is a trailer: ${line}`;
      if (!problems.includes(named)) problems.push(named);
    }
  }
  return problems;
}
