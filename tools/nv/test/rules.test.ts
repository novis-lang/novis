// The decision record guard in `bun nv rules --check`. The cases name every record form, so this file
// carries `nv-rules:record-examples`.
import { describe, expect, test } from "bun:test";
import { RECORD_DIRS, recordFindings } from "../cmd/rules.ts";

const one = (text: string) => recordFindings([], [["docs/x.md", text]]);

describe("nv rules --check, decision records", () => {
  test("each record directory present is a finding", () => {
    expect(recordFindings(RECORD_DIRS, []).map((f) => f.where)).toEqual(RECORD_DIRS);
  });

  test("a record cited by its number is a finding", () => {
    expect(one("see ADR 0004 for why")).toEqual([{ where: "docs/x.md:1", what: "cites a decision record" }]);
    expect(one("ADRs 0012 and 0013")).toHaveLength(1);
  });

  test("a record cited by its path is a finding", () => {
    expect(one("first\nsee docs/decisions/0199.md")).toEqual([{ where: "docs/x.md:2", what: "cites a decision record" }]);
    expect(one("`data/decisions/0001.json`")).toHaveLength(1);
  });

  test("a bare section mark is a finding only when its number leads with a zero", () => {
    expect(one("as 0107 § 3 says")).toHaveLength(1);
    expect(one("RFC 3986 § 3 and v1.0123 § 2")).toEqual([]);
  });

  test("the old record directory named in a path is a finding", () => {
    expect(one("[the record](docs/adr/tooling-parity.md)")).toHaveLength(1);
  });

  test("a clean tree has no finding, and a file carrying the marker is skipped", () => {
    expect(one("a rule holds its reasons and its Declined list")).toEqual([]);
    expect(one("ADR 0004\n<!-- nv-rules:record-examples -->")).toEqual([]);
  });
});
