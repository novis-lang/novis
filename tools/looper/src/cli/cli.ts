// The kit's CLI frame: it picks the command named by the first argument, and gives every command one
// `--json` flag and one set of exit codes. `docs/kit-contract.md` § *The CLI* is the contract a reader
// of the output relies on.
//
// - `--json` is global: it may stand anywhere before a `--`. A command that takes it returns one
//   document, and the frame prints it as one line on stdout. The progress lines a command writes
//   through `Context.line` then go to stderr, so stdout holds only the document.
// - A command reports a refusal by throwing `Refusal`. The frame writes `error: <why>` on stderr, and
//   with `--json` also prints `{"error": "<why>"}` on stdout. Any other error a command throws is a
//   refusal too, with exit 1, so a reader still finds one document.
// - A command whose `json` is false refuses `--json` with exit 2, as looper's do.
// - A command that hands the terminal to a child, as a `[gates]` command does, returns `own`. The
//   frame then prints nothing, and the child's output is the command's.
// - The frame keeps no state between calls. A command's text or document is written once, when it
//   returns.

/** The exit codes every command keeps. */
export const Exit = {
  ok: 0,
  /** Refused, or a problem found. */
  refused: 1,
  /** The arguments are wrong. */
  usage: 2,
  /** The looper service does not answer. */
  serviceAbsent: 3,
  /** Another worker runs the loop. */
  taken: 4,
} as const;

/** A command's refusal: `why` is the sentence a person reads, and `code` the exit code. */
export class Refusal extends Error {
  constructor(
    why: string,
    readonly code: number = Exit.refused,
  ) {
    super(why);
  }
}

/** A refusal for wrong arguments, which exits 2. */
export function usage(why: string): Refusal {
  return new Refusal(why, Exit.usage);
}

/** Where the frame writes. A test passes its own to read what a command printed. */
export interface Io {
  out(text: string): void;
  err(text: string): void;
}

/** What a command gets besides its arguments. */
export interface Context {
  json: boolean;
  /** Writes a progress line: on stdout, or with `--json` on stderr. */
  line(text: string): void;
  /** Writes a line on stderr, with or without `--json`. */
  err(text: string): void;
}

/** What a command returns. `doc` is printed with `--json`, and `text` without it. */
export interface Report {
  code?: number;
  doc?: unknown;
  text?: string;
  /** The command's output went to the terminal as a child wrote it, so the frame prints nothing. */
  own?: boolean;
}

export interface Command {
  name: string;
  /** One line for the usage text. */
  about: string;
  /** Whether the command takes `--json`. */
  json: boolean;
  run(args: string[], ctx: Context): Report | Promise<Report>;
}

export const processIo: Io = {
  out: (text) => void process.stdout.write(text),
  err: (text) => void process.stderr.write(text),
};

/** The usage text: the program and each command with its line. */
export function usageText(commands: readonly Command[]): string {
  const width = Math.max(0, ...commands.map((c) => c.name.length));
  const lines = commands.map((c) => `  ${c.name.padEnd(width)}  ${c.about}`);
  return ["Usage: looper-kit [--json] <command> [args]", "", "Commands:", ...lines, ""].join("\n");
}

/** Runs the command `argv` names, and gives the exit code. */
export async function run(argv: readonly string[], commands: readonly Command[], io: Io = processIo): Promise<number> {
  const end = argv.indexOf("--");
  const head = end < 0 ? argv : argv.slice(0, end);
  const json = head.includes("--json");
  const args = [...head.filter((a) => a !== "--json"), ...(end < 0 ? [] : argv.slice(end))];

  const refuse = (why: string, code: number): number => {
    io.err(`error: ${why}\n`);
    if (json) io.out(`${JSON.stringify({ error: why })}\n`);
    return code;
  };

  const [name, ...rest] = args;
  if (name === undefined) {
    io.err(usageText(commands));
    return refuse("name a command, as `--help` lists them", Exit.usage);
  }
  if (name === "--help" || name === "-h" || name === "help") {
    io.out(usageText(commands));
    return Exit.ok;
  }
  const command = commands.find((c) => c.name === name);
  if (command === undefined) return refuse(`unknown command \`${name}\`: \`--help\` lists the commands`, Exit.usage);
  if (json && !command.json) {
    io.err(`error: \`${name}\` takes no --json\n`);
    return Exit.usage;
  }

  const ctx: Context = { json, line: (text) => (json ? io.err : io.out)(`${text}\n`), err: (text) => io.err(`${text}\n`) };
  let report: Report;
  try {
    report = await command.run(rest, ctx);
  } catch (e) {
    if (e instanceof Refusal) return refuse(e.message, e.code);
    return refuse(e instanceof Error ? e.message : String(e), Exit.refused);
  }
  if (report.own) return report.code ?? Exit.ok;
  if (json) io.out(`${JSON.stringify(report.doc ?? {})}\n`);
  else if (report.text !== undefined) io.out(report.text.endsWith("\n") ? report.text : `${report.text}\n`);
  return report.code ?? Exit.ok;
}
