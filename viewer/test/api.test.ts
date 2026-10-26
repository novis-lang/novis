import { describe, expect, test } from "bun:test";
import {
  requestQuery,
  snippetParts,
  streamFilter,
  streamQuery,
  traceKey,
  type RequestDetail,
  type RequestPage,
  type SearchResults,
  type Settings,
  type StreamPage,
} from "../src/api.ts";
import detailFixture from "./fixtures/request.json";
import fixture from "./fixtures/requests.json";
import searchFixture from "./fixtures/search.json";
import settingsFixture from "./fixtures/settings.json";
import streamFixture from "./fixtures/stream.json";

// `crates/nvs-cli/tests/debug_api.rs` writes the fixtures from a real index. These assignments are
// the check that the types in `src/api.ts` describe what the service sends: `tsc` fails when they
// do not. A JSON import types `kind`, `view` and `theme` as `string`, so the one cast for each
// narrows it and checks the rest.
const page: RequestPage = fixture;
const detail: RequestDetail = {
  ...detailFixture,
  events: detailFixture.events.map((event) => ({ ...event, kind: event.kind as RequestDetail["events"][number]["kind"] })),
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
  theme: settingsFixture.theme as Settings["theme"],
  filters: settingsFixture.filters.map((filter) => ({ ...filter, view: filter.view as Settings["filters"][number]["view"] })),
};

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

  test("a pasted traceresponse value or trace id gives its key", () => {
    expect(traceKey("00-4BF92F3577B34DA6A3CE929D0E0E4736-00f067aa0ba902b7-01")).toEqual({
      trace: "4bf92f3577b34da6a3ce929d0e0e4736",
      span: "00f067aa0ba902b7",
    });
    expect(traceKey(" 4bf92f3577b34da6a3ce929d0e0e4736 ")).toEqual({ trace: "4bf92f3577b34da6a3ce929d0e0e4736" });
    expect(traceKey("/orders/7")).toBeNull();
  });
});
