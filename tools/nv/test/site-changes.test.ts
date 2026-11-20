// `bun nv site --check changes` and the release step (`lib/changes.ts`): a moved feature owes an entry,
// an entry has its shape, and a release moves the entries into the notes, the feature changelogs and a
// new snapshot.

import { afterEach, describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { changesProblems, CHANGES, CORE_CHANGELOG, notesSections, releaseChanges, RELEASED, type ChangeEntry, type Changes, type Released } from "../lib/changes.ts";
import { proseProblems } from "../cmd/site.ts";
import { renderNotes } from "../cmd/release.ts";
import type { Entry } from "../proofs/roster.ts";
import { scratch, type Scratch } from "./scratch.ts";

const feature = (id: string, kind: Entry["kind"] = "member", group = "Core\\Str"): Entry => ({ id, kind, group, path: "", anchor: "", summary: "", help: "" });

const LENGTH = feature("Core\\Str::length");
const SLICE = feature("Core\\Str::slice");
const TRIM = feature("Core\\Str::trim");

/** The behaviour hashes now: `length` and `slice` are unchanged, `trim` is new. */
const NOW: Record<string, string> = { "Core\\Str::length": "aaaa", "Core\\Str::slice": "bbbb", "Core\\Str::trim": "cccc" };
const behaviour = (e: Entry) => NOW[e.id]!;
const roster = [LENGTH, SLICE, TRIM];

const snap = (features: Record<string, string>): Released => ({ since: "v0.0.4", features });
const changes = (entries: ChangeEntry[], unseen: Changes["unseen"] = {}): Changes => ({ entries, unseen });
const entry = (kind: ChangeEntry["kind"], features: string[], more: Partial<ChangeEntry> = {}): ChangeEntry => ({
  kind,
  features,
  text: `\`${features[0]}\` works in a new way.`,
  ...more,
});

const check = (released: Released, c: Changes) => changesProblems(roster, behaviour, released, c, proseProblems);

describe("site --check changes", () => {
  test("an added, a removed and a changed feature with no entry are each named", () => {
    const problems = check(snap({ "Core\\Str::length": "aaaa", "Core\\Str::slice": "0000", "Core\\Str::pad": "dddd" }), changes([]));
    expect(problems).toEqual([
      "removed: Core\\Str::pad is no longer on the roster, and no `removed` entry names it",
      "changed: Core\\Str::slice's tests moved since v0.0.4, and no `changed` or `fixed` entry or `unseen` record names it",
      "new: Core\\Str::trim is not in the snapshot of v0.0.4, and no `new` entry names it",
    ]);
  });

  test("an entry naming the feature clears it", () => {
    const problems = check(
      snap({ "Core\\Str::length": "aaaa", "Core\\Str::slice": "0000", "Core\\Str::pad": "dddd" }),
      changes([
        entry("new", ["Core\\Str::trim"]),
        entry("fixed", ["Core\\Str::slice"]),
        entry("removed", ["Core\\Str::pad"], { instead: "Use `Core\\Str::padStart`." }),
      ]),
    );
    expect(problems).toEqual([]);
  });

  test("an unseen record clears a feature only at the hash it names", () => {
    const base = snap({ "Core\\Str::length": "aaaa", "Core\\Str::slice": "0000", "Core\\Str::trim": "cccc" });
    expect(check(base, changes([], { "Core\\Str::slice": { hash: "bbbb", why: "a test was renamed" } }))).toEqual([]);
    const old = check(base, changes([], { "Core\\Str::slice": { hash: "9999", why: "a test was renamed" } }));
    expect(old).toContain("website/src/data/changes.json unseen: Core\\Str::slice names hash 9999, and its behaviour hash is now bbbb");
    expect(old.some((p) => p.startsWith("changed: Core\\Str::slice"))).toBe(true);
  });

  test("a removed entry and a breaking entry with no instead text are named", () => {
    const base = snap({ "Core\\Str::length": "aaaa", "Core\\Str::slice": "0000", "Core\\Str::trim": "cccc", "Core\\Str::pad": "dddd" });
    const problems = check(base, changes([entry("removed", ["Core\\Str::pad"]), entry("changed", ["Core\\Str::slice"], { breaking: true })]));
    expect(problems).toEqual([
      `${CHANGES} entry 1: a removed entry says what to write instead in \`instead\``,
      `${CHANGES} entry 2: a breaking entry says what to write instead in \`instead\``,
    ]);
  });

  test("an entry over three sentences or outside the prose bounds is named", () => {
    const base = snap({ "Core\\Str::length": "aaaa", "Core\\Str::slice": "0000", "Core\\Str::trim": "cccc" });
    const four = entry("changed", ["Core\\Str::slice"], { text: "One. Two. Three. Four." });
    const long = entry("changed", ["Core\\Str::slice"], {
      text: "This sentence is written to be far longer than any sentence a reader should meet, because it keeps adding words until it passes the bound of twenty-five words.",
    });
    const problems = check(base, changes([four, long]));
    expect(problems[0]).toBe(`${CHANGES} entry 1: text is 4 sentence(s), and an entry is one to three`);
    expect(problems.some((p) => p.startsWith(`${CHANGES} entry 2: text:`))).toBe(true);
    expect(problems).toHaveLength(2);
  });

  test("an entry naming a feature neither the roster nor the snapshot has is named", () => {
    const base = snap({ "Core\\Str::length": "aaaa", "Core\\Str::slice": "bbbb", "Core\\Str::trim": "cccc" });
    expect(check(base, changes([entry("new", ["Core\\Str::nothing"])]))).toEqual([
      `${CHANGES} entry 1: names Core\\Str::nothing, which neither the roster nor the snapshot has`,
    ]);
  });

  test("a highlight, faster or security entry may name a feature whose hash did not move", () => {
    const base = snap({ "Core\\Str::length": "aaaa", "Core\\Str::slice": "bbbb", "Core\\Str::trim": "cccc" });
    expect(check(base, changes([entry("changed", ["Core\\Str::length"])]))).toEqual([
      `${CHANGES} entry 1: names Core\\Str::length, whose behaviour has not moved since v0.0.4`,
    ]);
    const allowed = [
      entry("faster", ["Core\\Str::length"]),
      entry("changed", ["Core\\Str::length"], { highlight: true }),
      entry("fixed", ["Core\\Str::length"], { security: true }),
    ];
    expect(check(base, changes(allowed))).toEqual([]);
  });
});

describe("site --changes --release", () => {
  let s: Scratch | null = null;
  afterEach(() => {
    s?.cleanup();
    s = null;
  });

  const ENTRIES: ChangeEntry[] = [
    entry("fixed", ["Core\\Str::slice"], { text: "A fix." }),
    entry("new", ["Core\\Str::trim"], { text: "A new method." }),
    entry("faster", ["Core\\Str::length"], { text: "Faster." }),
    entry("changed", ["Core\\Str::slice"], { text: "A break.", breaking: true, instead: "Write it so." }),
    entry("removed", ["Core\\Str::pad"], { text: "A removal.", instead: "Write it so." }),
    entry("fixed", ["Core\\Str::slice"], { text: "A hole.", security: true }),
    entry("new", ["Core\\Str::trim"], { text: "The headline.", highlight: true }),
  ];

  test("the release notes start with each entry in its part, and the release empties the file", () => {
    s = scratch();
    s.put(CHANGES, JSON.stringify({ entries: ENTRIES, unseen: { "Core\\Str::length": { hash: "aaaa", why: "renamed" } } }));
    const notes = renderNotes("0.0.5", "v0.0.4", [], "", "2026-01-02", 100, ENTRIES);
    expect(notes.startsWith("## [0.0.5] - 2026-01-02\n\n### Security fixes\n\n- A hole.\n")).toBe(true);
    releaseChanges(s.root, "0.0.5", roster, behaviour);
    expect(JSON.parse(readFileSync(join(s.root, CHANGES), "utf8"))).toEqual({ entries: [], unseen: {} });
  });

  test("each entry goes to the section its kind and flags name, in the six sections' order, and a highlight entry leads its section", () => {
    expect(notesSections(ENTRIES).join("\n")).toBe(
      [
        "### Security fixes",
        "",
        "- A hole.",
        "",
        "### Breaking changes",
        "",
        "- A break. Write it so.",
        "- A removal. Write it so.",
        "",
        "### New",
        "",
        "- The headline.",
        "- A new method.",
        "",
        "### Faster",
        "",
        "- Faster.",
        "",
        "### Fixes",
        "",
        "- A fix.",
        "",
      ].join("\n"),
    );
  });

  test("the release adds each entry to the changelog of every feature it names", () => {
    s = scratch();
    s.put(CHANGES, JSON.stringify({ entries: [entry("new", ["Core\\Str::trim", "directive:app"], { text: "Two things." })], unseen: {} }));
    s.put(CORE_CHANGELOG, JSON.stringify({ "//": "a note", "Str.trim": [{ version: "0.0.3", change: "Before." }] }));
    releaseChanges(s.root, "0.0.5", [...roster, feature("directive:app", "directive", "")], behaviour);
    expect(JSON.parse(readFileSync(join(s.root, CORE_CHANGELOG), "utf8"))).toEqual({
      "//": "a note",
      "Str.trim": [
        { version: "0.0.3", change: "Before." },
        { version: "0.0.5", change: "Two things." },
      ],
      "directive:app": [{ version: "0.0.5", change: "Two things." }],
    });
  });

  test("the release writes a new snapshot named after its version", () => {
    s = scratch();
    s.put(CHANGES, JSON.stringify({ entries: [], unseen: {} }));
    releaseChanges(s.root, "0.0.5", roster, behaviour);
    expect(JSON.parse(readFileSync(join(s.root, RELEASED), "utf8"))).toEqual({ since: "v0.0.5", features: NOW });
  });
});
