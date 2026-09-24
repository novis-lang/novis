import { describe, expect, test } from "bun:test";
import { citations, headings, section } from "../lib/prose.ts";

const DOC = [
  "# Title", //                                       1
  "",
  "See `rule:types/conversion` and ADR 0004.", //      3
  "",
  "## 4 Four", //                                      5
  "",
  "````sh", //                                         7
  "# not a heading, rule:fake/one",
  "```", //                                            a shorter fence does not close it
  "````", //                                           10 closes the fence
  "",
  "### 4.1 Nested", //                                 12
  "A [link](../x.md) and `[not](y.md)`; 0007 § 3 and decisions/0010.md.",
  "~~~",
  "## inside tilde",
  "~~~",
  "## Five ##", //                                     17
  "last",
].join("\n");

describe("prose", () => {
  test("headings skip fenced blocks of either kind, and drop a closing run of #", () => {
    expect(headings(DOC)).toEqual([
      { level: 1, text: "Title", line: 1 },
      { level: 2, text: "4 Four", line: 5 },
      { level: 3, text: "4.1 Nested", line: 12 },
      { level: 2, text: "Five", line: 17 },
    ]);
  });

  test("a section runs to the next heading at its level or above", () => {
    const four = section(DOC, "## 4");
    expect(four?.split("\n")[0]).toBe("## 4 Four");
    expect(four).toContain("### 4.1 Nested");
    expect(four).not.toContain("## Five");
    expect(section(DOC, "4.1 Nested")?.split("\n").length).toBe(6);
    expect(section(DOC, "### Five")).toBeNull();
    expect(section(DOC, "Five")).toBe("## Five ##\nlast\n");
  });

  test("citations are read outside fences, links outside code spans", () => {
    expect(citations(DOC)).toEqual([
      { kind: "rule", target: "types/conversion", line: 3 },
      { kind: "decision", target: "0004", line: 3 },
      { kind: "decision", target: "0007", line: 13 },
      { kind: "decision", target: "0010", line: 13 },
      { kind: "link", target: "../x.md", line: 13 },
    ]);
  });

  test("CRLF text reads the same as LF", () => {
    expect(headings(DOC.replace(/\n/g, "\r\n"))).toEqual(headings(DOC));
  });
});
