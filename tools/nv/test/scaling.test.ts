import { expect, test } from "bun:test";
import { join } from "node:path";
import {
  agrees, AREAS, type Batch, batchSizes, boundsOf, CEILING, clockFlat, compileCounts, COUNT_BOUND, countsAgree, increments, judgeCounts, type Judged, ladderOf, ladderSizes, LSP_URI, lspFrame, lspScript, missingAreas,
  HEADER_VALUE, PEAK_SLACK, PER_CONNECTION, peakGrows, printedFiles, proposed, rebased, serveShape, SHAPED_REQUESTS, slopeOf, START, withBatch,
} from "../cmd/scaling.ts";
import { ROOT } from "../lib/paths.ts";

/** A ramp whose every count costs `cost(n)` at batch `n`, over `fixed` set-up. */
function ramp(cost: (n: number) => number, fixed = 1000): Batch[] {
  return batchSizes(100_000).slice(0, 4).map((size) => {
    const c = fixed + cost(size);
    return { size, counts: { statements: c, calls: c, allocations: c, bytes: c }, ns: 0 };
  });
}

test("the closing line passes the batch where it passed the bench's iterations, and nothing else moves", () => {
  const src = "<?nvs\n// bench: iterations 400_000\necho Bench::run($labels, 400_000) > 400000 ? 'done' : 'no', \"\\n\";\n";
  expect(withBatch(src, 400000, 32)).toBe("<?nvs\n// bench: iterations 400_000\necho Bench::run($labels, 32) > 400000 ? 'done' : 'no', \"\\n\";\n");
  expect(withBatch("echo Bench::run() > 400 ? 'done' : 'no';\n", 400, 32)).toBeNull();
  expect(withBatch("echo Bench::run(400, 400), \"\\n\";\n", 400, 32)).toBeNull();
  expect(withBatch("echo Bench::run(f(400)), \"\\n\";\n", 400, 32)).toBe("echo Bench::run(f(32)), \"\\n\";\n");
});

test("batches double from the start, stay under the bench's own count and the ceiling, and start lower for a small bench", () => {
  const big = batchSizes(400_000);
  expect(big[0]).toBe(START);
  expect(big.at(-1)).toBe(CEILING);
  expect(big.every((s, i) => i === 0 || s === big[i - 1]! * 2)).toBe(true);
  expect(batchSizes(100)).toEqual([8, 16, 32, 64]);
  expect(batchSizes(64)).toEqual([4, 8, 16, 32]);
  expect(batchSizes(4)).toEqual([]);
});

test("a linear count agrees at four batches with slope 1, and a quadratic one agrees with slope 2 and fails", () => {
  const linear = ramp((n) => 7 * n);
  expect(countsAgree(linear)).toBe(true);
  const flat = judgeCounts(linear);
  expect(flat.over).toEqual([]);
  expect(flat.slopes.statements).toBeCloseTo(1, 6);
  const square = ramp((n) => n * n);
  expect(countsAgree(square)).toBe(true);
  const grows = judgeCounts(square);
  expect(grows.slopes.bytes).toBeCloseTo(2, 6);
  expect(grows.over).toHaveLength(4);
});

test("a count that does nothing per operation has no slope and agrees, and a stray jump does not agree", () => {
  expect(slopeOf([0, 0, 0])).toBeNull();
  expect(agrees([0, 0, 0])).toBe(true);
  expect(agrees([0, 0, 0.5])).toBe(false);
  expect(agrees([1, 1.4, 1])).toBe(false);
  expect(agrees([3, 3, 3])).toBe(true);
  expect(slopeOf([3, 3, 3])).toBe(1);
});

test("a copied config names the copy for a path in the copied folder, the original for any other, and leaves the rest", () => {
  const root = join(ROOT, ".agent-tmp", "scaling-test");
  const copied = join(ROOT, "benches", "members", "core", "Db");
  const copyDir = join(root, "benches", "members", "core", "Db");
  const text = 'entry = "benches/members/core/Db/connect.nvs"\nca = "tests"\nallow = ["fs.read"]\nurl = "redis://127.0.0.1:16379"\n';
  expect(rebased(text, ROOT, copyDir, copied, root)).toBe(
    'entry = "connect.nvs"\nca = "../../../../../../tests"\nallow = ["fs.read"]\nurl = "redis://127.0.0.1:16379"\n',
  );
});

