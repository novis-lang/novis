// How the kit starts a program from an argv, as `crates/looper-engine/src/program.rs` does: the one
// resolver that a job, a check and a hook share.
//
// - `find` resolves a bare name as a Windows shell does. It tries each directory of `PATH` in order,
//   and in each one the name with each extension of `PATHEXT`, in order. A name whose extension is
//   already one of `PATHEXT`'s is looked for as named.
// - `program` builds what a spawn runs for an argv. A `argv[0]` with a directory is started as named.
//   A bare name is resolved by `find` on the `PATH` the process will see. A `.cmd` or `.bat` match runs
//   through `cmd /d /c`, as a shell would pass it. An argument of letters, digits and `-_./\:+@#~`
//   goes bare, and any other is put in double quotes, which the script's `%1` keeps.
// - An argument cmd cannot carry unchanged, one holding `"`, `%` or a line break, is refused with the
//   reason. The kit never changes an argument silently.
//
// It keeps no state, and costs one scan of `PATH` per command.

import { statSync } from "node:fs";
import { delimiter, extname, join } from "node:path";
import { quote } from "../os/index.ts";

/** The extensions a name resolves with when `PATHEXT` is unset, as Windows' own default. */
export const DEFAULT_PATHEXT = ".COM;.EXE;.BAT;.CMD";

/** What a spawn runs: the argv, and the whole command line that carries it. */
export interface Program {
  argv: string[];
  line: string;
}

function isFile(path: string): boolean {
  try {
    return statSync(path).isFile();
  } catch {
    return false;
  }
}

/** The file a bare `name` resolves to on `path` with the extensions of `pathext`, or null. */
export function find(name: string, path: string, pathext: string): string | null {
  if (name === "" || /[/\\:]/.test(name)) return null;
  const exts = pathext.split(";").filter((e) => e !== "");
  const ext = extname(name).replace(/^\./, "").toLowerCase();
  const named = ext !== "" && exts.some((e) => e.replace(/^\./, "").toLowerCase() === ext);
  const candidates = named ? [name] : exts.map((e) => `${name}${e}`);
  for (const dir of path.split(delimiter)) {
    if (dir === "") continue;
    for (const c of candidates) {
      const file = join(dir, c);
      if (isFile(file)) return file;
    }
  }
  return null;
}

/** `env[key]`, matched without case as Windows reads a variable's name. */
function lookup(env: Record<string, string | undefined>, key: string): string | undefined {
  const k = Object.keys(env).find((name) => name.toLowerCase() === key.toLowerCase());
  return k === undefined ? undefined : env[k];
}

/** `arg`, or why cmd cannot carry it unchanged inside double quotes. */
function unchanged(arg: string): string {
  if (/["%\r\n]/.test(arg)) {
    throw new Error(
      `the argument ${JSON.stringify(arg)} cannot reach a .cmd or .bat script unchanged, because cmd reads \`"\`, \`%\` and line breaks itself`,
    );
  }
  return arg;
}

/** What runs `argv` in the environment `env`, or an error saying why it cannot run. */
export function program(argv: readonly string[], env: Record<string, string | undefined> = process.env): Program {
  const [name, ...rest] = argv;
  if (name === undefined) throw new Error("the argv is empty");
  const pathext = lookup(env, "PATHEXT") ?? DEFAULT_PATHEXT;
  const path = lookup(env, "PATH");
  const found = path === undefined ? null : find(name, path, pathext);
  if (found !== null && /\.(cmd|bat)$/i.test(found)) {
    let inner = `"${unchanged(found)}"`;
    for (const a of rest) {
      unchanged(a);
      inner += a !== "" && /^[A-Za-z0-9\-_./\\:+@#~]+$/.test(a) ? ` ${a}` : ` "${a}"`;
    }
    const comspec = lookup(env, "ComSpec") ?? "cmd.exe";
    // cmd strips the outer pair of quotes from a line that holds more than two.
    return { argv: [comspec, "/d", "/c", inner], line: `${quote(comspec)} /d /c "${inner}"` };
  }
  const full = [found ?? name, ...rest];
  return { argv: full, line: full.map(quote).join(" ") };
}
