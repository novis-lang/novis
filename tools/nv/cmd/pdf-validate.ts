// `bun nv pdf-validate`: the files `Novis\Pdf` writes under a standard, checked by veraPDF
// (`rule:core-classes/pdf-standards-and-passwords`).
//
//     bun nv pdf-validate [--bin <nvs>] [--keep]
//
// The command writes three small programs under `.agent-tmp/pdf-validate/`, one per standard: PDF/A-2b,
// PDF/A-3b with a Factur-X invoice attached, and PDF/UA-1. It runs each with the built binary
// (`--bin`, else `target/release/nvs`), and each writes its PDF beside itself. It then runs veraPDF's
// own image on each file with that standard's flavour, and reads the `isCompliant` verdict of the
// machine-readable report veraPDF writes by default.
//
// The image is pinned by digest, `VERAPDF_IMAGE`: the digest Docker Hub listed for `verapdf/cli:v1.30.3`
// when the pin was taken. A new veraPDF is a new pin, taken from `docker buildx imagetools inspect
// verapdf/cli:<tag>`. The pull runs on its own first, without the hang watch, because it waits on the
// network on purpose.
//
// Where Docker runs no Linux containers, the files are still rendered, and the command says why veraPDF
// did not run and exits 0, as a `requires: <service>` program is skipped (`dockerAbsent` in
// `proofs/run.ts`, the one probe). CI runs it on the `proofs` job's Linux leg.
//
// It exits 0 when every file is compliant or veraPDF cannot run here, 1 naming each file that is not
// compliant with its first failed rules, or a render that failed, and 2 on a usage error. The scratch
// folder is deleted on success, and kept on a failure, or with `--keep`, for a look at the files.

import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { isAbsolute, join } from "node:path";
import { ROOT, rel } from "../lib/paths.ts";
import { failLine, run as runProc } from "../lib/proc.ts";
import { releaseCli } from "../lib/relink.ts";
import { dockerAbsent } from "../proofs/run.ts";

export const summary = "veraPDF on the PDF/A-2b, PDF/A-3b and PDF/UA-1 files the binary writes: nv pdf-validate [--bin <nvs>] [--keep]";

const USAGE = "usage: bun nv pdf-validate [--bin <nvs>] [--keep]";

/** veraPDF's CLI image, `verapdf/cli:v1.30.3`, by digest. */
export const VERAPDF_IMAGE = "verapdf/cli@sha256:d74b7026c18a0f171e73c153b617fe6520e1511feb24ca98a348b2bb897669fb";

/** Where the programs and their PDFs are written. */
export const SCRATCH = join(ROOT, ".agent-tmp", "pdf-validate");

/** How many failed rules a non-compliant file's line names. */
const RULES_SHOWN = 5;

export interface Fixture {
  /** The program's and the PDF's base name. */
  name: string;
  /** veraPDF's `--flavour` for the standard. */
  flavour: string;
  /** The program, which writes `<name>.pdf` beside itself. */
  source: string;
}

const CREATED = "->created(Core\\Time::fromIso('2026-03-01T09:00:00Z'))";

/** A program that renders `html` under `standard`, with `calls` after `standard(...)`, and writes `<name>.pdf`. */
function program(name: string, standard: string, html: string, calls: string[], before: string[] = []): string {
  return [
    "<?nvs",
    "use Novis\\Pdf\\Document;",
    ...before,
    `string $html = ${html};`,
    "var $rendered = Document::fromHtml($html)",
    `    ->standard(Document::Standard::${standard})`,
    ...calls.map((c) => `    ${c}`),
    "    ->render();",
    `Core\\IO::writeStream("${name}.pdf", [$rendered->bytes], {});`,
    "",
  ].join("\n");
}

