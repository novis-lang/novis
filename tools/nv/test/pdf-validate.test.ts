// `bun nv pdf-validate`: veraPDF's machine-readable report is read to its verdict and its failed rules,
// and the programs cover the three standards. Nothing here starts Docker or the binary.

import { expect, test } from "bun:test";
import { FIXTURES, VERAPDF_IMAGE, verdict } from "../cmd/pdf-validate.ts";

const COMPLIANT = `<?xml version="1.0" encoding="utf-8"?>
<report><jobs><job><item size="1"><name>/data/a2b.pdf</name></item>
<validationReport jobEndStatus="normal" profileName="PDF/A-2B validation profile" statement="PDF file is compliant with Validation Profile requirements." isCompliant="true">
<details passedRules="146" failedRules="0" passedChecks="400" failedChecks="0"></details>
</validationReport></job></jobs></report>`;

const FAILED = `<report><jobs><job>
<validationReport jobEndStatus="normal" isCompliant="false" profileName="PDF/A-2B validation profile">
<details passedRules="144" failedRules="2" passedChecks="390" failedChecks="3">
<rule specification="ISO 19005-2:2011" clause="6.6.4" testNumber="1" status="failed" failedChecks="1">
<description>The PDF/A version &amp; conformance level shall be specified</description><object>MainXMPPackage</object></rule>
<rule specification="ISO 19005-2:2011" clause="6.2.4.3" testNumber="2" status="passed" passedChecks="4"><description>passes</description></rule>
<rule status="failed" clause="6.1.3" testNumber="4" specification="ISO 19005-2:2011"><description>Encryption is not allowed</description></rule>
</details></validationReport></job></jobs></report>`;

test("a compliant report has no failed rules", () => {
  expect(verdict(COMPLIANT)).toEqual({ compliant: true, failed: [] });
});

test("a non-compliant report names each failed rule, whatever its attribute order", () => {
  expect(verdict(FAILED)).toEqual({
    compliant: false,
    failed: [
      "ISO 19005-2:2011 6.6.4-1: The PDF/A version & conformance level shall be specified",
      "ISO 19005-2:2011 6.1.3-4: Encryption is not allowed",
    ],
  });
});

test("output with no validation report is an error, not a pass", () => {
  const v = verdict("Exception in thread main");
  expect(v.compliant).toBe(false);
  expect(v.error).toContain("no validation report");
});

test("the programs cover PDF/A-2b, PDF/A-3b and PDF/UA-1, each writing its own file", () => {
  expect(FIXTURES.map((f) => f.flavour)).toEqual(["2b", "3b", "ua1"]);
  for (const f of FIXTURES) expect(f.source).toContain(`Core\\IO::writeStream("${f.name}.pdf"`);
  expect(FIXTURES.find((f) => f.flavour === "3b")!.source).toContain("->invoiceXml($xml)");
});

test("the image is pinned by digest", () => {
  expect(VERAPDF_IMAGE).toMatch(/^verapdf\/cli@sha256:[0-9a-f]{64}$/);
});
