import { describe, expect, test } from "bun:test";
import { EDITORS, type Editor } from "../src/api.ts";
import * as linkModule from "../src/editor-link.ts";
import { editorLink, mapPath, type EditorSettings } from "../src/editor-link.ts";

// Links from a record's site to the editor: each preset, a template, path mappings and refused schemes.
// `exceptions.browser.test.ts` checks the links on the page, and that a log message gives none.

const SITE = { file: "src/Shop/Orders.nvs", line: 42 };

function pick(editor: Editor, extra: Partial<EditorSettings> = {}): EditorSettings {
  return { editor, path_mappings: [], ...extra };
}

describe("an editor link", () => {
  test("every preset builds its own link, and the column is 1 when the site has none", () => {
    const links = Object.fromEntries(EDITORS.filter((e) => e !== "custom").map((e) => [e, editorLink(SITE, pick(e))]));
    expect(links).toEqual({
      vscode: "vscode://file/src/Shop/Orders.nvs:42:1",
      "vscode-insiders": "vscode-insiders://file/src/Shop/Orders.nvs:42:1",
      vscodium: "vscodium://file/src/Shop/Orders.nvs:42:1",
      cursor: "cursor://file/src/Shop/Orders.nvs:42:1",
      zed: "zed://file/src/Shop/Orders.nvs:42:1",
      idea: "idea://open?file=src/Shop/Orders.nvs&line=42",
      phpstorm: "phpstorm://open?file=src/Shop/Orders.nvs&line=42",
    });
    expect(editorLink({ ...SITE, column: 7 }, pick("vscode"))).toBe("vscode://file/src/Shop/Orders.nvs:42:7");
  });

  test("VS Code is the editor before the settings load", () => {
    expect(editorLink(SITE, null)).toBe("vscode://file/src/Shop/Orders.nvs:42:1");
  });

  test("an absolute path loses its leading slash in a file link, and keeps it in a query", () => {
    expect(editorLink({ file: "/home/a", line: 3 }, pick("vscode"))).toBe("vscode://file/home/a:3:1");
    expect(editorLink({ file: "/home/a", line: 3 }, pick("phpstorm"))).toBe("phpstorm://open?file=/home/a&line=3");
  });

  test("a Windows path has forward slashes and keeps its drive", () => {
    expect(editorLink({ file: "C:\\x\\Shop.nvs", line: 3 }, pick("vscode"))).toBe("vscode://file/C:/x/Shop.nvs:3:1");
  });

  test("a path is percent-encoded, with its slashes and colons kept", () => {
    expect(editorLink({ file: "/srv/my shop/a#b?.nvs", line: 1 }, pick("vscode"))).toBe("vscode://file/srv/my%20shop/a%23b%3F.nvs:1:1");
  });

  test("a template replaces each placeholder", () => {
    const settings = pick("custom", { editor_template: "subl://open?url=file://{file}&line={line}&col={column}" });
    expect(editorLink({ file: "/app/a.nvs", line: 9, column: 2 }, settings)).toBe("subl://open?url=file:///app/a.nvs&line=9&col=2");
  });

  test("the longest mapping that starts the path replaces that start", () => {
    const path_mappings = [
      { from: "/app/", to: "/home/dev/shop/" },
      { from: "/app/vendor/", to: "/home/dev/lib/" },
      { from: "/other/", to: "/nowhere/" },
    ];
    expect(mapPath("/app/vendor/x.nvs", path_mappings)).toBe("/home/dev/lib/x.nvs");
    expect(mapPath("/app/src/x.nvs", path_mappings)).toBe("/home/dev/shop/src/x.nvs");
    expect(mapPath("/srv/x.nvs", path_mappings)).toBe("/srv/x.nvs");
    expect(editorLink({ file: "/app/src/x.nvs", line: 4 }, pick("vscode", { path_mappings }))).toBe("vscode://file/home/dev/shop/src/x.nvs:4:1");
    // A mapping to a Windows path gives forward slashes.
    expect(editorLink({ file: "/app/x.nvs", line: 4 }, pick("vscode", { path_mappings: [{ from: "/app/", to: "D:\\work\\" }] }))).toBe(
      "vscode://file/D:/work/x.nvs:4:1",
    );
  });

  test("a template with a scheme that runs script, or with no scheme, gives no link", () => {
    for (const template of [
      "javascript:alert('{file}')",
      "JavaScript:alert(1)//{file}",
      "data:text/html,{file}",
      "vbscript:{file}",
      "blob:{file}",
      "{file}:{line}",
      " javascript:alert(1)",
      "java\tscript:alert(1)",
      "",
    ]) {
      expect(editorLink(SITE, pick("custom", { editor_template: template }))).toBeNull();
    }
    expect(editorLink(SITE, pick("custom"))).toBeNull();
  });

  test("a path cannot change the scheme of a template that starts with the file", () => {
    expect(editorLink({ file: "javascript:alert(1)", line: 1 }, pick("custom", { editor_template: "{file}" }))).toBeNull();
  });

  test("the module builds links from a site alone, and reads no file and line out of text", () => {
    expect(Object.keys(linkModule).sort()).toEqual(["PRESETS", "editorLink", "mapPath"]);
  });
});
