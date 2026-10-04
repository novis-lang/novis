import { expect, test } from "bun:test";
import { join } from "node:path";
import { agrees, type Batch, batchSizes, CEILING, countsAgree, increments, judgeCounts, rebased, slopeOf, START, withBatch } from "../cmd/scaling.ts";
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

test("an increment is the cost beyond the batch before, per added operation, so set-up cancels", () => {
  const b = ramp((n) => 5 * n, 123_456);
  expect(increments(b, (x) => x.counts.statements!)).toEqual([5, 5, 5]);
});