const INVOICE_XML = [
  `"<?xml version=\\"1.0\\" encoding=\\"UTF-8\\"?>"`,
  `"<rsm:CrossIndustryInvoice xmlns:rsm=\\"urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100\\""`,
  `" xmlns:ram=\\"urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100\\">"`,
  `'<rsm:ExchangedDocument><ram:ID>Shop-1042</ram:ID><ram:TypeCode>380</ram:TypeCode></rsm:ExchangedDocument>'`,
  `'</rsm:CrossIndustryInvoice>'`,
].join("\n    . ");

/** The three programs, one per standard veraPDF checks. */
export const FIXTURES: Fixture[] = [
  {
    name: "a2b",
    flavour: "2b",
    source: program(
      "a2b",
      "PdfA2b",
      `"<html lang=\\"en\\"><head><title>Certificate</title>"
    . '<style>@page { size: A4 landscape } body { text-align: center }</style></head>'
    . '<body><h1>Certificate</h1><p>Ada Lovelace finished the course Bookkeeping for Shops.</p></body></html>'`,
      [CREATED],
    ),
  },
  {
    name: "a3b",
    flavour: "3b",
    source: program(
      "a3b",
      "PdfA3b",
      `"<html lang=\\"en\\"><head><title>Invoice Shop-1042</title></head>"
    . '<body><h1>Invoice Shop-1042</h1><p>Total: 5.70 EUR</p></body></html>'`,
      [CREATED, "->invoiceXml($xml)"],
      [`string $xml = ${INVOICE_XML};`],
    ),
  },
  {
    name: "ua1",
    flavour: "ua1",
    source: program(
      "ua1",
      "PdfUa1",
      `"<html lang=\\"en\\"><head><title>Sales report</title></head><body>"
    . '<h1>Sales report</h1><p>Orders fell in June.</p>'
    . "<img src=\\"data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' width='60' height='40'><rect width='60' height='40' fill='gray'/></svg>\\" alt=\\"A gray box.\\">"
    . '<ul><li>May: 80 orders</li><li>June: 50 orders</li></ul>'
    . '<table><tr><th>Month</th><th>Orders</th></tr><tr><td>May</td><td>80</td></tr>'
    . '<tr><td>June</td><td>50</td></tr></table></body></html>'`,
      [],
    ),
  },
];

export interface Verdict {
  /** True when veraPDF's report says the file is compliant. */
  compliant: boolean;
  /** Each failed rule, as `<specification> <clause>-<test>: <description>`. */
  failed: string[];
  /** Set when the output holds no validation report: what went wrong instead. */
  error?: string;
}

function attr(attrs: string, name: string): string | undefined {
  return new RegExp(`\\b${name}="([^"]*)"`).exec(attrs)?.[1];
}

function unescapeXml(text: string): string {
  return text.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"').replace(/&apos;/g, "'").replace(/&amp;/g, "&");
}

/** The verdict in veraPDF's machine-readable XML report, for a run on one file. */
export function verdict(xml: string): Verdict {
  const report = /<validationReport\b([^>]*)>/.exec(xml);
  const compliant = report ? attr(report[1]!, "isCompliant") : undefined;
  if (compliant === undefined) {
    const status = /<taskException\b[^>]*>([\s\S]*?)<\/taskException>/.exec(xml)?.[1];
    return { compliant: false, failed: [], error: status ? unescapeXml(status.replace(/<[^>]+>/g, " ").trim()) : "no validation report in veraPDF's output" };
  }
  const failed: string[] = [];
  for (const m of xml.matchAll(/<rule\b([^>]*)>([\s\S]*?)<\/rule>/g)) {
    const attrs = m[1]!;
    if (attr(attrs, "status") !== "failed") continue;
    const description = /<description>([\s\S]*?)<\/description>/.exec(m[2]!)?.[1]?.trim() ?? "";
    failed.push(`${attr(attrs, "specification") ?? "?"} ${attr(attrs, "clause") ?? "?"}-${attr(attrs, "testNumber") ?? "?"}: ${unescapeXml(description)}`);
  }
  return { compliant: compliant === "true", failed };
}

