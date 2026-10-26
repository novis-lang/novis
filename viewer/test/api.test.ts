import { describe, expect, test } from "bun:test";
import { requestQuery, traceKey, type RequestPage } from "../src/api.ts";
import fixture from "./fixtures/requests.json";

// `crates/nvs-cli/tests/debug_api.rs` writes the fixture from a real index. This assignment is the
// check that the types in `src/api.ts` describe what the service sends: `tsc` fails when they do not.
const page: RequestPage = fixture;

describe("the request list's API", () => {
  test("the fixture is one page, newest first", () => {
    expect(page.requests.length).toBeGreaterThan(0);
    const ids = page.requests.map((row) => row.id);
    expect(ids).toEqual([...ids].sort((a, b) => b - a));
    expect(page.requests.some((row) => row.exception !== null)).toBe(true);
  });

  test("a filter becomes the query string, and an unset key is left out", () => {
    expect(requestQuery({ before: 40, limit: 20 })).toBe("before=40&limit=20");
    expect(requestQuery({})).toBe("");
  });

  test("a pasted traceresponse value or trace id gives its key", () => {
    expect(traceKey("00-4BF92F3577B34DA6A3CE929D0E0E4736-00f067aa0ba902b7-01")).toEqual({
      trace: "4bf92f3577b34da6a3ce929d0e0e4736",
      span: "00f067aa0ba902b7",
    });
    expect(traceKey(" 4bf92f3577b34da6a3ce929d0e0e4736 ")).toEqual({ trace: "4bf92f3577b34da6a3ce929d0e0e4736" });
    expect(traceKey("/orders/7")).toBeNull();
  });
});