test("a ladder declares its start, max and expect, defaults to kind run, and never expects quadratic", () => {
  const src = "<?nvs\n// scaling: start 100\n// scaling: max 100_000\n// scaling: expect nlogn\necho Bench::run(100), \"\\n\";\n";
  expect(ladderOf(src)).toEqual({ kind: "run", start: 100, max: 100000, expect: "nlogn", proposal: false, size: "" });
  expect(ladderOf(src.replace("nlogn", "linear\n// scaling: proposal"))).toMatchObject({ expect: "linear", proposal: true });
  expect(ladderOf(src.replace("nlogn", "quadratic"))).toContain("never accepted");
  expect(ladderOf(src.replace("// scaling: max 100_000\n", ""))).toContain("needs");
  expect(ladderOf(src.replace("expect nlogn", "kind gpu"))).toContain("not one of");
  expect(ladderSizes(100, 1000)).toEqual([100, 200, 400, 800]);
});

test("a made-up linear ladder passes its bound, a quadratic one fails, and an n log n one passes `nlogn` alone", () => {
  expect(judgeCounts(ramp((n) => 9 * n), undefined, boundsOf("linear").count).over).toEqual([]);
  expect(judgeCounts(ramp((n) => n * n), undefined, boundsOf("nlogn").count).over).toHaveLength(4);
  const nlogn = ramp((n) => 50 * n * Math.log2(n));
  expect(judgeCounts(nlogn, undefined, boundsOf("nlogn").count).over).toEqual([]);
  expect(judgeCounts(nlogn, undefined, boundsOf("constant").count).over).toHaveLength(4);
  expect(boundsOf("linear")).toEqual({ count: COUNT_BOUND, clock: 1.5 });
});

test("a ladder marked proposal that grows waits for the user, and one that does not grow keeps its verdict", () => {
  const grows: Judged = { bench: "benches/scaling/arrays/sort.nvs", verdict: "grows", sizes: [16, 32, 64, 128], slopes: {}, clock: null, notes: ["bytes grows"] };
  const ladder = { kind: "run", start: 16, max: 128, expect: "linear", proposal: true, size: "" };
  expect(proposed(grows, ladder).verdict).toBe("proposal");
  expect(proposed(grows, { ...ladder, proposal: false }).verdict).toBe("grows");
  expect(proposed({ ...grows, verdict: "flat" }, ladder).verdict).toBe("flat");
});

test("a server's peak memory may wander under the slack, and one that rises with the requests served fails", () => {
  const served = (peaks: number[]): Batch[] => peaks.map((peak, i) => ({ size: 64 << i, counts: { peak }, ns: 0 }));
  expect(peakGrows(served([20_900, 19_100, 19_700, 19_800]))).toBeNull();
  expect(peakGrows(served([20_000, 20_000, 20_000, 20_000 * (1 + PEAK_SLACK)]))).toBeNull();
  expect(peakGrows(served([20_000, 22_000, 26_000, 34_000]))).toBe("peak memory grows with requests served: 70% above its lowest at 512 requests");
});

test("a serve ladder's size defaults to requests, belongs to serve alone, and shapes each load", () => {
  const src = "// scaling: kind serve\n// scaling: start 16\n// scaling: max 128\n// scaling: expect linear\n";
  expect(ladderOf(src)).toMatchObject({ kind: "serve", size: "requests" });
  expect(ladderOf(`${src}// scaling: size headers\n`)).toMatchObject({ size: "headers" });
  expect(ladderOf(`${src}// scaling: size cookies\n`)).toContain("not one of");
  expect(ladderOf(src.replace("kind serve", "kind run") + "// scaling: size headers\n")).toContain("`serve` ladder alone");
  expect(ladderOf(src.replace("kind serve", "kind fetch"))).toMatchObject({ kind: "fetch", size: "" });
  expect(serveShape("requests", 64)).toEqual({ path: "/", shape: {}, requests: 64, concurrency: 1 });
  expect(serveShape("headers", 3).shape.headers).toEqual([["x-field-0", HEADER_VALUE], ["x-field-1", HEADER_VALUE], ["x-field-2", HEADER_VALUE]]);
  expect(serveShape("header-bytes", 5).shape.headers).toEqual([["x-field", "aaaaa"]]);
  expect(serveShape("body-bytes", 7).shape.body!.length).toBe(7);
  expect(serveShape("query", 3).path).toBe("/?p0=value&p1=value&q=1");
  expect(serveShape("form", 3).shape.body!.toString()).toBe("p0=value&p1=value&q=1");
  expect(serveShape("connections", 8)).toMatchObject({ requests: 8 * PER_CONNECTION, concurrency: 8 });
  expect(serveShape("routes", 512)).toEqual({ path: "/last", shape: {}, requests: SHAPED_REQUESTS, concurrency: 1 });
});

