import { expect, test } from "bun:test";
import { pagePath, pickTimes } from "../cmd/walkthrough.ts";

const beats = [
  { title: "a", caption: "", start: 0, end: 2.5 },
  { title: "b", caption: "", start: 2.5, end: 6 },
];

test("a page path loses its slashes, and Git Bash's rewrite of a leading slash is undone", () => {
  expect(pagePath("in-depth/walkthrough-after-response")).toBe("in-depth/walkthrough-after-response");
  expect(pagePath("/in-depth/walkthrough-after-response/")).toBe("in-depth/walkthrough-after-response");
  expect(pagePath("C:/Program Files/Git/in-depth/walkthrough-after-response/")).toBe("in-depth/walkthrough-after-response");
});

test("frames are taken at every step's end, at the named steps, or at the named times", () => {
  expect(pickTimes(beats)).toEqual([
    { label: "step01", t: 2.5 },
    { label: "step02", t: 6 },
  ]);
  expect(pickTimes(beats, "2")).toEqual([{ label: "step02", t: 6 }]);
  expect(pickTimes(beats, undefined, "1.5,4")).toEqual([
    { label: "t1.5", t: 1.5 },
    { label: "t4", t: 4 },
  ]);
});

test("a step outside the walkthrough or a time that is not a number is a usage error", () => {
  expect(pickTimes(beats, "3")).toBeNull();
  expect(pickTimes(beats, "0")).toBeNull();
  expect(pickTimes(beats, undefined, "soon")).toBeNull();
  expect(pickTimes(beats, undefined, "-1")).toBeNull();
});
