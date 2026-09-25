// The driver's console. The session's transcript and the
// driver's own steps scroll up the terminal, and a live block of three rows stays under them:
//
//   ─────────────────────────────────────────────  a grey rule
//   ⠋ goal tooling-overhaul/8 | 64% | 84.2kin/6.1kout | 42 tool calls | session 14 | Run the guard tests | 23m41s
//     [g] goal table · [h] halt now · [i] type a prompt · [s] stop after this session · [p] hold after …
//
// The status line's words are `driver/status.ts`'s `statusRow`, which `row` is set to; this module paints
// it white behind a cyan spinner, cut at the terminal's width, and ends it with how long the current phase
// has been going. A session is one phase from launch to exit, so while one runs that clock is the session's.
// Between two sessions the row's last field is the driver's phase, `phase()`: `acceptance sweep 31/58`,
// `usage wall, 12m03s left`, `held`. The window title is the same row without the clock, which would
// rewrite the title every second. The key row lists only the keys that do something now, and is left out when
// stdin is not a console.
//
// `say` erases the block, writes its line and paints the block again, and a ticker repaints it eight times
// a second, so the spinner moves while the driver waits on a build or a session. Every line ends in CR LF
// on a terminal: a Windows console can be in a mode where a bare LF moves down a row without going back
// to the first column, and every later line then starts where the last one ended. Each painted line also
// ends by resetting its colour. With stdout not a terminal, nothing is painted and no colour is written.
//
// `Control` is the keys, and the files under `.loop/` that do what the keys do from another terminal:
//   - `r` ends a usage-limit or backoff wait (`.loop/retry`);
//   - `s` stops the run after the current session, and pressed again within five seconds takes that
//     back (`.loop/stop`);
//   - `p` holds the run between two sessions (`.loop/pause`, whose `held:` line says the hold took
//     effect). A hold taken with `p` is lifted only with `p`; one another agent took by writing the file
//     is lifted by deleting it, or with `p`;
//   - `h` freezes the running session and every process under it, and again lets it carry on
//     (`.loop/halt`);
//   - `i` freezes the session while a prompt is typed, and Enter sends the prompt to it as a user
//     message (`.loop/say` sends that file's text);
//   - `g` prints the goal as a table into the scrollback.
// On Windows the keys are read with the C runtime's `_kbhit` and `_getwch`, which leave the console's
// Ctrl-C as it is. Elsewhere stdin goes into raw mode, and a Ctrl-C byte calls the interrupt handler.
//
// Everything said is also written, stamped, to `.loop/logs/<run>-console.log`, and the driver's own lines
// go into the running session's log as `loop_console` events.

