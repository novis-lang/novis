// The `Core` and `Novis` class introductions name no PHP function, no PHP
// superglobal and not PHP itself (`rule:programs/no-compatibility-promise`), because the website and
// its `/llms-full.txt` publish both. The cards are held to the same list and the same test of a word by
// `no_member_text_says_what_it_replaces_in_php` in `crates/nvs-stdlib/src/registry.rs`.

import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";

const PHP = new Set(
  readFileSync(join(ROOT, "tools/data/php-functions.txt"), "utf8")
    .split("\n")
    .filter((line) => line && !line.startsWith("#")),
);
// Words that are PHP functions and mean something else first: a gettext alias, Novis's `exit` and
// `clone`, the JOSE `dir` key algorithm, digest names, Unix commands, the Unix `syslog` service and C's
// `printf`. The same list is `NOT_PHP` in `crates/nvs-stdlib/src/registry.rs`.
const NOT_PHP = new Set(["_", "exit", "clone", "dir", "md5", "sha1", "crc32", "chmod", "chown", "symlink", "syslog", "printf"]);

/** Every name Novis gives a `Core` class, member, enum case or constant, lower-cased. */
function novisNames(): Set<string> {
  const names = new Set<string>();
  const walk = (v: unknown) => {
    if (Array.isArray(v)) v.forEach(walk);
    else if (v && typeof v === "object")
      for (const [k, x] of Object.entries(v)) {
        if (k === "name" && typeof x === "string") names.add(x.split(/\\|::/).pop()!.toLowerCase());
        walk(x);
      }
  };
  walk(JSON.parse(readFileSync(join(ROOT, "website/src/data/core.json"), "utf8")));
  return names;
}

/**
 * The PHP names `text` carries, in order: the word `PHP`, a superglobal written as code, and a PHP
 * function that is written as code (`` `array_map` ``, `` `array_map(…)` ``, `` `socket_*` ``) or has an
 * underscore in it. A word Novis uses for its own member, class, enum case or constant is not one,
 * because `count` and `round` are both, and neither is a word in `NOT_PHP`.
 */
export function phpNames(text: string, php: Set<string>, novis: Set<string>): string[] {
  const isPhp = (name: string) => php.has(name) && !NOT_PHP.has(name) && !novis.has(name.toLowerCase());
  const found: string[] = [];
  if (/\bPHP\b/.test(text)) found.push("PHP");
  text.split("`").forEach((piece, i) => {
    if (i % 2 === 0) {
      for (const w of piece.split(/[^A-Za-z0-9_]/)) if (/^[a-z]/.test(w) && w.includes("_") && isPhp(w)) found.push(w);
      return;
    }
    const global = piece.match(/^\$(GLOBALS|_[A-Z]+)\b/);
    if (global) return void found.push(`$${global[1]}`);
    if (piece.startsWith("$")) return;
    const name = piece.match(/^[A-Za-z0-9_]*/)![0];
    const after = piece.slice(name.length);
    const family = after === "*" && name.endsWith("_") && [...php].some((f) => f.startsWith(name));
    if (family || ((after === "" || after.startsWith("(") || after === "*") && isPhp(name))) found.push(name);
  });
  return found;
}

describe("php names", () => {
  test("a PHP function counts written as code or with an underscore, and a Novis word never does", () => {
    const php = new Set(["array_map", "count", "strlen", "socket_create", "_", "exit"]);
    const novis = new Set(["count"]);
    const names = (text: string) => phpNames(text, php, novis);
    expect(names("as `array_map` does")).toEqual(["array_map"]);
    expect(names("as array_map does")).toEqual(["array_map"]);
    expect(names("the `strlen($s)` idiom")).toEqual(["strlen"]);
    expect(names("replaces `$_ENV` and `$GLOBALS`")).toEqual(["$_ENV", "$GLOBALS"]);
    expect(names("PHP's own")).toEqual(["PHP"]);
    expect(names("replaces `socket_*`")).toEqual(["socket_"]);
    expect(names("`count` entries, strlen, `_` and `exit`, `array_map_keys`")).toEqual([]);
    expect(names("`$name` or `$_` alone")).toEqual([]);
  });

  test("the class introductions and every chapter's keywords name no PHP function", () => {
    const novis = novisNames();
    const inDir = (dir: string) => readdirSync(join(ROOT, dir)).filter((f) => f.endsWith(".md")).map((f) => `${dir}/${f}`);
    const whole = [...inDir("docs/reference/core"), ...inDir("docs/reference/novis")];
    // The concept page is the one reference chapter that may name PHP.
    const chapters = [...inDir("docs/reference/lang"), ...inDir("docs/reference/tools")].filter((f) => !f.endsWith("30-php-differences.md"));
    // `Core\Password::verify` reads bcrypt hashes PHP wrote, which is interop rather than translation.
    const interop = (line: string, found: string[]) => found.join() === "PHP" && /bcrypt hash(es)? that PHP wrote/.test(line);
    const named: string[] = [];
    for (const file of [...whole, ...chapters]) {
      readFileSync(join(ROOT, file), "utf8")
        .split("\n")
        .forEach((line, i) => {
          const keywords = line.startsWith("keywords:");
          if (!keywords && !whole.includes(file)) return;
          // Every keyword is a word a searcher types, so each one is checked as if it were written as code.
          const text = keywords ? line.split(",").map((w) => "`" + w.replace(/^keywords:/, "").trim() + "`").join(" ") : line;
          const found = phpNames(text, PHP, novis);
          if (found.length && !interop(line, found)) named.push(`${file}:${i + 1} [${found.join(", ")}]`);
        });
    }
    expect(named).toEqual([]);
  });
});
