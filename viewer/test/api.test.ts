import { describe, expect, test } from "bun:test";
import {
  exceptionKey,
  exceptionQuery,
  requestQuery,
  snippetParts,
  streamFilter,
  streamQuery,
  traceKey,
  type CoverageMap,
  type ExceptionPage,
  type ExceptionRequests,
  type RequestDetail,
  type RequestPage,
  type SearchResults,
  type Settings,
  type SourceFile,
  type StreamPage,
} from "../src/api.ts";
import { heat } from "../src/heat.ts";
import coverageFixture from "./fixtures/coverage.json";
import exceptionFixture from "./fixtures/exception.json";
import exceptionsFixture from "./fixtures/exceptions.json";
import detailFixture from "./fixtures/request.json";
import fixture from "./fixtures/requests.json";
import searchFixture from "./fixtures/search.json";
import settingsFixture from "./fixtures/settings.json";
import sourceFixture from "./fixtures/source.json";
import streamFixture from "./fixtures/stream.json";

// `crates/nvs-cli/tests/debug_api.rs` writes the fixtures from a real index. These assignments are
// the check that the types in `src/api.ts` describe what the service sends: `tsc` fails when they
// do not. A JSON import types `kind`, `view`, `editor` and `theme` as `string`, so the one cast for each
// narrows it and checks the rest.
const page: RequestPage = fixture;
/** A JSON import types a header pair as `string[]`. */
const pair = (header: string[]): [string, string] => [header[0]!, header[1]!];
const detail: RequestDetail = {
  ...detailFixture,
  events: detailFixture.events.map((event) => ({ ...event, kind: event.kind as RequestDetail["events"][number]["kind"] })),
  input: {
    ...detailFixture.input,
    headers: detailFixture.input.headers.map(pair),
    response_headers: detailFixture.input.response_headers.map(pair),
  },
};
const streamPage: StreamPage = {
  ...streamFixture,
  records: streamFixture.records.map((record) => ({ ...record, kind: record.kind as StreamPage["records"][number]["kind"] })),
};
const results: SearchResults = {
  ...searchFixture,
  groups: searchFixture.groups.map((group) => ({
    ...group,
    hits: group.hits.map((hit) => ({ ...hit, kind: hit.kind as SearchResults["groups"][number]["hits"][number]["kind"] })),
  })),
};
const saved: Settings = {
  ...settingsFixture,
  editor: settingsFixture.editor as Settings["editor"],
  theme: settingsFixture.theme as Settings["theme"],
  filters: settingsFixture.filters.map((filter) => ({ ...filter, view: filter.view as Settings["filters"][number]["view"] })),
};
const groups: ExceptionPage = exceptionsFixture;
const group: ExceptionRequests = exceptionFixture;
const counted: CoverageMap = coverageFixture;
const text: SourceFile = sourceFixture;

describe("the request list's API", () => {
  test("the fixture is one page, newest first", () => {
    expect(page.requests.length).toBeGreaterThan(0);
    const ids = page.requests.map((row) => row.id);
    expect(ids).toEqual([...ids].sort((a, b) => b - a));
    expect(page.requests.some((row) => row.exception !== null)).toBe(true);
  });

  test("the request fixture has every list, in time order", () => {
    const starts = detail.events.map((event) => event.start_us);
    expect(starts).toEqual([...starts].sort((a, b) => a - b));
    expect(detail.dumps.length).toBeGreaterThan(0);
    expect(detail.logs.length).toBeGreaterThan(0);
    expect(detail.exception?.message).toBeTruthy();
  });

  test("a filter becomes the query string, and an unset key is left out", () => {
    expect(requestQuery({ before: 40, limit: 20 })).toBe("before=40&limit=20");
    expect(requestQuery({})).toBe("");
  });

  test("the stream fixture is one page, newest first, and names each record's request", () => {
    const times = streamPage.records.map((record) => record.at_us);
    expect(times).toEqual([...times].sort((a, b) => b - a));
    expect(new Set(streamPage.records.map((record) => record.kind))).toContain("log");
    expect(streamPage.records.every((record) => record.request !== null)).toBe(true);
  });

  test("a stream filter round-trips through its link, and `kind` keeps its commas", () => {
    const query = streamQuery({ kinds: ["log", "dump"], level: "error", route: "/items/{id}" });
    expect(query).toBe("kind=log,dump&level=error&route=%2Fitems%2F%7Bid%7D");
    expect(streamFilter(query)).toEqual({ kinds: ["log", "dump"], level: "error", route: "/items/{id}" });
    expect(streamQuery({ kinds: [] }, { before: "17.4", limit: 5 })).toBe("before=17.4&limit=5");
    expect(streamFilter("kind=log,page")).toEqual({ kinds: ["log"] });
  });

  test("the search fixture groups its matches by request, and each snippet marks a word", () => {
    expect(results.groups.length).toBeGreaterThan(0);
    for (const group of results.groups) {
      for (const hit of group.hits) expect(snippetParts(hit.snippet).some((part) => part.match)).toBe(true);
    }
    expect(snippetParts("\u0002stock\u0003 is empty")).toEqual([
      { text: "stock", match: true },
      { text: " is empty", match: false },
    ]);
    expect(snippetParts("no match")).toEqual([{ text: "no match", match: false }]);
  });

  test("the settings fixture has a saved filter", () => {
    expect(saved.filters[0]?.view).toBe("search");
    expect(new URLSearchParams(saved.filters[0]!.query).get("q")).toBe("route:/orders duration:>10ms");
  });

  test("the exceptions fixtures group throws by class and site, and a group lists its requests", () => {
    expect(groups.groups.length).toBeGreaterThan(0);
    for (const one of groups.groups) {
      expect(one.chart.length).toBe(24);
      expect(one.chart.reduce((a, b) => a + b, 0)).toBe(one.count);
      expect(one.routes.length).toBeLessThanOrEqual(5);
    }
    expect(group.requests.length).toBe(group.group.count);
    for (const row of group.requests) expect(row.exception).toEqual({ class: group.group.class, file: group.group.file, line: group.group.line });
  });

  test("a group's link names its class and site, and reads back", () => {
    const key = { class: "Shop\\OutOfStock", file: "src/Shop/Orders.nvs", line: 42 };
    expect(exceptionQuery(key)).toBe("class=Shop%5COutOfStock&file=src%2FShop%2FOrders.nvs&line=42");
    expect(exceptionKey(exceptionQuery(key))).toEqual(key);
    expect(exceptionKey("class=A&file=b.nvs")).toBeNull();
    expect(exceptionKey("")).toBeNull();
  });

  test("the settings fixture has an editor and a path mapping", () => {
    expect(saved.editor).toBe("custom");
    expect(saved.editor_template).toContain("{file}");
    expect(saved.path_mappings.length).toBeGreaterThan(0);
  });

  test("the coverage fixture counts lines of a file the source fixture has", () => {
    const lines = counted.files[text.file];
    expect(lines).toBeDefined();
    const total = text.text.split("\n").length;
    for (const line of Object.keys(lines!)) expect(Number(line)).toBeLessThanOrEqual(total);
  });

  test("a line's heat is 0 when it did not run, and rises on a log scale to 4", () => {
    expect(heat(0, 100)).toBe(0);
    expect(heat(1, 1)).toBe(4);
    expect(heat(1, 100)).toBe(1);
    expect(heat(100, 100)).toBe(4);
    expect(heat(10, 100)).toBeGreaterThan(heat(2, 100));
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
