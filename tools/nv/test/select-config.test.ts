import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { blockPaths, configKeys } from "../select/config.ts";
import { logKeys, pathKeys } from "../select/keys.ts";
import { SelectStore } from "../select/store.ts";

const at = (p: string) => join(ROOT, p).replace(/\\/g, "/");

/** The keys a program run from the root records for `entry`: the file by part, and the entry file with
 * every directory above it, as `nvs_config::app::matching` writes them. */
function programKeys(entry: string): Set<string> {
  const lines = [`config\t${at("nvs.toml")}`];
  let p = at(entry);
  for (;;) {
    lines.push(`app\t${p}`);
    const up = p.slice(0, p.lastIndexOf("/"));
    if (up === "" || up === p || !up.includes("/")) {
      lines.push(`app\t${up}/`);
      break;
    }
    p = up;
  }
  return logKeys(lines.join("\n"));
}

const BASE = [
  "# the file",
  "[[app]]",
  'root = "."',
  'origin = "https://example.test"',
  "",
  "[[app]]",
  'entry = "docs/examples/core/Shop/list.nvs"',
  "[app.capabilities.fs]",
  'read = ["docs/examples/core/Shop"]',
  "",
  "[[app]]",
  'root = "docs/examples/core/Blog"',
  "[app.capabilities.cache]",
  "shared = true",
  "",
  "[limits]",
  'memory = "256M"',
  "",
].join("\n");

const edit = (from: string, to: string) => {
  expect(BASE).toContain(from);
  return BASE.replace(from, to);
};

describe("the repository's nvs.toml, keyed by part", () => {
  test("a footprint line of a configuration read is a config key, and an app line a lowercased path key", () => {
    const keys = logKeys([`config\t${at("nvs.toml")}`, `app\t${at("docs/Examples/X.nvs")}`, `app\t${at("")}`, "app\tC:/elsewhere", "app\t*", `config\t${at("target/x.toml")}`].join("\n"));
    expect([...keys].sort()).toEqual(["app:*", "app:.", "app:docs/examples/x.nvs", "app:~", "config:nvs.toml"]);
  });

  test("the root configuration moves no config key by path, and any other file moves its own", () => {
    expect(pathKeys("nvs.toml", false)).toContain("file:nvs.toml");
    expect(pathKeys("nvs.toml", false)).not.toContain("config:nvs.toml");
    expect(pathKeys("docs/examples/core/Shop/list/nvs.toml", false)).toContain("config:docs/examples/core/Shop/list/nvs.toml");
  });

  test("an edit of a global table moves the config key and no block", () => {
    expect(configKeys("nvs.toml", BASE, edit('memory = "256M"', 'memory = "512M"'))).toEqual(["config:nvs.toml"]);
  });

  test("an edit of one block moves that block's path, and every reader of the whole roster", () => {
    expect(configKeys("nvs.toml", BASE, edit("shared = true", "shared = false"))).toEqual(["app:*", "app:docs/examples/core/blog"]);
  });

  test("a block added moves its path; one moved within the file, or a comment, moves nothing", () => {
    const added = `${BASE}\n[[app]]\nentry = "docs/examples/core/Shop/add.nvs"\n`;
    expect(configKeys("nvs.toml", BASE, added)).toEqual(["app:*", "app:docs/examples/core/shop/add.nvs"]);
    const moved = [BASE.split("\n\n")[0], BASE.split("\n\n")[2], BASE.split("\n\n")[1], BASE.split("\n\n")[3], ""].join("\n\n");
    expect(configKeys("nvs.toml", BASE, moved)).toEqual([]);
    expect(configKeys("nvs.toml", BASE, edit("# the file", "# the file, again"))).toEqual([]);
  });

  test("a block's path is resolved against the file's directory, and one outside the tree is app:~", () => {
    expect(configKeys("nvs.toml", BASE, edit('root = "docs/examples/core/Blog"', 'root = "./docs/examples/core/Blog/../Blog"'))).toEqual([]);
    expect(configKeys("nvs.toml", BASE, edit('root = "docs/examples/core/Blog"', 'root = "../elsewhere"'))).toEqual(["app:*", "app:docs/examples/core/blog", "app:~"]);
  });

  test("what cannot be read by part moves the whole file", () => {
    const whole = ["config:nvs.toml"];
    expect(configKeys("nvs.toml", null, BASE)).toEqual(whole);
    expect(configKeys("nvs.toml", BASE, null)).toEqual(whole);
    expect(configKeys("nvs.toml", BASE, `${BASE}\n[limits\n`)).toEqual(whole);
    expect(configKeys("nvs.toml", BASE, `${BASE}\n[[app]]\nroot = "a"\nentry = "a/b.nvs"\n`)).toEqual(whole);
    expect(configKeys("nvs.toml", BASE, `${BASE}\n[[app]]\nmode = "production"\n`)).toEqual(whole);
    expect(configKeys("nvs.toml", BASE, `${BASE}\n[[app]]\nroot = "docs/examples/core/blog"\n`)).toEqual(whole);
    expect(configKeys("nvs.toml", BASE, BASE.replace("[[app]]\nroot = \".\"", 'app = 1\n[[x]]\nroot = "."'))).toEqual(whole);
  });

  test("a block reaches the programs it can apply to and no other", () => {
    const blog = programKeys("docs/examples/core/Blog/post.nvs");
    const shop = programKeys("docs/examples/core/Shop/list.nvs");
    const moved = configKeys("nvs.toml", BASE, edit("shared = true", "shared = false"));
    expect(moved.some((k) => blog.has(k))).toBe(true);
    expect(moved.some((k) => shop.has(k))).toBe(false);
    const added = configKeys("nvs.toml", BASE, `${BASE}\n[[app]]\nroot = "docs/examples"\n`);
    expect(added.some((k) => blog.has(k)) && added.some((k) => shop.has(k))).toBe(true);
    const global = configKeys("nvs.toml", BASE, edit('memory = "256M"', 'memory = "1G"'));
    expect(global.some((k) => blog.has(k)) && global.some((k) => shop.has(k))).toBe(true);
    const outside = configKeys("nvs.toml", BASE, `${BASE}\n[[app]]\nroot = "/srv"\n`);
    expect(outside.some((k) => blog.has(k))).toBe(true);
  });

  test("each block's path is resolved against the file's directory, and kept outside the tree", () => {
    const paths = blockPaths("docs/nvs.toml", '[[app]]\nroot = "Blog"\n[[app]]\nentry = "../examples/x.nvs"\n[[app]]\nroot = "../../elsewhere"\n', ROOT)!;
    expect(paths.get("app:docs/blog")).toBe("docs/Blog");
    expect(paths.get("app:examples/x.nvs")).toBe("examples/x.nvs");
    expect(paths.get("app:~")?.endsWith("/elsewhere")).toBe(true);
    expect(blockPaths("nvs.toml", "[[app]]\nmode = 1\n")).toBeNull();
  });

  test("the repository's own file reads by part, with one key per block", () => {
    const text = readFileSync(join(ROOT, "nvs.toml"), "utf8");
    const blocks = text.split(/\r?\n/).filter((l) => l.trim() === "[[app]]").length;
    const parsed = Bun.TOML.parse(text) as { app?: unknown[] };
    expect(parsed.app?.length).toBe(blocks);
    expect(configKeys("nvs.toml", text, text)).toEqual([]);
    expect(configKeys("nvs.toml", text, `${text}\n[[app]]\nentry = "examples/new.nvs"\n`)).toEqual(["app:*", "app:examples/new.nvs"]);
  });
});

