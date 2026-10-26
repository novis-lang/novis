import { afterEach, describe, expect, test } from "bun:test";
import { shotShape, shotVerdict, snippetShape } from "../cmd/site.ts";
import { parseShot, profileText, shotOut, shotText } from "../lib/shoot.ts";
import { scratch, type Scratch } from "./scratch.ts";

// A website screenshot's steps, its text and the `shots` part's verdicts
// (`rule:testing/a-screenshot-is-taken-from-a-real-run`). Taking a real shot needs a browser and a
// built `nvs`, and `bun nv site --check shots` is that run; these cases hold what it decides.

let s: Scratch;
afterEach(() => s?.cleanup());

const VIEWER = "app notes\nrequest GET /api/notes\nrequest POST /notes title=First note\nopen /\nwait .requests .route\nclick .requests .route\ntype input[type=search] home page\n";

describe("a .shot", () => {
  test("a viewer shot has its app, its requests and its steps in order", () => {
    expect(parseShot(VIEWER)).toEqual({
      kind: "viewer",
      app: "notes",
      requests: [
        { method: "GET", path: "/api/notes" },
        { method: "POST", path: "/notes", body: "title=First note" },
      ],
      steps: [
        { kind: "open", path: "/" },
        { kind: "wait", selector: ".requests .route" },
        { kind: "click", selector: ".requests .route" },
        { kind: "type", selector: "input[type=search]", text: "home page" },
      ],
    });
  });

  test("a profile shot names one program and nothing else", () => {
    expect(parseShot("profile main.nvs\n")).toEqual({ kind: "profile", program: "main.nvs" });
    expect(parseShot("profile main.nvs\nopen /\n")).toBe("a `profile` shot has no requests and no steps");
    expect(parseShot("profile ../main.nvs\n")).toBe("line 1: `profile ../main.nvs` is not a step");
  });

  test("a shot that names neither, both, or starts with no `open` is not one", () => {
    expect(parseShot("open /\n")).toBe("it names no `app` and no `profile`");
    expect(parseShot("app notes\nprofile main.nvs\n")).toBe("it names both an `app` and a `profile`");
    expect(parseShot("app notes\nwait .x\n")).toBe("its first step is not `open`");
    expect(parseShot("app notes\nscroll down\n")).toBe("line 2: `scroll down` is not a step");
  });
});

describe("a .shot.out", () => {
  test("a viewer's text has every value that differs from run to run masked", () => {
    const shown = "GET\n/api/notes\n200\n854 µs\n3.45 ms\n12 seconds ago\n2026-10-27 09:12:44\n4bf92f3577b34da6a3ce929d0e0e4736\nhttp://127.0.0.1:51234/\n\n\n\nEnd";
    expect(shotText(shown)).toBe("GET\n/api/notes\n200\n<duration>\n<duration>\n<ago>\n<date>\n<id>\nhttp://127.0.0.1:<port>/\n\nEnd");
  });

  test("a profile's text is each timeline's frames and calls, and no time", () => {
    const exported = JSON.stringify({
      shared: { frames: [{ name: "Shop::total" }, { name: "Shop::tax" }] },
      profiles: [
        { name: "main.nvs", events: [{ type: "O", frame: 1, at: 1 }, { type: "C", frame: 1, at: 5 }, { type: "O", frame: 0, at: 6 }, { type: "O", frame: 1, at: 7 }, { type: "C", frame: 1, at: 8 }, { type: "C", frame: 0, at: 9 }] },
        { name: "main.nvs 1", events: [{ type: "O", frame: 0, at: 2 }, { type: "C", frame: 0, at: 3 }] },
      ],
    });
    expect(profileText(exported)).toBe("timeline main.nvs\n  Shop::tax\t2 calls\n  Shop::total\t1 call\n\ntimeline main.nvs 1\n  Shop::total\t1 call");
  });

  test("the file starts with the hash of the assets the page draws with", () => {
    expect(shotOut("0123456789abcdef", "text\n\n")).toBe("assets 0123456789abcdef\n\ntext\n");
  });
});

describe("the shots part", () => {
  const P = "website/snippets/tools/debug-viewer/requests.shot";
  const OUT = shotOut("0123456789abcdef", "GET\n200");

  test("a missing or different .shot.out, or a missing picture, names the command that takes it", () => {
    expect(shotVerdict(P, OUT, OUT, true)).toEqual([]);
    expect(shotVerdict(P, OUT, OUT.replace("\n", "\r\n"), true)).toEqual([]);
    expect(shotVerdict(P, OUT, null, true)).toEqual([`${P}: no .shot.out beside it; run \`bun nv site --shoot ${P}\``]);
    expect(shotVerdict(P, OUT, OUT.replace("200", "404"), true)).toEqual([`${P}: its .shot.out is not what the run shows now; run \`bun nv site --shoot ${P}\``]);
    expect(shotVerdict(P, shotOut("fedcba9876543210", "GET\n200"), OUT, true)).toHaveLength(1);
    expect(shotVerdict(P, OUT, OUT, false)).toEqual([`${P}: no requests.png beside it; run \`bun nv site --shoot ${P}\``]);
  });

  test("a .shot.out or a picture with no .shot is named", () => {
    s = scratch();
    s.put(P, VIEWER);
    s.put(`${P}.out`, OUT);
    s.put("website/snippets/tools/debug-viewer/requests.png", "png");
    s.put("website/snippets/tools/old.shot.out", OUT);
    s.put("website/snippets/tools/old.png", "png");
    expect(shotShape(s.root)).toEqual({
      shots: [P],
      problems: ["website/snippets/tools/old.shot.out: a .shot.out and no .shot beside it", "website/snippets/tools/old.png: a picture and no .shot beside it"],
    });
  });

  test("a program in a shot's fixture is not a snippet", () => {
    s = scratch();
    s.put("website/snippets/tools/profiler/fibonacci.shot", "profile main.nvs\n");
    s.put("website/snippets/tools/profiler/fibonacci/main.nvs", "<?nvs\necho 1;\n");
    expect(snippetShape(s.root).snippets).toEqual([]);
  });
});