test("a clock whose slope is unread is flat only when the largest size stays near the smallest", () => {
  const at = (ns: number[]): Batch[] => ns.map((n, i) => ({ size: 256 << i, counts: {}, ns: n }));
  expect(clockFlat(at([100, 90, 110, 105, 120]))).toBe(true);
  expect(clockFlat(at([100, 90, 110, 105, 130]))).toBe(false);
  expect(clockFlat(at([100]))).toBe(false);
});

test("an area needs a ladder in its folder, and with a review, a section headed with its name", () => {
  const ladders = AREAS.filter((a) => a !== "json").map((a) => `benches/scaling/${a}/one.nvs`);
  expect(missingAreas(ladders, null)).toEqual({ noLadder: ["json"], noReview: [] });
  const review = AREAS.filter((a) => a !== "lsp").map((a) => `## \`${a}\`\n\nFine.\n`).join("\n");
  expect(missingAreas(ladders, review).noReview).toEqual(["lsp"]);
});

test("the compile line's counts are read by name, and stderr without one gives null", () => {
  expect(compileCounts("no errors\ncompile: tokens=7 nodes=2 names=0 exprs=0\n")).toEqual({ tokens: 7, nodes: 2, names: 0, exprs: 0 });
  expect(compileCounts("compile: tokens=7 nodes=2 names=0 exprs=0 ir=80\r\ncount: statements=1 calls=0 allocations=13 bytes=7081\r\n")!.ir).toBe(80);
  expect(compileCounts("count: statements=1 calls=0 allocations=13 bytes=7081\n")).toBeNull();
  expect(ladderOf("// scaling: kind compile\n// scaling: start 50\n// scaling: max 1600\n// scaling: expect linear\n")).toMatchObject({ kind: "compile" });
});

test("a printed program splits at its file lines, and one without them is the program alone", () => {
  expect(printedFiles("<?nvs\necho 1;\n")).toEqual({ program: "<?nvs\necho 1;\n", files: [] });
  const split = printedFiles("<?nvs\nrequire 'a.nvs';\n// scaling: file a.nvs\n<?nvs\nclass A {}\n// scaling: file b-1.nvs\r\n<?nvs\n");
  expect(split.program).toBe("<?nvs\nrequire 'a.nvs';\n");
  expect(split.files).toEqual([["a.nvs", "<?nvs\nclass A {}\n"], ["b-1.nvs", "<?nvs\n"]]);
});

test("an lsp session opens the document without its cursor, edits it, and asks at the cursor", () => {
  expect(lspScript("<?nvs\necho 1;\n")).toBeNull();
  const script = lspScript("<?nvs\nécho Base::pr<|>ice(1);\n") as { id?: number; method: string; params: any }[];
  expect(script.map((m) => m.method)).toEqual([
    "initialize", "initialized", "textDocument/didOpen", "textDocument/didChange", "textDocument/completion", "textDocument/hover", "textDocument/references", "textDocument/codeLens", "shutdown", "exit",
  ]);
  expect(script.filter((m) => m.id !== undefined).map((m) => m.id)).toEqual([1, 2, 3, 4, 5, 6]);
  expect(script[2]!.params.textDocument).toMatchObject({ uri: LSP_URI, version: 1, text: "<?nvs\nécho Base::price(1);\n" });
  expect(script[3]!.params.contentChanges[0].text).toBe("<?nvs\nécho Base::price(1);\n// edited\n");
  expect(script[4]!.params.position).toEqual({ line: 1, character: 13 });
  expect(lspFrame({ id: 1, method: "é" })).toBe('Content-Length: 38\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"é"}');
});

test("an increment is the cost beyond the batch before, per added operation, so set-up cancels", () => {
  const b = ramp((n) => 5 * n, 123_456);
  expect(increments(b, (x) => x.counts.statements!)).toEqual([5, 5, 5]);
});
