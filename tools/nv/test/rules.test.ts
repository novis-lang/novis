// The decision record and deleted doc guards in `bun nv rules --check`. The cases name every form they
// refuse, so this file carries `nv-rules:record-examples`.
import { describe, expect, test } from "bun:test";
import { RECORD_DIRS, deletedDocFindings, recordFindings } from "../cmd/rules.ts";

const one = (text: string) => recordFindings([], [["docs/x.md", text]]);

describe("nv rules --check, decision records", () => {
  test("each record directory present is a finding", () => {
    expect(recordFindings(RECORD_DIRS, []).map((f) => f.where)).toEqual(RECORD_DIRS);
  });

  test("a record cited by its number is a finding", () => {
    expect(one("see ADR 0004 for why")).toEqual([{ where: "docs/x.md:1", what: "cites a decision record" }]);
    expect(one("ADRs 0012 and 0013")).toHaveLength(1);
  });

  test("a record cited by its name alone, or by a test named for its number, is a finding", () => {
    expect(one("as the ADR says")).toHaveLength(1);
    expect(one("fn the_codes_adr_0098_still_belong() {")).toHaveLength(1);
    expect(one("object::elf::R_AARCH64_ADR_PREL_PG_HI21 => None,")).toEqual([]);
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

const doc = (text: string) => deletedDocFindings([["crates/x.rs", text]]);

describe("nv rules --check, deleted docs", () => {
  test("a deleted doc cited by its repository path is a finding", () => {
    expect(doc("//! `docs/plan/design.md` § *Safepoints*")).toEqual([{ where: "crates/x.rs:1", what: "cites a deleted doc" }]);
    expect(doc("# the policy in docs/implementation-plan.md's section")).toHaveLength(1);
    expect(doc("a note in docs/future-ideas.md")).toHaveLength(1);
  });

  test("a deleted doc cited by a relative link is a finding", () => {
    expect(doc("[the frozen design](../plan/design.md)")).toHaveLength(1);
    expect(doc("[the plan](implementation-plan.md)")).toHaveLength(1);
  });

  test("a doc that still exists is not a finding, and a file carrying the marker is skipped", () => {
    expect(doc("[the rules](../rules/types.md) and docs/setup.md")).toEqual([]);
    expect(doc("see [docs/design.md](docs/design.md)")).toEqual([]);
    expect(doc("docs/plan/m10.md\n// nv-rules:record-examples")).toEqual([]);
  });
});