describe("a footprint that held a configuration file whole", () => {
  const ids = (s: SelectStore, id: string) => s.footprint(id);

  test("is replaced by the first run that reads the file by part", () => {
    const s = new SelectStore(":memory:", "test-os");
    s.recordRun("proof:docs/examples/a.nvs", { def: "d", verdict: "green", keys: new Map([["file:nvs.toml", ""], ["exists:docs/examples/b.nvs", ""], ["fn:a.rs#f", "x"]]) });
    s.recordRun("proof:docs/examples/a.nvs", { def: "d", verdict: "green", keys: new Map([["config:nvs.toml", ""], ["app:docs/examples/a.nvs", ""], ["fn:a.rs#f", "x"]]) });
    expect(ids(s, "proof:docs/examples/a.nvs")).toEqual(["app:docs/examples/a.nvs", "config:nvs.toml", "fn:a.rs#f"]);
    s.close();
  });

  test("still widens when the run also read the file whole, or when it never held it", () => {
    const s = new SelectStore(":memory:", "test-os");
    s.recordRun("nvtest:t.ts", { def: "d", verdict: "green", keys: new Map([["file:nvs.toml", ""], ["exists:x", ""]]) });
    s.recordRun("nvtest:t.ts", { def: "d", verdict: "green", keys: new Map([["file:nvs.toml", ""], ["config:nvs.toml", ""]]) });
    expect(ids(s, "nvtest:t.ts")).toEqual(["config:nvs.toml", "exists:x", "file:nvs.toml"]);
    s.recordRun("proof:b.nvs", { def: "d", verdict: "green", keys: new Map([["exists:x", ""]]) });
    s.recordRun("proof:b.nvs", { def: "d", verdict: "green", keys: new Map([["config:nvs.toml", ""]]) });
    expect(ids(s, "proof:b.nvs")).toEqual(["config:nvs.toml", "exists:x"]);
    s.close();
  });
});
