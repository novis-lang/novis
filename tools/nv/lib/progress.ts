// What a long `nv` command is doing while it does it: a build, a roster read, a pool of programs. The
// text never goes into what the command prints, because an agent reads that output and pays for every
// line of it. With stderr a terminal, it is one line on stderr that is rewritten in place and ends in how
// long the command has run. Any write to stdout or stderr erases the line first, so the output a person
// reads is the same as without it, and the next `progress` call paints the line again. Otherwise, which
// is every shell call an agent makes, it goes nowhere.

const LINE_EVERY_MS = 100;
const TICK_MS = 1000;

const started = performance.now();
let text = "";
let lineAt = 0;
let lineTimer: ReturnType<typeof setTimeout> | null = null;
let ticker: ReturnType<typeof setInterval> | null = null;
/** Whether the terminal line is on screen now. */
let showing = false;
let hooked = false;

const rawErr = process.stderr.write.bind(process.stderr);

/** `m:ss` since the command started. */
function elapsed(): string {
  const s = Math.floor((performance.now() - started) / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

/** Says what the command is doing now. Each call replaces the text of the one before it. */
export function progress(now: string): void {
  text = now.replace(/[\r\n]+/g, " ");
  if (process.stderr.isTTY) paintSoon();
}

function paintSoon(): void {
  hook();
  showing = true;
  if (lineTimer !== null) return;
  const wait = Math.max(0, lineAt + LINE_EVERY_MS - performance.now());
  if (wait > 0) {
    lineTimer = setTimeout(paint, wait);
    lineTimer.unref();
  } else paint();
}

function paint(): void {
  lineTimer = null;
  if (!showing) return;
  lineAt = performance.now();
  const clock = ` ${elapsed()}`;
  const room = Math.max(10, (process.stderr.columns ?? 100) - 1 - clock.length);
  const chars = Array.from(text);
  const shown = chars.length <= room ? text : `${chars.slice(0, room - 1).join("")}…`;
  // The cursor goes back to the first column, so even a write that does not erase starts there.
  rawErr(`\r${shown}${clock}\x1b[K\r`);
}

/** Erases the terminal line, for a child that writes to the same terminal. The next `progress` call paints
 * it again. */
export function erase(): void {
  if (!showing) return;
  showing = false;
  rawErr("\r\x1b[K");
}

/** Makes every write to stdout and stderr erase the line first, and keeps the clock moving. */
function hook(): void {
  if (hooked) return;
  hooked = true;
  for (const stream of [process.stdout, process.stderr]) {
    const write = stream.write.bind(stream) as (...a: unknown[]) => boolean;
    stream.write = ((...a: unknown[]) => {
      erase();
      return write(...a);
    }) as typeof stream.write;
  }
  // Bun's console writes to the file descriptors directly, not through `stream.write`.
  for (const name of ["log", "info", "warn", "error", "debug"] as const) {
    const f = console[name].bind(console);
    console[name] = (...a: unknown[]) => {
      erase();
      f(...a);
    };
  }
  ticker = setInterval(() => showing && paint(), TICK_MS);
  ticker.unref();
  process.on("exit", erase);
}

/**
 * Reads cargo's status lines one at a time and returns what each one says, `compiling nvs-stdlib (43
 * crates)`, or null for any other line. cargo writes those lines to stderr whether or not stderr is a
 * terminal, and beside `--message-format=json` too.
 */
export function cargoStatus(): (line: string) => string | null {
  let units = 0;
  return (line) => {
    const m = /^\s*(Compiling|Checking|Documenting|Running|Finished|Blocking|Updating|Downloading|Downloaded|Locking)\s+(.*)$/.exec(line);
    if (!m) return null;
    const verb = m[1]!;
    const what = m[2]!.split(" ")[0] ?? "";
    if (verb === "Compiling" || verb === "Checking" || verb === "Documenting") units++;
    if (verb === "Finished") return "finished";
    return `${verb.toLowerCase()} ${what}${units ? ` (${units} crate${units === 1 ? "" : "s"})` : ""}`;
  };
}

/** A `proc.run` `onLine` that shows cargo's status lines as progress: `<label>: compiling nvs-stdlib (43 crates)`. */
export function cargoLines(label: string): (line: string) => void {
  const status = cargoStatus();
  return (line) => {
    const said = status(line);
    if (said !== null) progress(`${label}: ${said}`);
  };
}
