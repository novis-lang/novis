// `event <kind> …`: appends one progress event to the job's event file, which `LOOPER_EVENTS` names
// (`../jobs/events.ts`), as `crates/looper/src/cmd/event.rs` does. It is for a script in PowerShell,
// Python or `.cmd`, so the script never writes the JSON itself.
//
// - Each kind takes the event's fields as arguments, and the time is now. An option's value comes as
//   the next argument, or after `=` as in `--name=-x`.
// - With `LOOPER_EVENTS` unset or empty it writes nothing and exits 0, so a script runs the same
//   outside a job.
// - It prints nothing on stdout, and takes no `--json`. A line past the format's `MAX_LINE`, or a file
//   it cannot write, exits 1 with the reason on stderr.

import { EventRefused, appendEvent } from "../jobs/events.ts";
import type { Event, Outcome } from "../jobs/fold.ts";
import { type Command, Refusal, usage } from "./cli.ts";

const KINDS = "`add`, `planned`, `wait`, `start`, `tick`, `note`, `end`, `drop` or `done`";

/** The words and options of one `event` call. */
class Args {
  readonly words: string[] = [];
  private readonly values = new Map<string, string[]>();
  private readonly flags = new Set<string>();

  constructor(
    readonly kind: string,
    args: string[],
    takes: { values: readonly string[]; flags?: readonly string[] },
  ) {
    for (let i = 0; i < args.length; i++) {
      const arg = args[i] as string;
      if (!arg.startsWith("--")) {
        this.words.push(arg);
        continue;
      }
      const eq = arg.indexOf("=");
      const name = arg.slice(2, eq < 0 ? undefined : eq);
      if (takes.flags?.includes(name) && eq < 0) {
        this.flags.add(name);
        continue;
      }
      if (!takes.values.includes(name)) throw usage(`\`event ${kind}\` does not take \`--${name}\``);
      const value = eq < 0 ? args[++i] : arg.slice(eq + 1);
      if (value === undefined) throw usage(`\`--${name}\` takes a value`);
      this.values.set(name, [...(this.values.get(name) ?? []), value]);
    }
  }

  /** The words, which must be exactly `names`. */
  take(...names: string[]): string[] {
    if (this.words.length !== names.length) {
      const want = names.length === 0 ? "no words" : names.map((n) => `<${n}>`).join(" ");
      throw usage(`\`event ${this.kind}\` takes ${want}`);
    }
    return this.words;
  }

  value(name: string): string | undefined {
    return this.values.get(name)?.at(-1);
  }

  all(name: string): string[] {
    return this.values.get(name) ?? [];
  }

  flag(name: string): boolean {
    return this.flags.has(name);
  }
}

/** A whole number from `value`, the argument `name`. */
function whole(value: string, name: string): number {
  if (!/^\d+$/.test(value)) throw usage(`\`${name}\` takes a whole number`);
  return Number(value);
}

function outcome(value: string | undefined): Outcome | undefined {
  if (value === undefined) return undefined;
  if (!["green", "red", "skipped", "cancelled"].includes(value)) {
    throw usage("`--result` takes `green`, `red`, `skipped` or `cancelled`");
  }
  return value as Outcome;
}

/** The event `kind` with `rest` names. */
export function eventOf(kind: string | undefined, rest: string[]): Event {
  switch (kind) {
    case "add": {
      const a = new Args(kind, rest, { values: ["parent", "name", "kind", "green-at"], flags: ["kept"] });
      const [id] = a.take("id") as [string];
      const greenAt = a.value("green-at");
      if (greenAt !== undefined && !a.flag("kept")) throw usage("`--green-at` needs `--kept`");
      return {
        event: "add",
        id,
        parent: a.value("parent"),
        name: a.value("name"),
        kind: a.value("kind"),
        ...(a.flag("kept") ? { kept: true } : {}),
        green_at: greenAt === undefined ? undefined : whole(greenAt, "--green-at"),
      };
    }
    case "planned":
      new Args(kind, rest, { values: [] }).take();
      return { event: "planned" };
    case "wait": {
      const [id, on] = new Args(kind, rest, { values: [] }).take("id", "on") as [string, string];
      return { event: "wait", id, on };
    }
    case "start": {
      const a = new Args(kind, rest, { values: ["job"] });
      const [id] = a.take("id") as [string];
      return { event: "start", id, job: a.value("job") };
    }
    case "tick": {
      const a = new Args(kind, rest, { values: ["total"] });
      const [id, done] = a.take("id", "done") as [string, string];
      const total = a.value("total");
      return { event: "tick", id, done: whole(done, "done"), total: total === undefined ? undefined : whole(total, "--total") };
    }
    case "note": {
      const a = new Args(kind, rest, { values: ["id"] });
      const [text] = a.take("text") as [string];
      return { event: "note", id: a.value("id"), text };
    }
    case "end": {
      const a = new Args(kind, rest, { values: ["result", "why", "log", "artifact"] });
      const [id] = a.take("id") as [string];
      const artifacts = a.all("artifact");
      return {
        event: "end",
        id,
        result: outcome(a.value("result")),
        why: a.value("why"),
        log: a.value("log"),
        ...(artifacts.length > 0 ? { artifacts } : {}),
      };
    }
    case "drop": {
      const [id] = new Args(kind, rest, { values: [] }).take("id") as [string];
      return { event: "drop", id };
    }
    case "done": {
      const a = new Args(kind, rest, { values: ["result"] });
      a.take();
      return { event: "done", result: outcome(a.value("result")) };
    }
    default:
      throw usage(kind === undefined ? `\`event\` takes ${KINDS}` : `\`event\` takes ${KINDS}, and got \`${kind}\``);
  }
}

export const eventCommand: Command = {
  name: "event",
  about: "Appends one progress event to the job's event file: `event add|planned|wait|start|tick|note|end|drop|done`.",
  json: false,
  run: (args) => {
    const [kind, ...rest] = args;
    const event = eventOf(kind, rest);
    try {
      appendEvent(event);
    } catch (e) {
      if (e instanceof EventRefused) throw new Refusal(e.message);
      throw e;
    }
    return {};
  },
};
