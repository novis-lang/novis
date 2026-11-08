// The repository root's `nvs.toml` configures and names nothing under `docs/examples/` or
// `website/snippets/` (`rule:testing/an-example-carries-its-own-configuration`). An example that needs a
// grant or a setting carries it in its own directory's `nvs.toml`, which the website shows beside it.
// The check reads the file twice: parsed, for an `[[app]]` whose `root` or `entry` lies under either
// tree, and as text, for any line, comments included, that names a path under either tree.

import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";

/** The trees whose programs the website shows, each with its output. */
const SHOWN = ["docs/examples", "website/snippets"];

/** A path under either tree on one line of the raw text, where a TOML string may escape `\` as `\\`. */
const NAMED = /(?<![\w.-])(docs[\\/]+examples|website[\\/]+snippets)(?![\w.-])/;

/** A path as the file writes it, relative to the file, with `/` and no leading `./`. */
function normal(path: string): string {
  return path.replace(/[\\/]+/g, "/").replace(/^(\.\/)+/, "").replace(/\/+$/, "");
}

function underShown(path: string): string | undefined {
  const p = normal(path);
  return SHOWN.find((tree) => p === tree || p.startsWith(`${tree}/`));
}

/** Every problem `text`, a root `nvs.toml`, has, each naming where it is and where the setting goes. */
function problems(text: string): string[] {
  const out: string[] = [];
  const parsed = Bun.TOML.parse(text) as { app?: Record<string, unknown>[] };
  for (const [i, app] of (parsed.app ?? []).entries()) {
    for (const key of ["root", "entry"] as const) {
      const value = app[key];
      if (typeof value !== "string") continue;
      const tree = underShown(value);
      if (tree) {
        out.push(
          `[[app]] #${i + 1}: ${key} = "${value}" is under ${tree}/. ` +
            "Move this configuration into an nvs.toml in the example's own directory.",
        );
      }
    }
  }
  for (const [i, line] of text.split(/\r?\n/).entries()) {
    const m = NAMED.exec(line);
    if (m) {
      out.push(
        `line ${i + 1} names ${normal(m[1]!)}/: ${line.trim()} ` +
          "An example's configuration goes into an nvs.toml in the example's own directory.",
      );
    }
  }
  return out;
}

test("the root nvs.toml configures and names nothing under docs/examples/ or website/snippets/", () => {
  expect(problems(readFileSync(join(ROOT, "nvs.toml"), "utf8"))).toEqual([]);
});

test("a file that names neither tree passes", () => {
  const text = [
    "# Every attack under tests/hostile reads this block.",
    "[[app]]",
    'root = "."',
    "",
    "[[app]]",
    'entry = "tests/hostile/config/limits.nvs"',
    "",
    "# A path that only starts with the same letters is another tree.",
    "[[app]]",
    'root = "docs/examples-old"',
  ].join("\n");
  expect(problems(text)).toEqual([]);
});

test("an [[app]] whose entry is an example fails, on the block and on its line", () => {
  const text = ["[[app]]", 'entry = "docs/examples/core/Db-Row/bool/01-flag.nvs"'].join("\n");
  const found = problems(text);
  expect(found).toHaveLength(2);
  expect(found[0]).toStartWith('[[app]] #1: entry = "docs/examples/core/Db-Row/bool/01-flag.nvs"');
  expect(found[1]).toStartWith("line 2 names docs/examples/");
});

test("an [[app]] whose root is a snippet directory fails, written with ./ and backslashes", () => {
  const text = ["[[app]]", 'root = ".\\\\website\\\\snippets\\\\limits"'].join("\n");
  const found = problems(text);
  expect(found[0]).toContain("is under website/snippets/");
  expect(found[1]).toStartWith("line 2 names website/snippets/");
});

test("an [[app]] whose root is a whole tree fails", () => {
  expect(problems(["[[app]]", 'root = "docs/examples"'].join("\n"))[0]).toContain(
    "is under docs/examples/",
  );
});

test("a comment that names an example fails, though it configures nothing", () => {
  const text = [
    "# `docs/examples/config/limits/01-show.nvs` prints this value.",
    "[limits]",
    'memory = "256M"',
  ].join("\n");
  expect(problems(text)).toEqual([
    "line 1 names docs/examples/: # `docs/examples/config/limits/01-show.nvs` prints this value. " +
      "An example's configuration goes into an nvs.toml in the example's own directory.",
  ]);
});
