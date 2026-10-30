// `nv deps`'s readers and its judgement, on text written here. Nothing in this file reaches a registry.

import { expect, test } from "bun:test";
import {
  actionPins,
  cargoInstallPins,
  cargoPins,
  cargoRange,
  compare,
  filePins,
  imagePins,
  judge,
  newestStable,
  newestTag,
  npmPins,
  parseImage,
  type Pin,
} from "../cmd/deps.ts";

const CARGO = `[workspace.dependencies]
# A plain comment.
# HOLD (revisit 2026-11-16): the parent still needs 0.2.
fallible-iterator = "0.2"
serde = { version = "1", features = ["derive"] } # trailing
rustls = { version = "0.23", features = [
    "ring",
] }
sha2-v11 = { package = "sha2", version = "0.11" }
local = { path = "crates/local" }
inherited = { workspace = true }
from-git = { git = "https://example.com/x.git" }

[patch.crates-io]
html5ever = { path = "vendor/html5ever" }

[target.'cfg(windows)'.dependencies]
windows-sys = "0.61"

[dependencies.tabled]
version = "0.5"
`;

test("a Cargo.toml gives every versioned registry row, its renamed package and the hold above it", () => {
  const pins = cargoPins(CARGO, "Cargo.toml");
  expect(pins.map((p) => [p.name, p.pinned, p.line])).toEqual([
    ["fallible-iterator", "0.2", 4],
    ["serde", "1", 5],
    ["rustls", "0.23", 6],
    ["sha2", "0.11", 9],
    ["windows-sys", "0.61", 18],
    ["tabled", "0.5", 20],
  ]);
  expect(pins[0]!.hold).toEqual({ revisit: "2026-11-16", reason: "the parent still needs 0.2." });
  expect(pins[1]!.hold).toBeUndefined();
});

test("a file under vendor/ or the synced looper kit gives no pin", () => {
  const PACKAGE = JSON.stringify({ devDependencies: { typescript: "^5.9.3" } }, null, 2);
  const text: Record<string, string> = {
    "Cargo.toml": CARGO,
    "vendor/html5ever/Cargo.toml": CARGO,
    "tools/looper/package.json": PACKAGE,
    "tools/nv/package.json": PACKAGE,
  };
  const read = (f: string) => text[f] ?? "";
  expect(filePins("vendor/html5ever/Cargo.toml", read)).toEqual([]);
  expect(filePins("tools/looper/package.json", read)).toEqual([]);
  expect(filePins("Cargo.toml", read)).toHaveLength(6);
  expect(filePins("tools/nv/package.json", read).map((p) => [p.name, p.pinned])).toEqual([["typescript", "^5.9.3"]]);
});

test("a Cargo requirement is a caret range unless it says otherwise", () => {
  expect(cargoRange("0.258")).toBe("^0.258");
  expect(cargoRange(">=1.2, <2")).toBe(">=1.2 <2");
  expect(cargoRange("=0.14.0")).toBe("=0.14.0");
  const crate = (pinned: string): Pin => ({ kind: "crate", name: "x", pinned, file: "Cargo.toml", line: 1 });
  expect(compare(crate("0.258"), "0.262.0")).toBe("behind");
  expect(compare(crate("49"), "49.0.3")).toBe("current");
  expect(compare(crate("1.2"), "1.9.0")).toBe("current");
  expect(compare(crate("1"), "1.1.8+spec-1.1.0")).toBe("current");
});

test("a package.json gives its dependencies, engines.bun and the holds object's entries", () => {
  const text = JSON.stringify(
    {
      engines: { bun: "1.4.3", node: ">=22" },
      holds: { typescript: "HOLD (revisit 2026-11-16): typescript-eslint allows <6.1.0" },
      dependencies: { a: "^1.0.0", b: "workspace:*" },
      devDependencies: { typescript: "~6.0.3" },
    },
    null,
    2,
  );
  const pins = npmPins(text, "package.json");
  expect(pins.map((p) => [p.name, p.pinned])).toEqual([
    ["a", "^1.0.0"],
    ["typescript", "~6.0.3"],
    ["bun", "1.4.3"],
  ]);
  expect(pins[1]!.line).toBe(text.split("\n").findIndex((l) => l.includes('"typescript": "~6.0.3"')) + 1);
  expect(pins[1]!.hold?.revisit).toBe("2026-11-16");
  const npm = (pinned: string): Pin => ({ kind: "npm", name: "x", pinned, file: "package.json", line: 1 });
  expect(compare(npm("~6.0.3"), "7.0.2")).toBe("behind");
  expect(compare(npm("1.4.3"), "1.4.3")).toBe("current");
});