import { dlopen, FFIType } from "bun:ffi";
import { appendFileSync, existsSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { cookConsole } from "../lib/tty.ts";
import type { Tree } from "./proctree.ts";
import { cut, keyRow } from "./status.ts";

const WIN = process.platform === "win32";
const OUT_TTY = process.stdout.isTTY === true;
/** How a line ends on the console; see the module doc. */
const EOL = OUT_TTY ? "\r\n" : "\n";

export const STOP = ".loop/stop";
export const PAUSE = ".loop/pause";
export const RETRY = ".loop/retry";
export const HALT = ".loop/halt";
export const SAY = ".loop/say";
const VERIFY_PROGRESS = ".agent-tmp/verify-progress.json";
const SESSION_BASE = ".loop/session-start.json";

const abs = (path: string) => join(ROOT, path);

// ------------------------------------------------------------------------------------------ colours

export const C = {
  RESET: "\x1b[0m",
  GRAY: "\x1b[90m",
  RED: "\x1b[31m",
  GREEN: "\x1b[32m",
  YELLOW: "\x1b[33m",
  BLUE: "\x1b[34m",
  MAGENTA: "\x1b[35m",
  CYAN: "\x1b[36m",
  WHITE: "\x1b[37m",
  DIM_CYAN: "\x1b[36;2m",
  DIM_GREEN: "\x1b[32;2m",
} as const;

let coloured = OUT_TTY || process.env.FORCE_COLOR === "1";

export function paint(text: string, colour: string): string {
  return coloured ? `${colour}${text}${C.RESET}` : text;
}

/**
 * Sets the Windows console's modes through `lib/tty.ts` now, without waiting for its next tick, and turns
 * colour off when the console does not take escape sequences. Called when the ticker starts and after
 * every session. Does nothing elsewhere, or when stdout is not a console.
 */
export function consoleMode(): void {
  if (WIN && OUT_TTY && !cookConsole()) coloured = false;
}

// -------------------------------------------------------------------------------------------- times

/** `123` is `2m03s`. */
export function mmss(seconds: number): string {
  const s = Math.max(0, Math.trunc(seconds));
  return s >= 60 ? `${Math.floor(s / 60)}m${String(s % 60).padStart(2, "0")}s` : `${s}s`;
}

/** `mmss` below an hour, `4h52m` above it. */
export function hms(seconds: number): string {
  const s = Math.max(0, Math.trunc(seconds));
  return s < 3600 ? mmss(s) : `${Math.floor(s / 3600)}h${String(Math.floor((s % 3600) / 60)).padStart(2, "0")}m`;
}

/** `84213` is `84.2k`. */
export function ktok(tokens: number): string {
  return tokens >= 1000 ? `${(tokens / 1000).toFixed(1)}k` : String(tokens);
}

const pad2 = (n: number) => String(n).padStart(2, "0");
export const clock = (d = new Date()) => `${pad2(d.getHours())}:${pad2(d.getMinutes())}:${pad2(d.getSeconds())}`;
const now = () => performance.now() / 1000;

// -------------------------------------------------------------------------------------- console log

/** The file every said line is also written to, and the session log that gets the driver's own lines. */
class ConsoleLog {
  run = "";
  session = "";

  openRun(path: string): void {
    this.run = path;
    try {
      mkdirSync(dirname(path), { recursive: true });
    } catch {
      this.run = "";
    }
  }

  openSession(path: string): void {
    this.session = path;
  }

  closeSession(): void {
    this.session = "";
  }

  line(text: string, driver: boolean): void {
    if (!this.run && !(driver && this.session)) return;
    const d = new Date();
    const stamp = `${clock(d)}.${String(d.getMilliseconds()).padStart(3, "0")}`;
    try {
      if (this.run) appendFileSync(this.run, text.split("\n").map((ln) => `[${stamp}] ${ln}\n`).join(""));
      if (driver && this.session) appendFileSync(this.session, `${JSON.stringify({ type: "loop_console", ts: stamp, text })}\n`);
    } catch {
      // A log that cannot be written never ends a run.
    }
  }

  /** A line straight into the session's log, unstamped: the harness's own event stream. */
  raw(line: string): void {
    try {
      if (this.session) appendFileSync(this.session, line);
    } catch {
      // As above.
    }
  }
}

export const CONSOLE = new ConsoleLog();

// --------------------------------------------------------------------------------- the live block

class StatusLine {
  static readonly BRAILLE = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";
  static readonly ASCII = "|/-\\";
  static readonly INTERVAL = 120;
  static readonly TITLE_ROOM = 120;

  enabled = false;
  utf8 = false;
  frames = StatusLine.ASCII;
  bar = "-";
  dash = " -- ";
  /** The status line's words at a width; `turn` sets it to the goal's `statusRow`. */
  row: (width: number) => string = (width) => cut(this.phase(), width);
  private phaseName = "";
  /** When the current phase began: the clock at the status line's end counts from here. */
  private since = now();
  private detail = "";
  private done = 0;
  private total = 0;
  private frame = 0;
  /** Rows the block holds on screen now; 0 when it is not drawn. */
  private rows = 0;
  private lastTitle = "";
  private timer: ReturnType<typeof setInterval> | null = null;

  constructor() {
    // Unicode wherever the terminal takes it: every Windows console Bun writes to does, since Bun writes UTF-16.
    const locale = process.env.LC_ALL || process.env.LC_CTYPE || process.env.LANG || "";
    if (WIN || /utf-?8/i.test(locale)) {
      this.utf8 = true;
      this.frames = StatusLine.BRAILLE;
      this.bar = "─";
      this.dash = " — ";
    }
  }

  /** Begins painting, when stdout is a terminal that takes colour. */
  start(): void {
    if (this.enabled || !(coloured && OUT_TTY)) return;
    consoleMode();
    this.enabled = true;
    this.lastTitle = "";
    this.since = now();
    this.timer = setInterval(() => this.tick(), StatusLine.INTERVAL);
  }

  /** Erases the block, gives the window title back, and stops painting. */
  stop(): void {
    if (this.timer !== null) clearInterval(this.timer);
    this.timer = null;
    this.erase();
    if (this.enabled) {
      process.stdout.write("\x1b]0;\x07");
      this.lastTitle = "";
    }
    this.enabled = false;
  }

  private tick(): void {
    CONTROL.poll();
    VERIFY.poll();
    this.frame++;
    this.draw();
  }

  /** Sets the driver's phase. A new phase clears its detail and its count, and starts the clock again. */
  set(f: { phase?: string; detail?: string; total?: number; done?: number }): void {
    if (f.phase !== undefined && f.phase !== this.phaseName) {
      this.phaseName = f.phase;
      this.since = now();
      this.detail = "";
      this.done = this.total = 0;
    }
    if (f.detail !== undefined) this.detail = f.detail;
    if (f.total !== undefined) this.total = f.total;
    if (f.done !== undefined) this.done = f.done;
    this.draw();
  }

  /** Every planned step is behind the phase, and `steps` more follow as the part named `detail`: a sweep's Linux legs, counted once they know their work. */
  extend(steps: number, detail: string): void {
    this.done = this.total;
    this.total += steps;
    this.detail = detail;
    this.draw();
  }

  /** One planned step of the phase is finished. */
  advance(): void {
    this.done++;
    this.draw();
  }

  /** The driver's phase as the status line's last field: `acceptance sweep 31/58`, `usage wall, 12m03s left`, `held`. */
  phase(): string {
    const head = this.total > 0 ? `${this.phaseName} ${Math.min(this.done, this.total)}/${this.total}` : this.phaseName;
    return this.detail ? `${head}, ${this.detail}` : head;
  }

  width(): number {
    return process.stdout.columns ?? 100;
  }

  /** How long the current phase has been going; the whole session while one runs, since its phase lasts that long. */
  elapsed(): string {
    return mmss(now() - this.since);
  }

  private statusLine(): string {
    // One column short of the room: a line that exactly fills the terminal wraps, and the next erase half removes it.
    // The clock is kept and the row is cut, so the clock is always read.
    const clock = ` | ${this.elapsed()}`;
    const room = Math.max(18, this.width() - 3 - clock.length);
    const spin = this.frames[this.frame % this.frames.length]!;
    return `${paint(spin, C.CYAN)} ${paint(cut(this.row(room).replace(/\n/g, " "), room) + clock, C.WHITE)}`;
  }

  private divider(): string {
    return paint(this.bar.repeat(Math.max(18, this.width() - 1)), C.GRAY);
  }

  private keys(): string {
    const room = Math.max(18, this.width() - 1);
    // The typed text ends the row, so the terminal's cursor stands right after its last character.
    if (CONTROL.typing !== null) return paint(keyRow([], CONTROL.typing, room, this.utf8), C.YELLOW);
    const bits: string[] = ["[g] goal table"];
    const s = CONTROL.session;
    if (s && s.frozenFor.size > 0) bits.push(`[h] HALTED${this.dash}press h to carry on`);
    else if (s) bits.push("[h] halt now");
    if (s && s.streaming && s.frozenFor.size === 0) bits.push("[i] type a prompt");
    if (CONTROL.parked) bits.push("[r] retry now");
    if (CONTROL.stop) {
      const grace = CONTROL.stopIn();
      bits.push(grace > 0 ? `[s] stopping in ${grace.toFixed(0)}s${this.dash}press s to cancel` : `[s] stopping after this session${this.dash}press s to cancel`);
    } else {
      bits.push("[s] stop after this session");
    }
    if (CONTROL.held) bits.push(`[p] held${this.dash}press p to carry on`);
    else if (CONTROL.pauseBy) bits.push(`[p] holding after this session${this.dash}press p to cancel`);
    else bits.push("[p] hold after this session");
    const loud = CONTROL.stop || CONTROL.pauseBy !== "" || (s !== null && s.frozenFor.size > 0);
    return paint(keyRow(bits, null, room, this.utf8), loud ? C.YELLOW : C.GRAY);
  }

  /** The window title: the status line itself. */
  private title(): string {
    // A title ends at the first BEL or ESC, so a control character in a tool's description is blanked.
    // biome-ignore lint/suspicious/noControlCharactersInRegex: the control characters are what is blanked
    return this.row(StatusLine.TITLE_ROOM).replace(/[\x00-\x1f\x7f]/g, " ");
  }

  /** The title's escape sequence, or "" when the title already says this. */
  private titleSeq(): string {
    const text = this.title();
    if (text === this.lastTitle) return "";
    this.lastTitle = text;
    return `\x1b]0;${text}\x07`;
  }

  private height(): number {
    return CONTROL.tty ? 3 : 2;
  }

  /** The sequence that clears every row of the block and leaves the cursor at the start of the rule's row. */
  eraseSeq(): string {
    if (!this.rows) return "";
    const out = "\r\x1b[2K" + "\x1b[A\r\x1b[2K".repeat(this.rows - 1);
    this.rows = 0;
    return out;
  }

  erase(): void {
    const out = this.eraseSeq();
    if (out) process.stdout.write(out);
  }

  /**
   * The sequence that repaints every row of the block in place. The rows under the first are claimed once,
   * with line ends, and every frame after moves between them with cursor-up and cursor-down, which never
   * scroll.
   */
  drawSeq(): string {
    if (!this.enabled) return "";
    const want = this.height();
    let out = this.rows && this.rows !== want ? this.eraseSeq() : "";
    out += this.titleSeq();
    if (!this.rows) out += EOL.repeat(want - 1);
    out += `\x1b[${want - 1}A\r\x1b[2K${this.divider()}`;
    for (const line of [this.statusLine(), ...(want > 2 ? [this.keys()] : [])]) out += `\x1b[B\r\x1b[2K${line}`;
    this.rows = want;
    return out;
  }

  draw(): void {
    const out = this.drawSeq();
    if (out) process.stdout.write(out);
  }
}

export const TICKER = new StatusLine();

/** One line on the console, above the live block, and in the console log. `driver` marks the driver's own line. */
export function say(text = "", colour?: string, driver = false): void {
  CONSOLE.line(text, driver);
  const body = text
    .split("\n")
    .map((ln) => (coloured ? C.RESET : "") + (colour ? paint(ln, colour) : ln))
    .join(EOL);
  process.stdout.write(`${TICKER.eraseSeq()}${OUT_TTY ? "\r" : ""}${body}${EOL}${TICKER.drawSeq()}`);
}

/** One line of the driver's progress between sessions, stamped with the wall clock. */
export function step(text: string, colour: string = C.GRAY): void {
  say(`   [${clock()}] ${text}`, colour, true);
}

/** The one line of a session a person has to read: whether the loop still drives itself. */
export function verdict(hand: boolean, text: string): void {
  say("");
  say(`   ==> ${hand ? "YOUR HAND IS NEEDED" : "nothing for you to do"} -- ${text}`, hand ? C.RED : C.GREEN);
}

/** Sleeps `seconds`, with the status line counting down; `until` is looked at four times a second and ends the wait early. */
export async function wait(seconds: number, until?: () => boolean): Promise<void> {
  const end = now() + seconds;
  for (;;) {
    const left = end - now();
    if (left <= 0) return;
    CONTROL.poll();
    if (until?.()) return;
    TICKER.set({ detail: `${hms(left)} left` });
    await Bun.sleep(Math.min(250, left * 1000));
  }
}

// ------------------------------------------------------------------------------ the running session

/** The `claude` child in flight, as the keys see it: something to halt, and something to talk to. */
export class LiveSession {
  /** Why it is frozen: `halt`, `typing`, or both. It thaws when the last reason goes. */
  frozenFor = new Set<string>();
  frozenAt = 0;
  halts = 0;
  /** Seconds spent frozen, over every freeze. */
  halted = 0;
  prompts = 0;

  constructor(
    readonly tree: Tree,
    private readonly write: (line: string) => boolean,
    private readonly open: () => boolean,
  ) {}

  /** Whether it still takes a message: false once its reply is complete and its stdin is closed. */
  get streaming(): boolean {
    return this.open();
  }

  freeze(why: string): number {
    if (this.frozenFor.size === 0) {
      this.frozenAt = now();
      this.halts++;
    }
    this.frozenFor.add(why);
    return this.tree.freeze();
  }

  /** `why` no longer holds. Thaws when nothing else does, and returns the seconds spent frozen. */
  thaw(why: string): number {
    if (!this.frozenFor.delete(why) || this.frozenFor.size > 0) return 0;
    const spent = now() - this.frozenAt;
    this.halted += spent;
    this.tree.thaw();
    return spent;
  }

  /** One user message down stdin. False when the session no longer takes any. */
  send(text: string, operator = true): boolean {
    if (!this.streaming) return false;
    const ok = this.write(`${JSON.stringify({ type: "user", message: { role: "user", content: [{ type: "text", text }] } })}\n`);
    if (ok && operator) this.prompts++;
    return ok;
  }

  /** What a person did to this session, for its ledger line; "" when nobody did anything. */
  record(): string {
    const bits: string[] = [];
    if (this.halts) bits.push(`halted ${this.halts}x for ${hms(this.halted)}`);
    if (this.prompts) bits.push(`${this.prompts} operator prompt(s)`);
    return bits.join(", ");
  }

  finish(): void {
    this.frozenFor.clear();
    this.tree.close();
  }
}

// -------------------------------------------------------------------------------------------- keys

type KeysApi = ReturnType<typeof loadKeys>;

function loadKeys() {
  return dlopen("msvcrt.dll", {
    _kbhit: { args: [], returns: FFIType.i32 },
    _getwch: { args: [], returns: FFIType.u16 },
  }).symbols;
}

class Control {
  /** Seconds an armed stop waits before it is acted on: the window to take it back. */
  static readonly STOP_GRACE = 5;
  /** Seconds between looks at `.loop/pause`, `.loop/halt` and `.loop/say`. */
  static readonly FILE_POLL = 0.5;

  stop = false;
  stopAt = 0;
  retry = false;
  /** A wait is running that `r` ends. */
  parked = false;
  /** Who owns the armed hold: `user`, `agent` or "". */
  pauseBy = "";
  /** The verdict a hold the driver armed waits on, or "" for any other hold. */
  pauseWhy = "";
  /** The hold has taken effect: the driver sits between two sessions. */
  held = false;
  session: LiveSession | null = null;
  /** The prompt being typed after `i`, or null while keys are commands. */
  typing: string | null = null;
  tty = false;
  /** Called on a Ctrl-C that arrives as a character. */
  onInterrupt: () => void = () => process.kill(process.pid, "SIGINT");
  /** Called on `g`: prints the goal table. */
  onGoal: () => void = () => say("   [g] no goal is loaded yet", C.GRAY);
  private pauseSeen = 0;
  private sessionSeen = 0;
  private keysApi: KeysApi | null = null;
  private queued: string[] = [];
  private raw = false;
  private readonly onData = (chunk: Buffer | string) => {
    this.queued.push(...String(chunk));
  };

  /** Takes the console's keys, when stdin is one. */
  enable(): void {
    if (this.tty || process.stdin.isTTY !== true) return;
    try {
      if (WIN) {
        this.keysApi = loadKeys();
      } else {
        process.stdin.setRawMode(true);
        this.raw = true;
        process.stdin.on("data", this.onData);
        process.stdin.resume();
      }
      this.tty = true;
    } catch {
      this.tty = false;
    }
  }

  /** Gives the terminal back as it was found. */
  disable(): void {
    if (this.raw) {
      try {
        process.stdin.off("data", this.onData);
        process.stdin.setRawMode(false);
        process.stdin.pause();
      } catch {
        // A terminal that is already gone.
      }
      this.raw = false;
    }
    this.tty = false;
  }

  /** What is already typed, without waiting for anything. */
  private typed(): string[] {
    if (!this.tty) return [];
    if (!WIN) return this.queued.splice(0);
    const keys: string[] = [];
    try {
      while (this.keysApi!._kbhit()) {
        const ch = String.fromCharCode(this.keysApi!._getwch());
        // A function or arrow key: its second half is dropped.
        if (ch === "\x00" || ch === "\xe0") {
          this.keysApi!._getwch();
          continue;
        }
        keys.push(ch);
      }
    } catch {
      this.tty = false;
    }
    return keys;
  }

  /** One look at the console and the files; called by the ticker and by `wait`. */
  poll(): void {
    for (const ch of this.typed()) {
      if (ch === "\x03") {
        this.onInterrupt();
        continue;
      }
      if (this.typing !== null) {
        this.type(ch);
        continue;
      }
      const key = ch.toLowerCase();
      if (key === "h") this.toggleHalt();
      else if (key === "i") this.beginTyping();
      else if (key === "s") {
        this.stop = !this.stop;
        this.stopAt = now() + Control.STOP_GRACE;
        say(
          this.stop ? `   [s] stop requested -- the run ends after the current session, unless s is pressed again within ${Control.STOP_GRACE}s` : "   [s] stop cancelled -- the run carries on",
          this.stop ? C.YELLOW : C.GREEN,
        );
      } else if (key === "r") {
        if (this.parked) {
          this.retry = true;
          say("   [r] retry requested -- the wait ends now", C.GREEN);
        } else {
          say("   [r] does nothing right now; it ends a usage or overload wait", C.GRAY);
        }
      } else if (key === "p") this.togglePause();
      else if (key === "g") this.onGoal();
    }
    this.syncPause();
    this.syncSession();
  }

  attach(session: LiveSession): void {
    rmSync(abs(HALT), { force: true });
    this.session = session;
    this.sessionSeen = 0;
  }

  detach(): void {
    this.session = null;
    this.typing = null;
    rmSync(abs(HALT), { force: true });
  }

  private toggleHalt(): void {
    if (!this.session) {
      say("   [h] does nothing right now; it halts a running session", C.GRAY);
      return;
    }
    if (existsSync(abs(HALT))) rmSync(abs(HALT), { force: true });
    else writeQuiet(HALT, "The running session is frozen where it stands, with every process under it.\nDelete this file, or press h at the console, to let it carry on.\n");
    this.sessionSeen = 0;
    this.syncSession();
  }

  private syncSession(): void {
    const s = this.session;
    const t = now();
    if (!s || t - this.sessionSeen < Control.FILE_POLL) return;
    this.sessionSeen = t;
    const want = existsSync(abs(HALT));
    if (want && !s.frozenFor.has("halt")) {
      const count = s.freeze("halt");
      say(`   [h] HALTED -- ${count} process(es) frozen where they stood, nothing lost. Press h, or delete ${HALT}, to carry on`, C.YELLOW);
    } else if (!want && s.frozenFor.has("halt")) {
      const spent = s.thaw("halt");
      say(`   [h] carrying on after ${hms(spent)}`, C.GREEN);
    }
    if (s.streaming && existsSync(abs(SAY))) {
      let text: string;
      try {
        text = readFileSync(abs(SAY), "utf8").trim();
      } catch {
        return;
      }
      rmSync(abs(SAY), { force: true });
      if (text) this.deliver(text, SAY);
    }
  }

  private beginTyping(): void {
    const s = this.session;
    if (!s) {
      say("   [i] does nothing right now; it sends a prompt to a running session", C.GRAY);
      return;
    }
    if (!s.streaming) {
      say("   [i] this session cannot be sent a prompt: its reply is already complete", C.GRAY);
      return;
    }
    this.typing = "";
    s.freeze("typing");
    say("   [i] the session is frozen while you type -- Enter sends, Esc cancels", C.YELLOW);
  }

  /** One key of a prompt: every key is text here, `s` and `p` included. */
  private type(ch: string): void {
    if (ch === "\r" || ch === "\n") {
      const text = (this.typing ?? "").trim();
      this.typing = null;
      if (text && this.session) this.deliver(text, "the console");
      else say("   [i] nothing sent", C.GRAY);
      this.session?.thaw("typing");
    } else if (ch === "\x1b") {
      this.typing = null;
      say("   [i] cancelled -- nothing sent", C.GRAY);
      this.session?.thaw("typing");
    } else if (ch === "\x08" || ch === "\x7f") {
      this.typing = (this.typing ?? "").slice(0, -1);
    } else if (ch >= " ") {
      this.typing = (this.typing ?? "") + ch;
    }
  }

  private deliver(text: string, via: string): void {
    const s = this.session;
    if (!s || !s.send(text)) {
      say(`   [i] NOT sent -- the session is no longer taking input: ${text}`, C.RED);
      return;
    }
    CONSOLE.raw(`${JSON.stringify({ type: "loop_prompt", via, text })}\n`);
    say(`   [i] sent to the session, from ${via}: ${text}`, C.GREEN);
  }

  private togglePause(): void {
    if (this.pauseBy) {
      const mine = this.pauseBy === "user";
      this.pauseBy = "";
      this.pauseWhy = "";
      this.held = false;
      rmSync(abs(PAUSE), { force: true });
      say(mine ? "   [p] carrying on -- the hold is lifted" : `   [p] carrying on -- ${PAUSE} was another agent's hold, and the console outranks it`, C.GREEN);
      return;
    }
    this.pauseBy = "user";
    this.writePause();
    say(`   [p] hold requested -- the run stops between sessions and waits. Press p again to carry on; deleting ${PAUSE} will not, because this hold was taken at the console`, C.YELLOW);
  }

  /** Writes `.loop/pause`: who owns the hold, and whether it has taken effect. The `held:` line is what an agent waits for. */
  private writePause(): void {
    const mine = this.pauseBy === "user";
    const note = mine
      ? "This hold was taken at the console with `p`, and is lifted there with `p`.\nDeleting this file does not lift it -- the driver writes it straight back.\n"
      : "Delete this file to let the run carry on.\nDo not edit this tree until the `held:` line above is there: until then the\nhold is only queued, and the session in flight is still committing to it.\n";
    const d = new Date();
    const held = this.held ? `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())} ${clock(d)}` : "(not yet)";
    const why = this.pauseWhy ? `why:     ${this.pauseWhy}\n` : "";
    writeQuiet(PAUSE, `by:      ${mine ? "user (the console)" : "agent (this file)"}\nheld:    ${held}\npid:     ${process.pid}\n${why}\n${note}`);
  }

  /** Reconciles the armed hold with the file: a file that appears arms an agent's hold, and one that goes lifts it, unless the console owns the hold. */
  private syncPause(): void {
    const t = now();
    if (t - this.pauseSeen < Control.FILE_POLL) return;
    this.pauseSeen = t;
    const there = existsSync(abs(PAUSE));
    if (this.pauseBy === "user") {
      if (!there) this.writePause();
      return;
    }
    if (there && !this.pauseBy) {
      // Whose hold it is comes out of the file: every turn is a new process, so the one adopting a hold is routinely not the one the key was pressed at.
      this.pauseBy = this.fileOwner();
      say(
        this.pauseBy === "agent" ? `   ${PAUSE} appeared -- the run holds after the current session, and carries on when the file goes` : "   the hold taken at the console still stands -- press p to carry on",
        C.YELLOW,
      );
    } else if (!there && this.pauseBy === "agent") {
      this.pauseBy = "";
      this.pauseWhy = "";
      this.held = false;
      say(`   ${PAUSE} is gone -- carrying on`, C.GREEN);
    }
  }

  /** `user` for a hold the console took, `agent` for anything else, an unreadable file included. */
  private fileOwner(): string {
    try {
      const by = readFileSync(abs(PAUSE), "utf8")
        .split("\n")
        .find((l) => l.startsWith("by:"));
      if (by !== undefined) return by.includes("user") ? "user" : "agent";
    } catch {
      // As below.
    }
    return "agent";
  }

  /** Why the run should hold here, or "". */
  pauseReason(): string {
    this.pauseSeen = 0;
    this.syncPause();
    if (this.pauseBy === "user") return "p was pressed at the console";
    if (this.pauseBy === "agent") return `${PAUSE} present`;
    return "";
  }

  enterHold(): void {
    this.held = true;
    this.writePause();
  }

  /**
   * Holds the run because the driver needs a person, naming the verdict. It is an agent's hold, lifted by
   * deleting `.loop/pause` as well as by `p`, since the person it waits for may be reading the tree from
   * another terminal. A hold already armed is left to whoever armed it.
   */
  armHold(why: string): void {
    if (this.pauseBy) return;
    this.pauseBy = "agent";
    this.pauseWhy = why;
    this.writePause();
  }

  leaveHold(): void {
    this.held = false;
    if (this.pauseBy === "user") this.writePause();
  }

  /** The run ends: a hold taken at the console or armed by the driver goes with the run. Another agent's file is left alone. */
  dropPause(): void {
    if (this.pauseBy === "user" || this.pauseWhy) rmSync(abs(PAUSE), { force: true });
    this.pauseBy = "";
    this.pauseWhy = "";
    this.held = false;
  }

  /** Seconds an armed stop can still be taken back in, or 0. */
  stopIn(): number {
    return this.stop ? Math.max(0, this.stopAt - now()) : 0;
  }

  /** Why the run should end now, or "". */
  stopReason(): string {
    if (this.stop && this.stopIn() <= 0) return "s was pressed at the console";
    if (existsSync(abs(STOP))) return `${STOP} present`;
    return "";
  }

  /** A request a wait would end on right now: a stop past its grace, or a retry. A hold is not one. */
  pending(): boolean {
    return this.stopReason() !== "" || this.retry || existsSync(abs(RETRY));
  }

  /** Consumes a retry request, with its file. */
  takeRetry(): boolean {
    const asked = this.retry || existsSync(abs(RETRY));
    this.retry = false;
    rmSync(abs(RETRY), { force: true });
    return asked;
  }

  /** Waits out an `s` still inside its window, so a stop pressed at the end of a turn is acted on or taken back. */
  async settleStop(): Promise<void> {
    while (this.stopIn() > 0) {
      this.poll();
      await Bun.sleep(100);
    }
  }
}

export const CONTROL = new Control();

function writeQuiet(path: string, text: string): void {
  try {
    mkdirSync(dirname(abs(path)), { recursive: true });
    writeFileSync(abs(path), text);
  } catch {
    // A control file that cannot be written leaves the key undone, and the next look tries again.
  }
}

/**
 * Holds here while a hold stands. Returns "" when it lifts and the run goes on, or the reason it stops
 * instead: a stop always wins over a hold.
 */
export async function holdPause(ledger: (line: string) => void): Promise<string> {
  let stop = CONTROL.stopReason();
  const why = CONTROL.pauseReason();
  if (stop || !why) return stop;
  const began = now();
  CONTROL.enterHold();
  step(`holding before the next session -- ${why}; ${CONTROL.pauseBy === "user" ? "press p to carry on" : `delete ${PAUSE}, or press p, to carry on`}`, C.YELLOW);
  TICKER.set({ phase: "held" });
  try {
    for (;;) {
      CONTROL.poll();
      stop = CONTROL.stopReason();
      if (stop || !CONTROL.pauseReason()) break;
      TICKER.set({ detail: hms(now() - began) });
      await Bun.sleep(250);
    }
  } finally {
    CONTROL.leaveHold();
  }
  const spent = hms(now() - began);
  step(`held ${spent} -- ${stop ? "stopping" : "carrying on"}`, stop ? C.YELLOW : C.GREEN);
  ledger(`       held ${spent} -- ${why}`);
  return stop;
}

// ------------------------------------------------------------------------------------ nv verify

/** What `nv verify` is doing inside a session's tool call: one grey line per step, and the step in flight on the status line. */
class VerifyWatch {
  static readonly EVERY = 0.5;
  /** Seconds after which a step still in flight is a killed run's leftover. */
  static readonly STALE = 600;
  /** Wall-clock seconds of the arming; 0 while no session runs. */
  private armed = 0;
  private seen = "";
  private next = 0;

  arm(): void {
    this.armed = Date.now() / 1000;
    this.seen = "";
    this.next = 0;
  }

  disarm(): void {
    this.armed = 0;
  }

  poll(): void {
    if (!this.armed) return;
    const t = now();
    if (t < this.next) return;
    this.next = t + VerifyWatch.EVERY;
    const entry = this.read();
    if (entry === null || "finished" in entry || !("step" in entry)) return;
    const key = `${entry.index}\0${entry.step}`;
    if (key === this.seen) return;
    this.seen = key;
    const done = Array.isArray(entry.done) ? entry.done : [];
    const last = done.at(-1);
    const before = last ? `${last.name} ok ${mmss(Number(last.seconds))} -> ` : "";
    say(`     ~ verify: ${before}${entry.step} (${entry.index}/${entry.total})`, C.GRAY);
  }

  private read(): Record<string, any> | null {
    try {
      if (statSync(abs(VERIFY_PROGRESS)).mtimeMs / 1000 < this.armed) return null;
      const entry = JSON.parse(readFileSync(abs(VERIFY_PROGRESS), "utf8"));
      if (typeof entry !== "object" || entry === null) return null;
      if (Date.now() / 1000 - Number(entry.at ?? 0) > VerifyWatch.STALE) return null;
      return entry;
    } catch {
      return null;
    }
  }
}

export const VERIFY = new VerifyWatch();

// ----------------------------------------------------------------------------- what a session landed

function git(...args: string[]): string {
  try {
    const r = Bun.spawnSync(["git", ...args], { cwd: ROOT, stdout: "pipe", stderr: "ignore" });
    return r.exitCode === 0 ? r.stdout.toString().trim() : "";
  } catch {
    return "";
  }
}

/** Where the running session began, which its commits are counted from. */
class SliceWatch {
  /** HEAD when the session started. */
  base = "";

  /** A session is about to start. Its base is also written to `.loop/session-start.json` for `nv session`. */
  start(base: string): void {
    this.base = base;
    try {
      if (base) writeFileSync(abs(SESSION_BASE), `${JSON.stringify({ base })}\n`);
      else rmSync(abs(SESSION_BASE), { force: true });
    } catch {
      // A convenience; a run never fails over it.
    }
  }

  /** The subjects of the commits since the base, oldest first, which the goal table's last line names. */
  subjects(): string[] {
    if (!this.base) return [];
    return git("log", "--reverse", "--format=%s", `${this.base}..HEAD`)
      .split("\n")
      .filter((l) => l !== "");
  }
}

export const SLICES = new SliceWatch();
