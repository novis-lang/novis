// The decision record and deleted doc guards in `bun nv rules --check`. The cases name every form they
// refuse, so this file carries `nv-rules:record-examples`.
import { describe, expect, test } from "bun:test";
import { RECORD_DIRS, bareMarkFindings, bareMarks, deletedDocFindings, markCounts, recordFindings } from "../cmd/rules.ts";

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
    expect(doc("see [docs/design.md](docs/design.md)")).toHaveLength(1);
    expect(doc("// the signature column of `docs/spec/01-core-library.md`")).toHaveLength(1);
    expect(doc("/// `docs/perf/userland-gap.md` § D is the measurement")).toHaveLength(1);
  });

  test("the perf ledgers under data/perf are not a deleted doc", () => {
    expect(doc("//     bun nv proofs --record-perf writes data/perf/members.ndjson")).toEqual([]);
    expect(doc("[the ledger](../../data/perf/bench-budget.json)")).toEqual([]);
  });

  test("a deleted doc cited by a relative link is a finding", () => {
    expect(doc("[the frozen design](../plan/design.md)")).toHaveLength(1);
    expect(doc("[the plan](implementation-plan.md)")).toHaveLength(1);
    expect(doc("[how it works](../design.md)")).toHaveLength(1);
    expect(doc("[the Core spec](../spec/01-core-library.md#10-coreerror)")).toHaveLength(1);
    expect(doc("[the pass](../perf/performance-pass.md)")).toHaveLength(1);
  });

  test("a public page that moved to the website is a deleted doc, and the files that took its rest are not", () => {
    expect(doc("# `docs/docker.md` § *Building an image locally*")).toHaveLength(1);
    expect(doc("([why](docs/why-tainted-and-secret.md))")).toHaveLength(1);
    expect(doc("# `docker/README.md` § *Building an image locally*")).toEqual([]);
    expect(doc("`website/src/content/docs/guides/production/docker.mdx`")).toEqual([]);
  });

  test("the reference that became /llms-full.txt is a deleted doc, and a file named like it is not", () => {
    expect(doc("`docs/novis.md` is built from this command")).toHaveLength(1);
    expect(doc("[`docs/novis.md`](../novis.md) is the one-file reference")).toHaveLength(1);
    expect(doc("the old `.cursor/rules/novis.mdc` is retired")).toEqual([]);
    expect(doc("the website's `/llms-full.txt`")).toEqual([]);
  });

  test("the reference findings list is a deleted doc by its file name in any form", () => {
    expect(doc("// `docs/reference/findings.md`'s P15 is the report")).toHaveLength(1);
    expect(doc("/// findings.md's D33, decided here")).toHaveLength(1);
    expect(doc("[the list](findings.md#triage)")).toHaveLength(1);
    expect(doc("for (const f of findings) report(f);")).toEqual([]);
  });

  test("the Core spec cited by a section mark is a finding in either case", () => {
    expect(doc("// Spec § 16's `Core\\Net`")).toHaveLength(1);
    expect(doc("/// what spec § 4 says")).toHaveLength(1);
    expect(doc("/// a field the spec derives")).toEqual([]);
  });

  test("a doc that still exists is not a finding, and a file carrying the marker is skipped", () => {
    expect(doc("[the rules](../rules/types.md) and docs/setup.md")).toEqual([]);
    expect(doc("see [the ground rules](docs/ground-rules.md)")).toEqual([]);
    expect(doc("docs/plan/m10.md\n// nv-rules:record-examples")).toEqual([]);
  });
});

describe("nv rules --check, bare section marks", () => {
  test("a mark after a standard's name is kept", () => {
    expect(bareMarks("// RFC 3986 § 5.4 resolves a reference")).toEqual([]);
    expect(bareMarks("/// [MS-TDS] § 2.2.6 and MS-DTYP § 2.3.4")).toEqual([]);
    expect(bareMarks("// (FIPS 180-4 § 6.2) and (XML 1.0 § 2.3)")).toEqual([]);
    expect(bareMarks("// RFC 9110 §§ 8.8–8.9")).toEqual([]);
    expect(bareMarks("/// WebAuthn Level 3 § 16.2 and § 16.11, WHATWG URL § 5")).toEqual([]);
  });

  test("a second mark of the same standard, joined by a list word, is kept", () => {
    expect(bareMarks("// RFC 9110 § 8.8 and § 13.1, RFC 6265 § 4.1, § 5.2")).toEqual([]);
  });

  test("a mark citing a heading in asterisks is kept", () => {
    expect(bareMarks("//! `AGENTS.md` § *The priority ordering* and § *Text an end user reads*")).toEqual([]);
  });

  test("a mark that names no document is bare", () => {
    expect(bareMarks("// § 3")).toEqual([1]);
    expect(bareMarks("ok\n/// because § 2.1 says so, and § B")).toEqual([2, 2]);
    expect(bareMarks("// `docs/reference/README.md` § 4")).toEqual([1]);
    expect(bareMarks("// RFC 3986 § 3 says this. The module's § 4 says that.")).toEqual([1]);
    expect(bareMarks("// § Safepoints")).toEqual([1]);
  });

  const two = "// § 1\n// § 2";

  test("a count that rose is a finding, and so is a mark in a file the record does not name", () => {
    expect(bareMarkFindings({ "crates/x.rs": 1 }, [["crates/x.rs", two]])).toEqual([
      { where: "crates/x.rs", what: "has 2 bare section mark(s), and its record allows 1 (lines 1, 2)" },
    ]);
    expect(bareMarkFindings({}, [["crates/new.rs", "// § 1"]])).toHaveLength(1);
  });

  test("a count that fell or held passes", () => {
    expect(bareMarkFindings({ "crates/x.rs": 3 }, [["crates/x.rs", two]])).toEqual([]);
    expect(bareMarkFindings({ "crates/x.rs": 2 }, [["crates/x.rs", two]])).toEqual([]);
  });

  test("the record names each file with a bare mark, in path order", () => {
    expect(markCounts([["crates/b.rs", two], ["crates/a.rs", "// RFC 3986 § 1"], ["crates/A/c.rs", "// § 9"]])).toEqual({
      "crates/A/c.rs": 1,
      "crates/b.rs": 2,
    });
  });
});