test("a workflow gives its action SHAs with their comments, and its pinned cargo installs", () => {
  const text = [
    "      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7",
    "      - uses: dtolnay/rust-toolchain@2c76c034f9e32a5d6e2c104dc4df23a22476db04 # nightly branch",
    "    uses: ./.github/workflows/ci.yml",
    "        uses: github/codeql-action/init@abc # v4",
    "        run: cargo install cargo-geiger --version 0.14.0 --locked",
    "        run: cargo install cargo-fuzz --locked",
  ].join("\n");
  expect(actionPins(text, "ci.yml").map((p) => [p.name, p.note])).toEqual([
    ["actions/checkout", "v7"],
    ["dtolnay/rust-toolchain", "nightly branch"],
    ["github/codeql-action", "v4"],
  ]);
  expect(cargoInstallPins(text, "ci.yml").map((p) => [p.name, p.pinned, p.line])).toEqual([["cargo-geiger", "=0.14.0", 5]]);
});

test("a Dockerfile and a compose file give the images they pull, never a stage, scratch or a local build", () => {
  const dockerfile = [
    "ARG DEBIAN_BASE=debian:trixie-slim",
    "FROM golang:1.27.2-alpine AS build",
    "FROM scratch AS data",
    "FROM build AS again",
    "FROM ${DEBIAN_BASE} AS debian",
  ].join("\n");
  expect(imagePins(dockerfile, "Dockerfile").map((p) => p.pinned)).toEqual(["debian:trixie-slim", "golang:1.27.2-alpine"]);
  const compose = ["    image: nginx:1.29.3-alpine", "    image: nvs-bench-serve:local", "    image: ghcr.io/hatoo/oha@sha256:00"].join("\n");
  expect(imagePins(compose, "compose.yaml").map((p) => p.pinned)).toEqual(["nginx:1.29.3-alpine", "ghcr.io/hatoo/oha@sha256:00"]);
});

test("an image reference splits into its registry, repository, tag and digest", () => {
  expect(parseImage("php:8.5.11-fpm")).toEqual({ registry: "registry-1.docker.io", repo: "library/php", tag: "8.5.11-fpm", digest: undefined });
  expect(parseImage("oven/bun:1.3.5-slim").repo).toBe("oven/bun");
  expect(parseImage("mcr.microsoft.com/mssql/server:2022-CU20")).toMatchObject({ registry: "mcr.microsoft.com", repo: "mssql/server", tag: "2022-CU20" });
  expect(parseImage("ghcr.io/hatoo/oha@sha256:00")).toMatchObject({ registry: "ghcr.io", repo: "hatoo/oha", tag: undefined, digest: "sha256:00" });
});

test("the newest tag keeps the pinned tag's shape, and a tag with no number is not compared", () => {
  const tags = ["8.5.11-fpm", "8.5.12-fpm", "8.6.0RC1-fpm", "8.5.12-cli", "8.5.13-fpm-alpine", "latest"];
  expect(newestTag("8.5.11-fpm", tags)).toBe("8.5.12-fpm");
  expect(newestTag("1.29.3-alpine", ["1.29.3-alpine", "1.29.10-alpine", "1.30.0-alpine3.22"])).toBe("1.29.10-alpine");
  expect(newestTag("trixie-slim", tags)).toBeUndefined();
  expect(newestStable(["1.0.0", "1.2.0-rc.1", "1.1.9", "1.10.0+build"])).toBe("1.10.0+build");
});

test("a pin behind with a hold is held until the hold's date, then expired", () => {
  const pin: Pin = { kind: "crate", name: "x", pinned: "0.2", file: "Cargo.toml", line: 1, hold: { revisit: "2026-11-16", reason: "r" } };
  expect(judge(pin, "behind", "2026-11-16")).toBe("held");
  expect(judge(pin, "behind", "2026-11-17")).toBe("expired");
  expect(judge(pin, "current", "2027-01-01")).toBe("current");
  expect(judge({ ...pin, hold: undefined }, "behind", "2026-01-01")).toBe("behind");
});
