import { describe, expect, test } from "bun:test";
import { manifestProblems, nextGroupItems, parseEdits, parseWrap, stripTrailers, validateHandoff } from "../cmd/session.ts";

describe("nv session manifestProblems", () => {
  test("a problem two copies share is refused once, and each names the floor check", () => {
    const value = { context: { shapes: ["A shape nobody wrote"] }, stages: [] };
    const found = manifestProblems([
      { value, where: "g", prose: null },
      { value, where: "g", prose: null },
    ]);
    expect(found).toHaveLength(1);
    expect(found[0]).toContain("'A shape nobody wrote'");
    expect(found[0]).toEndWith("`nv chain --check` is on the floor and halts a DONE claim on this; fix the manifest before the wrap, or drop the line");
  });

  test("a clean manifest refuses nothing", () => {
    expect(manifestProblems([{ value: { context: { shapes: ["A commit message"] }, stages: [] }, where: "g", prose: null }])).toEqual([]);
  });
});

describe("nv session parseWrap", () => {
  test("a handoff keeps its own `## ` headings, and a known directive ends it", () => {
    const { sections, errors } = parseWrap(
      "## handoff\n## State\nwhere it stands\n\n## Next group\n- [ ] x\n## commit: a.ts b.ts\nfeat(loop): a subject\n\n## status\nCONTINUE done\n",
    );
    expect(errors).toEqual([]);
    expect(sections.map((s) => [s.kind, s.arg, s.line])).toEqual([
      ["handoff", "", 1],
      ["commit", "a.ts b.ts", 7],
      ["status", "", 10],
    ]);
    expect(sections[0]!.body).toBe("## State\nwhere it stands\n\n## Next group\n- [ ] x");
    expect(sections[1]!.body).toBe("feat(loop): a subject");
  });

  test("the same malformed input the Python parser refuses is refused", () => {
    const { errors } = parseWrap("stray text\n## handoff\nbody\n## handoff\nagain\n## commit:\nmsg\n## status: CONTINUE\nx\n## playbook: Tooling\n\n");
    expect(errors).toEqual([
      "line 1: text before the first `## ` directive: 'stray text'",
      "line 4: a second `## handoff` section",
      "line 6: `## commit:` needs an argument",
      "line 8: `## status` takes no argument, got 'CONTINUE'",
      "line 10: `## playbook` has an empty body",
    ]);
  });

  test("a CRLF heading has its carriage return trimmed from the argument", () => {
    const { sections } = parseWrap("## commit: a.ts\r\nmsg\r\n");
    expect(sections[0]!.arg).toBe("a.ts");
  });
});

describe("nv session parseEdits", () => {
  test("pairs are normalized to one single-spaced paragraph", () => {
    const { pairs, errors } = parseEdits("--- old\nconformance   is\n506\n--- new\nconformance is 512\n--- old\ndrop me\n--- new\n");
    expect(errors).toEqual([]);
    expect(pairs).toEqual([
      ["conformance is 506", "conformance is 512"],
      ["drop me", ""],
    ]);
  });

  test("a new with no old, text in front and no pair at all are refused", () => {
    expect(parseEdits("note\n--- new\nx").errors).toEqual([
      "text before the first `--- old`: 'note'",
      "a `--- new` fragment with no `--- old` in front of it",
      "an empty `--- old` fragment -- it must quote what is there now",
    ]);
    expect(parseEdits("").errors).toEqual(["no `--- old` / `--- new` fragment pair"]);
  });
});

describe("nv session handoff", () => {
  const handoff = (item: string) => `## State\nx\n\n## Next group\n**Stage 1** -- files\n\n${item}\n\n## Backlog\n- y\n`;

  test("an open item with a repo-rooted anchor passes, and a ticked one needs none", () => {
    expect(validateHandoff(handoff("- [ ] **Do it** at `tools/nv/cmd/session.ts:12`.\n- [x] **Done**"))).toEqual([]);
    expect(nextGroupItems(handoff("- [ ] a\n      more\n- [ ] b")).map(([n]) => n)).toEqual([1, 2]);
  });

  test("a bare anchor and a missing heading are refused", () => {
    const errors = validateHandoff(handoff("- [ ] **Do it** at `ctx.rs:12`.").replace("## Backlog", "## Later"));
    expect(errors[0]).toBe("`## handoff` -- missing the required `## Backlog` heading");
    expect(errors[1]).toContain("anchors `ctx.rs:12` without its directory");
    expect(errors[2]).toContain("carries no repo-rooted `file:NN` anchor");
  });
});

describe("nv session stripTrailers", () => {
  test("a trailer and prose boilerplate go, and a quoted one stays", () => {
    const { text, removed } = stripTrailers("feat(x): y\n\nBody.\n\nCo-Authored-By: A <a@b>\n🤖 Generated with [Tool](https://x)\n");
    expect(removed).toBe(2);
    expect(text).toBe("feat(x): y\n\nBody.\n");
    const quoted = "feat(x): y\n\n`🤖 Generated with [tool](url)` is the commonest spelling.\n";
    expect(stripTrailers(quoted)).toEqual({ text: quoted, removed: 0 });
  });
});
