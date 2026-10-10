// `bun nv services --up` over a fake `docker`: the goal is handed in, and every process the command
// would start is answered here, so no daemon is needed.

import { describe, expect, test } from "bun:test";
import { type DockerSeams, services } from "../cmd/services.ts";
import type { Goal } from "../lib/chain.ts";

const DOCKER = { compose: "tests/docker/compose.yml", services: ["db-one", "db-two"] };

function goal(docker?: typeof DOCKER): Goal {
  return { checks: [], files: [], env: docker ? { docker } : {} } as unknown as Goal;
}

/** A `docker` that answers `docker info` with `daemon` (null: it fails), and every other call with exit 0. */
function fake(daemon: string | null): { argv: string[][]; detached: string[][]; seams: Partial<DockerSeams> } {
  const argv: string[][] = [];
  const detached: string[][] = [];
  return {
    argv,
    detached,
    seams: {
      which: () => "docker",
      run: async (a) => {
        argv.push(a);
        if (a[1] === "info") return daemon === null ? { code: 1, stdout: "", stderr: "error during connect" } : { code: 0, stdout: `${daemon}\n`, stderr: "" };
        return { code: 0, stdout: "", stderr: "" };
      },
      detach: (_exe, args) => void detached.push(args),
      sleep: async () => {},
      waitMs: 0,
    },
  };
}

async function servicesOver(docker: typeof DOCKER | undefined, daemon: string | null, args = ["--up"]) {
  const f = fake(daemon);
  const lines: string[] = [];
  const named: (string | undefined)[] = [];
  const code = await services(args, {
    plan: (slug) => {
      named.push(slug);
      return { slug: slug ?? "demo", goal: goal(docker) };
    },
    docker: f.seams,
    say: (l) => lines.push(l),
  });
  return { code, lines, named, ...f };
}

describe("bun nv services", () => {
  test("nv services --up brings up the compose services the goal's env names", async () => {
    const got = await servicesOver(DOCKER, "27.1.0", ["--up", "--goal", "demo"]);
    expect(got.named).toEqual(["demo"]);
    expect(got.code).toBe(0);
    expect(got.argv).toEqual([
      ["docker", "info", "--format", "{{.ServerVersion}}"],
      ["docker", "compose", "-f", DOCKER.compose, "up", "-d", "--wait", "db-one", "db-two"],
    ]);
    expect(got.lines.at(-1)).toBe("GREEN db-one, db-two up for demo");
  });

  test("nv services --up does nothing and exits 0 for a goal with no docker env", async () => {
    const got = await servicesOver(undefined, null);
    expect(got.code).toBe(0);
    expect(got.argv).toEqual([]);
    expect(got.lines).toEqual(["goal demo names no docker services -- nothing to bring up"]);
  });

  test("nv services --up exits 1 when the docker daemon cannot be reached", async () => {
    const got = await servicesOver(DOCKER, null);
    expect(got.code).toBe(1);
    expect(got.detached).toEqual([["desktop", "restart", "--timeout", "0"]]);
    expect(got.argv.some((a) => a[1] === "compose")).toBe(false);
    expect(got.lines.at(-1)).toStartWith("RED this goal needs a reachable Docker daemon");
  });

  test("nv services without --up is a usage error", async () => {
    expect((await servicesOver(DOCKER, "27.1.0", [])).code).toBe(2);
  });
});