interface Options {
  bin: string;
  keep: boolean;
}

function parse(args: string[]): Options | string {
  const o: Options = { bin: releaseCli(), keep: false };
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    if (a === "--keep") o.keep = true;
    else if (a === "--bin") {
      const v = args[++i];
      if (v === undefined) return "--bin needs a path";
      o.bin = isAbsolute(v) ? v : join(ROOT, v);
    } else return `unknown argument ${a}`;
  }
  return o;
}

export async function run(args: string[]): Promise<number> {
  if (args.includes("--help") || args.includes("-h")) {
    console.log(USAGE);
    return 0;
  }
  const o = parse(args);
  if (typeof o === "string") {
    console.error(`nv pdf-validate: ${o}\n${USAGE}`);
    return 2;
  }
  if (!existsSync(o.bin)) {
    console.error(`nv pdf-validate: no binary at ${rel(o.bin)}: build it, or name one with --bin`);
    return 1;
  }

  rmSync(SCRATCH, { recursive: true, force: true });
  mkdirSync(SCRATCH, { recursive: true });
  const kept = `the files are in ${rel(SCRATCH)}`;

  for (const f of FIXTURES) {
    const path = join(SCRATCH, `${f.name}.nvs`);
    writeFileSync(path, f.source);
    const r = await runProc([o.bin, "run", path], { cwd: ROOT, timeoutMs: 120_000 });
    if (r.code !== 0 || !existsSync(join(SCRATCH, `${f.name}.pdf`))) {
      console.error(`nv pdf-validate: ${f.name}.nvs did not render: ${failLine(r.stderr + r.stdout, r.code, rel(o.bin))}; ${kept}`);
      return 1;
    }
  }

  const absent = await dockerAbsent();
  if (absent !== null) {
    console.log(`nv pdf-validate: rendered ${FIXTURES.length} files, and veraPDF did not run: ${absent}`);
    if (!o.keep) rmSync(SCRATCH, { recursive: true, force: true });
    return 0;
  }

  const pull = await runProc(["docker", "pull", "--quiet", VERAPDF_IMAGE], { cwd: ROOT, timeoutMs: 600_000, hang: false });
  if (pull.code !== 0) {
    console.error(`nv pdf-validate: ${failLine(pull.stderr + pull.stdout, pull.code, `docker pull ${VERAPDF_IMAGE}`)}`);
    return 1;
  }

  let bad = 0;
  for (const f of FIXTURES) {
    const r = await runProc(
      ["docker", "run", "--rm", "-v", `${SCRATCH}:/data:ro`, VERAPDF_IMAGE, "--flavour", f.flavour, `/data/${f.name}.pdf`],
      { cwd: ROOT, timeoutMs: 300_000 },
    );
    const v = verdict(r.stdout);
    if (v.error !== undefined) {
      console.error(`nv pdf-validate: veraPDF did not check ${f.name}.pdf (exit ${r.code}): ${v.error}`);
      if (r.stderr.trim()) console.error(r.stderr.trim().split(/\r?\n/).slice(-5).join("\n"));
      bad++;
    } else if (!v.compliant) {
      console.error(`nv pdf-validate: ${f.name}.pdf is not compliant with flavour ${f.flavour}, ${v.failed.length} failed rules:`);
      for (const rule of v.failed.slice(0, RULES_SHOWN)) console.error(`  ${rule}`);
      if (v.failed.length > RULES_SHOWN) console.error(`  ... and ${v.failed.length - RULES_SHOWN} more`);
      bad++;
    } else {
      console.log(`${f.name}.pdf is compliant with flavour ${f.flavour}`);
    }
  }
  if (bad > 0) {
    console.error(`nv pdf-validate: ${bad} of ${FIXTURES.length} files failed; ${kept}`);
    return 1;
  }
  if (!o.keep) rmSync(SCRATCH, { recursive: true, force: true });
  return 0;
}
