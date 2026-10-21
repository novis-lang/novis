import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import {
  type Answer,
  type DeployLoad,
  agreement,
  buildRecord,
  buildScenarios,
  CASES,
  COMPARE_DIR,
  CONFIG_CHANGES,
  CORES_CASE,
  CORES_TURNS,
  checkResult,
  LOOP_DEFAULT,
  LOOP_IN_CODE,
  MEMORY_DEFAULT,
  comparable,
  configResult,
  coresResult,
  crashResult,
  csrfResult,
  DEPLOY_BIG_PARTS,
  DEPLOY_EXT,
  DEPLOY_PARTS,
  DEPLOY_RUNS,
  DEPLOY_SWITCH,
  DEPLOY_WAYS,
  deployEntry,
  deployParts,
  deployRelease,
  deployResult,
  failureCounts,
  medianIndex,
  medianOf,
  mergeDeploys,
  mergeRunaways,
  RUNAWAY_RUNS,
  ESCAPE_TERM,
  escapeResult,
  headersResult,
  htmlEntities,
  LIVE_EDIT_SETTING,
  liveEditResult,
  mixedVersions,
  PEERS,
  type PeerResult,
  phpChildren,
  PARALLEL_GRANT,
  PARALLEL_PARTS,
  parallelResult,
  rowLine,
  type RowResult,
  runawayResult,
  SCENARIOS,
  scenarioFolder,
  withScenarios,
  stackEnv,
  steadyResult,
  type TableEdit,
  tableEditCommand,

  waitingResult,
} from "../cmd/bench-compare.ts";

const ctx = { cpus: 4, concurrency: 64, tree: "cases", case: "hello", commit: "abc1234" };

describe("the peer table", () => {
  test("names Novis, PHP, Bun and Go, in that order", () => {
    expect(PEERS.map((p) => p.id)).toEqual(["nvs", "php", "bun", "go"]);
  });

  test("every peer has its own compose file and port, and an nginx config that exists", () => {
    expect(new Set(PEERS.map((p) => p.stack)).size).toBe(PEERS.length);
    expect(new Set(PEERS.map((p) => p.port)).size).toBe(PEERS.length);
    for (const p of PEERS) {
      expect(existsSync(join(COMPARE_DIR, p.stack))).toBe(true);
      expect(existsSync(join(COMPARE_DIR, "nginx", p.nginx))).toBe(true);
    }
  });

  test("only PHP-FPM is behind the FastCGI config", () => {
    expect(PEERS.filter((p) => p.nginx === "fastcgi.conf").map((p) => p.id)).toEqual(["php"]);
  });

  test("every nginx config terminates TLS 1.3 only", () => {
    for (const conf of new Set(PEERS.map((p) => p.nginx))) {
      const text = readFileSync(join(COMPARE_DIR, "nginx", conf), "utf8");
      expect(text).toContain("listen 443 ssl");
      expect(text).toMatch(/^\s*ssl_protocols TLSv1\.3;$/m);
    }
  });

  test("a stack's environment carries the case, the budget, the nginx config and the port", () => {
    const go = PEERS.find((p) => p.id === "go")!;
    expect(stackEnv(go, ctx)).toEqual({
      TREE: "cases",
      CASE: "hello",
      BACKEND_CPUS: "4",
      BACKEND_MEMORY: "0",
      APP_DIR: "/app",
      APP_MOUNT_MODE: "ro",
      NGINX_CONF: "http.conf",
      NGINX_PORT: String(go.port),
    });
    expect(stackEnv(go, { ...ctx, memory: "512m" }).BACKEND_MEMORY).toBe("512m");
    const nvs = PEERS.find((p) => p.id === "nvs")!;
    expect(stackEnv(nvs, ctx).NVS_BUILD_COMMIT).toBe("abc1234");
  });

  test("a row may serve a link read-write, name its part count and give a peer its own nginx config", () => {
    const php = PEERS.find((p) => p.id === "php")!;
    const env = stackEnv(php, { ...ctx, appDir: "/app/current", mount: "rw", parts: 2000, nginx: "fastcgi-switch.conf" });
    expect(env).toMatchObject({ APP_DIR: "/app/current", APP_MOUNT_MODE: "rw", DEPLOY_PARTS: "2000", NGINX_CONF: "fastcgi-switch.conf" });
    expect(stackEnv(php, ctx).DEPLOY_PARTS).toBeUndefined();
  });

  test("the switch config differs from the FastCGI one only in the root and the two lines that name the script", () => {
    const plain = readFileSync(join(COMPARE_DIR, "nginx", "fastcgi.conf"), "utf8").split("\n");
    const lines = readFileSync(join(COMPARE_DIR, "nginx", "fastcgi-switch.conf"), "utf8").split("\n");
    const body = (all: string[]) => all.slice(all.findIndex((l) => l.startsWith("worker_processes")));
    const changed = body(lines).filter((l) => !body(plain).includes(l)).map((l) => l.trim()).filter((l) => l !== "");
    expect(changed).toEqual(["root /app/current;", "fastcgi_param SCRIPT_FILENAME $realpath_root/index.php;", "fastcgi_param DOCUMENT_ROOT $realpath_root;"]);
  });

  test("the settings follow the budget and the concurrency", () => {
    const of = (id: string) => PEERS.find((p) => p.id === id)!.settings(ctx);
    expect(of("go").GOMAXPROCS).toBe(4);
    expect(of("bun").processes).toBe(4);
    expect(of("php")["pm.max_children"]).toBe(128);
    expect(phpChildren(8)).toBe(96);
    expect(phpChildren(1000)).toBe(2000);
  });

  test("every case has a program for every peer", () => {
    const files = ["main.nvs", "index.php", "server.ts", "main.go"];
    for (const c of CASES) for (const f of files) expect(existsSync(join(COMPARE_DIR, "cases", c.id, f))).toBe(true);
  });
});

describe("the agreement mapping", () => {
  test("maps the spellings of a single and a double quote, and nothing else", () => {
    expect(htmlEntities("&#039; &#x27; &apos; &#39; &#X27;")).toBe("&#39; &#39; &#39; &#39; &#39;");
    expect(htmlEntities("&#34; &quot;")).toBe("&quot; &quot;");
    expect(htmlEntities("&lt;&gt;&amp; &#60; '")).toBe("&lt;&gt;&amp; &#60; '");
  });

  const html = { ...CASES[0]!, html: true };
  const text = { ...CASES[0]!, html: false };
  const answer = (body: string, contentType = "text/html; charset=UTF-8", status = 200): Answer => ({ status, contentType, body });

  test("compares the Content-Type without case or spaces", () => {
    expect(comparable(text, answer("x", "text/html;charset=utf-8"))).toEqual(comparable(text, answer("x", "text/html; charset=UTF-8")));
  });

  test("maps a body only in an HTML case", () => {
    expect(comparable(html, answer("&#039;")).body).toBe("&#39;");
    expect(comparable(text, answer("&#039;")).body).toBe("&#039;");
  });

  test("peers that differ only in the mapped spellings agree", () => {
    const answers = new Map([
      ["nvs", [answer("<h1>&lt;&#39;&quot;</h1>")]],
      ["go", [answer("<h1>&lt;&#39;&#34;</h1>")]],
      ["php", [answer("<h1>&lt;&#039;&quot;</h1>")]],
    ]);
    expect(agreement(html, answers)).toBeNull();
  });

  test("names the first peer that differs, and what differs", () => {
    const answers = new Map([
      ["nvs", [answer("hello")]],
      ["php", [answer("hello")]],
      ["bun", [answer("hello", "text/plain")]],
      ["go", [answer("hello", undefined, 404)]],
    ]);
    const bad = agreement(text, answers)!;
    expect(bad.peer).toBe("bun");
    expect(bad.input).toBe(0);
    expect(bad.why).toContain("Content-Type");
    answers.delete("bun");
    expect(agreement(text, answers)!.why).toContain("status 404");
  });

  test("a peer missing an answer disagrees", () => {
    const answers = new Map([
      ["nvs", [answer("hello")]],
      ["php", []],
    ]);
    expect(agreement(text, answers)).toEqual({ peer: "php", input: 0, why: "no answer" });
  });
});

describe("the feature scenarios", () => {
  test("every row has a program for every peer it does not give a fixed result", () => {
    const files: Record<string, string> = { nvs: "main.nvs", php: "index.php", bun: "server.ts", go: "main.go" };
    for (const s of SCENARIOS) {
      for (const peer of PEERS) {
        // A row that serves a case serves every peer's program, fixed or not. A generated one is written by
        // the row, at the top of its folder or in the release a link serves.
        const file = files[peer.id]!;
        const generated = Object.keys(s.generate?.() ?? {});
        const there = existsSync(join(COMPARE_DIR, s.tree, s.case, file)) || generated.some((path) => path === file || path === `releases/1/${file}`);
        expect(there).toBe(s.tree === "cases" || s.fixed?.[peer.id] === undefined);
      }
    }
  });

  test("the rows run in the goal's order, and each id once", () => {
    expect(SCENARIOS.map((s) => s.id)).toEqual(["escape", "taint", "secret", "live-edit", "config", "deploy", "deploy-big", "deploy-switch", "cores", "crash", "loop", "memory", "csrf", "headers", "waiting", "parallel"]);
  });

  test("every row either probes a stack or times a case, never both, and a timed row names a case or brings its own", () => {
    for (const s of SCENARIOS) {
      expect(s.probe === undefined).toBe(s.timed !== undefined);
      if (s.timed !== undefined) expect(s.timedCase !== undefined || CASES.some((c) => c.id === s.case)).toBe(true);
    }
  });

  test("the cores row times one part of the parallel program, which no peer splits", () => {
    const cores = SCENARIOS.find((s) => s.id === "cores")!;
    expect(cores.timedCase).toBe(CORES_CASE);
    expect(scenarioFolder(cores)).toBe("benches/compare/scenarios/parallel");
    expect(CORES_CASE.timed.path).toBe(`/?parts=1&turns=${CORES_TURNS}`);
  });

  const timed = (rps: number, over: Partial<PeerResult> = {}): PeerResult => ({
    requests_per_sec: rps,
    median_requests_per_sec: rps,
    p50_ms: 1,
    p99_ms: 2,
    errors: 0,
    error_kinds: {},
    status_codes: { "200": 300 },
    memory_idle_bytes: 10 * 1048576,
    memory_peak_bytes: 20 * 1048576,
    settings: {},
    ...over,
  });

  test("cores is built in from twice the throughput at four cpus, your-code where a launcher does it, and no below that", () => {
    expect(coresResult(timed(1000), timed(3500), undefined)).toEqual({
      result: "built-in",
      evidence: { requests_per_sec_1_cpu: 1000, requests_per_sec_4_cpus: 3500, scaling: 3.5, errors: 0, code: null },
    });
    expect(coresResult(timed(1000), timed(2000), "a launcher").result).toBe("your-code");
    expect(coresResult(timed(1000), timed(1100), undefined).result).toBe("no");
    expect(coresResult(timed(1000), timed(1100), "a launcher").result).toBe("no");
  });

  test("waiting fails on any request not answered 200, and its memory is per request in flight", () => {
    const maxChildren = { name: "pm.max_children", default: false };
    expect(waitingResult(timed(100), 1000, undefined)).toEqual({
      result: "built-in",
      evidence: { in_flight: 1000, kib_per_request: 10.2, failed_requests: 0, failures: null },
    });
    expect(waitingResult(timed(100), 1000, maxChildren).result).toBe("one-setting");
    expect(waitingResult(timed(100, { status_codes: { "200": 290, "503": 10 } }), 1000, undefined).evidence.failed_requests).toBe(10);
    const lost = waitingResult(timed(100, { errors: 1, error_kinds: { "connection closed": 1 }, status_codes: { "200": 298, "502": 2 } }), 1000, maxChildren);
    expect(lost.result).toBe("no");
    expect(lost.evidence.failures).toBe("502 x2; connection closed x1");
    expect(waitingResult(timed(100, { memory_peak_bytes: null }), 1000, undefined).evidence.kib_per_request).toBeNull();
  });

  test("a fixed result says why in its evidence, and a documentation link is a URL", () => {
    for (const s of SCENARIOS) {
      for (const r of Object.values(s.fixed ?? {})) {
        expect(Object.keys(r.evidence).length).toBeGreaterThan(0);
        if (r.evidence.docs !== undefined) expect(String(r.evidence.docs)).toStartWith("https://");
      }
    }
  });

  test("an edit that shows names the setting only when the peer starts with it on, and one that never shows names a setting it starts without", () => {
    const hot = LIVE_EDIT_SETTING.bun!;
    const timestamps = LIVE_EDIT_SETTING.php!;
    expect(liveEditResult(1.5, undefined)).toEqual({ result: "built-in", evidence: { seconds_to_new_answer: 1.5, restarts: 0 } });
    expect(liveEditResult(1.5, hot).result).toBe("built-in");
    expect(liveEditResult(1.5, timestamps)).toMatchObject({ result: "on-by-default", setting: "opcache.validate_timestamps" });
    expect(liveEditResult(null, hot)).toEqual({ result: "one-setting", setting: "bun --hot", evidence: { seconds_to_new_answer: null, restarts: 0 } });
    expect(liveEditResult(null, timestamps).result).toBe("no");
    expect(liveEditResult(null, undefined).result).toBe("no");
  });

  test("the deploy tree has 200 parts per program, and Novis's entry requires each one by its literal path", () => {
    for (const ext of Object.values(DEPLOY_EXT)) expect(Object.keys(deployParts("2", ext)).length).toBe(DEPLOY_PARTS);
    expect(deployParts("2", "nvs")["parts/p199.nvs"]).toBe('<?nvs\necho "2";\n');
    const generated = SCENARIOS.find((s) => s.id === "deploy")!.generate!();
    expect(Object.keys(generated).length).toBe(3 * DEPLOY_PARTS + 1);
    const requires = deployEntry().split("\n").filter((l) => l.startsWith("require "));
    expect(requires.length).toBe(DEPLOY_PARTS);
    expect(requires[0]).toBe("require 'parts/p000.nvs';");
  });

  test("the big deploy has 2000 parts named with four digits, and Novis's entry requires each one", () => {
    const parts = deployParts("1", "php", DEPLOY_BIG_PARTS);
    expect(Object.keys(parts).length).toBe(2000);
    expect(Object.keys(parts)[0]).toBe("parts/p0000.php");
    expect(parts["parts/p1999.php"]).toBe('<?php echo "1";\n');
    const requires = deployEntry(DEPLOY_BIG_PARTS).split("\n").filter((l) => l.startsWith("require "));
    expect(requires.length).toBe(2000);
    expect(requires[1999]).toBe("require 'parts/p1999.nvs';");
    const big = SCENARIOS.find((s) => s.id === "deploy-big")!;
    expect(big.parts).toBe(2000);
    expect(Object.keys(big.generate!()).length).toBe(3 * 2000 + 1);
  });

  test("the switch row generates release 1 under releases/1/, serves the link read-write, and gives PHP its realpath config", () => {
    const sw = SCENARIOS.find((s) => s.id === "deploy-switch")!;
    expect(sw).toMatchObject({ appDir: "/app/current", mount: "rw", nginx: { php: "fastcgi-switch.conf" } });
    const files = sw.generate!();
    for (const entry of ["main.nvs", "index.php", "server.ts"]) expect(files[`releases/1/${entry}`]).toBeDefined();
    expect(files["releases/1/parts/p000.ts"]).toBe('export const version = "1";\n');
    expect(Object.keys(files).every((path) => path.startsWith("releases/1/"))).toBe(true);
    const release = deployRelease("2", "php", DEPLOY_PARTS, "releases/2/");
    expect(Object.keys(release).length).toBe(DEPLOY_PARTS + 1);
    expect(release["releases/2/index.php"]).toBe(readFileSync(join(COMPARE_DIR, "scenarios", "deploy", "index.php"), "utf8"));
    expect(DEPLOY_SWITCH).toBe("ln -s releases/2 /app/next && mv -T /app/next /app/current");
  });

  test("every row that runs more than once merges its runs", () => {
    for (const s of SCENARIOS) if ((s.runs ?? 1) > 1) expect(s.merge).toBeDefined();
    expect(SCENARIOS.filter((s) => s.id.startsWith("deploy")).every((s) => s.runs === DEPLOY_RUNS)).toBe(true);
    expect(SCENARIOS.filter((s) => s.id === "loop" || s.id === "memory").every((s) => s.runs === RUNAWAY_RUNS)).toBe(true);
  });

  const deployLoad: DeployLoad = { requests: 940, failed: 0, failures: {}, mixed: 0, complete: 940, seconds: 1.2 };

  test("a deploy is built in with no command, one-setting naming its command, and no on a failure, a mixed reply or no new version", () => {
    const restart = DEPLOY_WAYS.bun!;
    expect(deployResult(deployLoad, 2.5, undefined, false)).toEqual({
      result: "built-in",
      evidence: {
        requests: 940,
        failed_requests: 0,
        failures: null,
        mixed_replies: 0,
        complete_replies: 940,
        seconds_to_copy: 2.5,
        seconds_to_new_answer: 1.2,
        command: null,
        restarts: 0,
        version_checked: true,
        runs: 1,
      },
    });
    expect(deployResult(deployLoad, 2.5, restart, false)).toMatchObject({ result: "one-setting", setting: "a restart", evidence: { restarts: 1 } });
    expect(deployResult(deployLoad, 2.5, undefined, false, LIVE_EDIT_SETTING.php)).toMatchObject({ result: "on-by-default", setting: "opcache.validate_timestamps" });
    expect(DEPLOY_WAYS.php).toBeUndefined();
    const failed = deployResult({ ...deployLoad, failed: 3, failures: { "502": 2, ECONNRESET: 1 } }, 2.5, restart, false);
    expect(failed).toMatchObject({ result: "no", evidence: { failed_requests: 3, failures: "502 x2; ECONNRESET x1" } });
    expect(deployResult({ ...deployLoad, mixed: 2 }, 2.5, undefined, false).result).toBe("no");
    expect(deployResult({ ...deployLoad, seconds: null }, 2.5, undefined, false).result).toBe("no");
    expect(deployResult(deployLoad, 2.5, DEPLOY_WAYS.go!, true).evidence.version_checked).toBe(false);
  });

  test("a mixed reply holds more than one version, of any number", () => {
    expect(mixedVersions("1121\n")).toBe(true);
    expect(mixedVersions("2222\n")).toBe(false);
    expect(mixedVersions("3333\n")).toBe(false);
    expect(mixedVersions("2233\n")).toBe(true);
    expect(mixedVersions("1233\n")).toBe(true);
    expect(mixedVersions("\n")).toBe(false);
  });

  test("deploy runs merge into sums, the slowest seconds, and a verdict on the sums", () => {
    const run = (over: Partial<typeof deployLoad>, copy = 2.1) => deployResult({ ...deployLoad, ...over }, copy, DEPLOY_WAYS.bun, false);
    const merged = mergeDeploys("bun", [run({ failures: {} }), run({ failed: 2, failures: { "502": 2 }, seconds: 3.4 }, 2.4), run({ failed: 1, failures: { "502": 1 } })]);
    expect(merged).toEqual({
      result: "no",
      evidence: {
        requests: 2820,
        failed_requests: 3,
        failures: "502 x3",
        mixed_replies: 0,
        complete_replies: 2820,
        seconds_to_copy: 2.4,
        seconds_to_new_answer: 3.4,
        command: "a restart",
        restarts: 3,
        version_checked: true,
        runs: 3,
      },
    });
    const clean = (over: Partial<typeof deployLoad>) => deployResult({ ...deployLoad, ...over }, 2.1, undefined, false, LIVE_EDIT_SETTING.php);
    expect(mergeDeploys("php", [clean({}), clean({}), clean({})])).toMatchObject({ result: "on-by-default", setting: "opcache.validate_timestamps", evidence: { mixed_replies: 0, runs: 3 } });
    expect(mergeDeploys("php", [clean({}), clean({ mixed: 77 }), clean({})])).toMatchObject({ result: "no", evidence: { mixed_replies: 77 } });
    expect(mergeDeploys("nvs", [clean({}), clean({ seconds: null }), clean({})]).evidence.seconds_to_new_answer).toBeNull();
    expect(failureCounts("502 x2; Connection reset x1")).toEqual(new Map([["502", 2], ["Connection reset", 1]]));
  });

  test("runaway runs merge into the median run's load, summed failures and restarts, and no when any run is no", () => {
    const up = { running: true, restarts: 0 };
    const limits = { name: "[limits] cpu_time", default: true };
    const run = (requests: number, failed = 0, backend = up) =>
      runawayResult({ requests: 1000, failed: 0, failures: null, p99_ms: 10 }, { requests, failed, failures: failed > 0 ? `502 x${failed}` : null, p99_ms: requests / 100 }, { status: 500, seconds: requests / 1000 }, backend, limits);
    const merged = mergeRunaways("nvs", [run(900), run(500), run(700)]);
    expect(merged).toEqual({
      result: "on-by-default",
      setting: "[limits] cpu_time",
      evidence: {
        baseline_requests: 1000,
        requests: 700,
        failed_requests: 0,
        failures: null,
        p99_ms: 7,
        runaway_status: 500,
        runaway_seconds: 0.7,
        backend_running: true,
        backend_restarts: 0,
        runs: 3,
        code: null,
      },
    });
    const bad = mergeRunaways("bun", [run(900), run(500, 4, { running: true, restarts: 1 }), run(700)]);
    expect(bad).toMatchObject({ result: "no", evidence: { failed_requests: 4, failures: "502 x4", backend_restarts: 1 } });
  });

  test("a config change is built in without a command, one-setting after a reload, and no otherwise", () => {
    expect(configResult(2.8, null, undefined).result).toBe("built-in");
    expect(configResult(null, 0.5, "php-fpm reload")).toEqual({
      result: "one-setting",
      setting: "php-fpm reload",
      evidence: { seconds_without_command: null, seconds_after_command: 0.5, command: "php-fpm reload", restarts: 0 },
    });
    expect(configResult(null, null, "php-fpm reload").result).toBe("no");
    expect(configResult(null, null, undefined).result).toBe("no");
  });


  test("every peer the config row runs has a change, and every other peer a fixed result", () => {
    const config = SCENARIOS.find((s) => s.id === "config")!;
    for (const peer of PEERS) expect(CONFIG_CHANGES[peer.id] !== undefined).toBe(config.fixed?.[peer.id] === undefined);
  });

  test("the cross-site POST is rejected when its reply does not say the state changed", () => {
    const changed = { status: 200, headers: {}, body: "changed\n" };
    expect(csrfResult(changed, { status: 403, headers: {}, body: "" })).toEqual({ result: "built-in", evidence: { cross_site_status: 403, rejected: true } });
    expect(csrfResult(changed, changed).result).toBe("your-code");
    expect(() => csrfResult({ status: 404, headers: {}, body: "" }, changed)).toThrow("same-site");
  });

  test("a check is built in only when it refuses with the row's own error", () => {
    expect(checkResult(1, "error[E0401]: expected `string`", "E0401")).toEqual({ result: "built-in", evidence: { check_exit: 1, error: "E0401" } });
    expect(checkResult(0, "No errors", "E0401").result).toBe("no");
    expect(checkResult(1, "error[E0001]: syntax", "E0401").result).toBe("no");
  });

  test("a runaway is stopped when the backend answers it, the load saw no failure, and the backend never stopped", () => {
    const baseline = { requests: 1000, failed: 0, failures: null, p99_ms: 10 };
    const load = { requests: 900, failed: 0, failures: null, p99_ms: 12.5 };
    const stopped = { status: 500, seconds: 1.2 };
    const up = { running: true, restarts: 0 };
    const limits = { name: "[limits] cpu_time", default: true };
    expect(runawayResult(baseline, load, stopped, up, undefined).result).toBe("built-in");
    expect(runawayResult(baseline, load, { status: 200, seconds: 30.4 }, up, limits)).toEqual({
      result: "on-by-default",
      setting: "[limits] cpu_time",
      evidence: {
        baseline_requests: 1000,
        requests: 900,
        failed_requests: 0,
        failures: null,
        p99_ms: 12.5,
        runaway_status: 200,
        runaway_seconds: 30.4,
        backend_running: true,
        backend_restarts: 0,
        runs: 1,
        code: null,
      },
    });
    expect(runawayResult(baseline, load, { status: null, seconds: null }, up, undefined).result).toBe("no");
    expect(runawayResult(baseline, load, { status: null, seconds: null }, up, undefined, "a Worker")).toMatchObject({ result: "your-code", evidence: { code: "a Worker" } });
    expect(runawayResult(baseline, load, { status: 502, seconds: 3 }, up, undefined).result).toBe("no");
    expect(runawayResult(baseline, { ...load, failed: 1 }, stopped, up, limits).result).toBe("no");
    expect(runawayResult(baseline, load, stopped, { running: false, restarts: 0 }, undefined).result).toBe("no");
    expect(runawayResult(baseline, load, stopped, { running: true, restarts: 1 }, undefined).result).toBe("no");
  });

  test("the runaway rows run every peer at its defaults: Novis and PHP name the limit they start with", () => {
    expect(LOOP_DEFAULT).toEqual({ nvs: { name: "[limits] cpu_time", default: true }, php: { name: "max_execution_time", default: true } });
    expect(MEMORY_DEFAULT).toEqual({ nvs: { name: "[limits] memory", default: true }, php: { name: "memory_limit", default: true } });
    expect(Object.keys(LOOP_IN_CODE)).toEqual(["bun"]);
    const shipped = readFileSync(join(import.meta.dir, "..", "..", "..", "crates", "nvs-config", "src", "default.toml"), "utf8");
    expect(shipped).toContain('#memory = "128M"                      # default');
    expect(shipped).toContain('#cpu_time = "30s"                     # default: 30s for server work');
  });

  test("steady load fails on every status but 200 and every error but the run's own deadline", () => {
    expect(steadyResult({ statusCodeDistribution: { "200": 90 }, errorDistribution: { "aborted due to deadline": 8 }, metrics: { latency_ms: { p99: 3.14 } } })).toEqual({
      requests: 90,
      failed: 0,
      failures: null,
      p99_ms: 3.1,
    });
    expect(steadyResult({ statusCodeDistribution: { "200": 90, "503": 4 }, errorDistribution: { timeout: 2 } })).toEqual({
      requests: 94,
      failed: 6,
      failures: "503 x4; timeout x2",
      p99_ms: null,
    });
  });

  // What the `sed` range `/^\[table\]$/,/^\[/` of `tableEditCommand` changes, so the shipped file is
  // checked without a `sed` on the host: from the table's header to the next header, each `#key = `
  // line becomes `key = value`.
  const editedLines = (shipped: string, edit: TableEdit): string[] => {
    const lines = shipped.split("\n");
    const start = lines.indexOf(`[${edit.table}]`);
    if (start < 0) return [];
    const after = lines.slice(start + 1).findIndex((line) => line.startsWith("["));
    const end = after < 0 ? lines.length : start + 1 + after;
    return lines
      .slice(start, end + 1)
      .flatMap((line) => Object.entries(edit.set).filter(([key]) => line.startsWith(`#${key} = `)).map(([key, value]) => `${key} = ${value}`));
  };

  test("a table edit uncomments each key inside its own table and nowhere else", () => {
    const shipped = readFileSync(join(import.meta.dir, "..", "..", "..", "crates", "nvs-config", "src", "default.toml"), "utf8");
    expect(editedLines(shipped, PARALLEL_GRANT.nvs!)).toEqual(["spawn = true"]);
    expect(tableEditCommand(PARALLEL_GRANT.nvs!)[2]).toBe("/^\\[capabilities\\.script\\]$/,/^\\[/{s|^#spawn = .*|spawn = true|}");
  });

  test("parallel is built in from twice the speed of parts run one after another, and names the setting it needs", () => {
    expect(parallelResult(1, 1.2, "goroutines", undefined)).toEqual({
      result: "built-in",
      evidence: { seconds_1_part: 1, seconds_4_parts: 1.2, speedup: 3.33, way: "goroutines" },
    });
    expect(parallelResult(1, 1.5, "spawn", { name: "[capabilities.script] spawn", default: false })).toMatchObject({
      result: "one-setting",
      setting: "[capabilities.script] spawn",
    });
    expect(parallelResult(1, PARALLEL_PARTS, undefined, undefined).result).toBe("no");
    expect(parallelResult(1, 2.1, "Worker", undefined).result).toBe("no");
  });

  test("a crash is survived when the next request is answered and the backend never stopped", () => {
    const up = { running: true, restarts: 0 };
    expect(crashResult(500, 200, up).result).toBe("built-in");
    expect(crashResult(502, 200, up).evidence).toEqual({ crash_status: 502, next_status: 200, backend_running: true, backend_restarts: 0 });
    expect(crashResult(500, 502, { running: false, restarts: 0 }).result).toBe("no");
    expect(crashResult(500, 200, { running: false, restarts: 0 }).result).toBe("no");
    expect(crashResult(502, 200, { running: true, restarts: 1 }).result).toBe("no");
  });

  test("the headers pass only with all three, naming a setting the peer starts with, and the frame policy may come from either header", () => {
    const all = { "x-content-type-options": "nosniff", "content-security-policy": "frame-ancestors 'none'", "referrer-policy": "no-referrer" };
    const headers = { name: "[http.headers]", default: true };
    expect(headersResult(all, undefined).result).toBe("built-in");
    expect(headersResult(all, headers)).toMatchObject({ result: "on-by-default", setting: "[http.headers]" });
    expect(headersResult({ ...all, "content-security-policy": "default-src 'self'", "x-frame-options": "DENY" }, undefined).result).toBe("built-in");
    expect(headersResult({ "x-content-type-options": "nosniff" }, headers)).toEqual({
      result: "your-code",
      evidence: { nosniff: true, frame_policy: false, referrer_policy: false },
    });
    expect(headersResult({ ...all, "x-content-type-options": "sniff" }, undefined).result).toBe("your-code");
  });

  test("the escape is built in only when the template and the direct write both escape the term", () => {
    const raw = `<h1>Results for ${ESCAPE_TERM}</h1>`;
    const escaped = "<h1>Results for &lt;script&gt;alert(1)&lt;/script&gt;</h1>";
    expect(escapeResult(escaped, escaped)).toEqual({ result: "built-in", evidence: { raw_script_from_template: false, raw_script_from_direct_write: false } });
    expect(escapeResult(escaped, raw)).toEqual({ result: "your-code", evidence: { raw_script_from_template: false, raw_script_from_direct_write: true } });
    expect(escapeResult(raw, raw).result).toBe("your-code");
  });

  test("a row's line names each peer's result in the table's order, with a setting where there is one", () => {
    const r = (result: RowResult["result"], setting?: string): RowResult => (setting === undefined ? { result, evidence: {} } : { result, setting, evidence: {} });
    const byPeer = new Map([["go", r("no")], ["nvs", r("one-setting", "limits.memory")], ["php", r("built-in")], ["bun", r("your-code")]]);
    expect(rowLine("loop", byPeer)).toBe("loop: Novis one-setting (limits.memory), PHP built-in, Bun your-code, Go no");
  });
});

describe("the record shape", () => {
  const result = (rps: number): PeerResult => ({
    requests_per_sec: rps,
    median_requests_per_sec: rps,
    p50_ms: 1,
    p99_ms: 2,
    errors: 0,
    error_kinds: {},
    status_codes: { "200": 300 },
    memory_idle_bytes: 1024,
    memory_peak_bytes: 2048,
    settings: {},
  });
  const run = {
    date: "2026-10-09T12:00:00",
    commit: "abc1234",
    arm: "fair" as const,
    backendCpus: 1,
    requests: 300,
    reps: 1,
    host: { processor: "cpu", cpus: 16, docker: "28.0.0" },
    digests: { "benches/compare/nginx/http.conf": "0123456789abcdef" },
  };

  test("names the machine, the peers and the images, and orders each case's peers as the table does", () => {
    const results = new Map([["hello", new Map([["go", result(4)], ["nvs", result(1)], ["bun", result(3)], ["php", result(2)]])]]);
    const record = buildRecord(run, results);
    expect(Object.keys(record)).toEqual([
      "date",
      "commit",
      "arm",
      "backend_cpus",
      "nginx_cpus",
      "tls",
      "requests",
      "reps",
      "host",
      "peers",
      "images",
      "cases",
      "scenarios",
      "config_digests",
    ]);
    expect(record.tls).toBe("TLSv1.3");
    expect(record.host).toEqual(run.host);
    expect(Object.keys(record.peers)).toEqual(["nvs", "php", "bun", "go"]);
    expect(record.cases.hello!.concurrency).toBe(64);
    expect(Object.keys(record.cases.hello!.peers)).toEqual(["nvs", "php", "bun", "go"]);
    expect(record.cases.hello!.peers.go!.requests_per_sec).toBe(4);
  });

  test("a peer's reps become the median rep's throughput and latency, the median memory, and summed errors", () => {
    const rep = (rps: number, p99: number, peak: number | null, errors = 0): PeerResult => ({
      ...result(rps),
      p99_ms: p99,
      memory_peak_bytes: peak,
      errors,
      error_kinds: errors > 0 ? { reset: errors } : {},
      status_codes: { "200": 300 },
    });
    const m = medianOf([rep(900, 9, 30), rep(1200, 4, 10), rep(1000, 6, 50, 2), rep(800, 12, null), rep(1100, 5, 20)]);
    expect(m.requests_per_sec).toBe(1000);
    expect(m.median_requests_per_sec).toBe(1000);
    expect(m.p99_ms).toBe(6);
    expect(m.memory_peak_bytes).toBe(20);
    expect(m.errors).toBe(2);
    expect(m.error_kinds).toEqual({ reset: 2 });
    expect(m.status_codes).toEqual({ "200": 1500 });
    expect(medianOf([rep(5, 1, null)]).memory_peak_bytes).toBeNull();
    expect(medianIndex([3, 1, 2, 4])).toBe(2);
  });

  test("a case that was not run is absent, not empty", () => {
    expect(buildRecord(run, new Map()).cases).toEqual({});
  });

  test("survives a round trip through JSON", () => {
    const record = buildRecord(run, new Map([["hello", new Map([["nvs", result(1)]])]]));
    expect(JSON.parse(JSON.stringify(record))).toEqual(record);
  });

  const built: RowResult = { result: "built-in", evidence: { status: 500 } };
  const setting: RowResult = { result: "one-setting", setting: "pm.max_children", evidence: { status: 503 } };

  test("a case run with no scenarios recorded yet says so with null", () => {
    expect(buildRecord(run, new Map()).scenarios).toBeNull();
  });

  test("the rows keep SCENARIOS order and PEERS order, and each names the folder its programs are in", () => {
    const rows = buildScenarios("2026-10-10T08:00:00", "def5678", new Map([
      ["config", new Map([["php", setting], ["nvs", built]])],
      ["loop", new Map([["go", built], ["nvs", built]])],
    ]));
    expect(rows.date).toBe("2026-10-10T08:00:00");
    expect(rows.commit).toBe("def5678");
    expect(Object.keys(rows.rows)).toEqual(["config", "loop"]);
    expect(Object.keys(rows.rows.config!.peers)).toEqual(["nvs", "php"]);
    expect(rows.rows.config!.peers.php).toEqual(setting);
    expect(rows.rows.loop!.folder).toBe("benches/compare/scenarios/runaway");
    expect(rows.rows.config!.folder).toBe("benches/compare/cases/hello");
  });

  test("every row's folder is on disk", () => {
    for (const s of SCENARIOS) expect(existsSync(join(COMPARE_DIR, "..", "..", scenarioFolder(s)))).toBe(true);
  });

  test("the rows replace a record's scenarios and keep its keys in buildRecord's order", () => {
    const rows = buildScenarios("2026-10-10T08:00:00", "def5678", new Map([["crash", new Map([["nvs", built]])]]));
    const prior = JSON.parse(JSON.stringify(buildRecord(run, new Map([["hello", new Map([["nvs", result(1)]])]]))));
    const merged = withScenarios(prior, rows);
    expect(Object.keys(merged)).toEqual(Object.keys(prior));
    expect(merged.scenarios).toEqual(rows);
    expect(merged.cases).toEqual(prior.cases);
    expect(Object.keys(withScenarios({ ...prior, scenarios: undefined }, rows))).toEqual(Object.keys(prior));
  });

  test("the rows go into no record that has no cases", () => {
    const rows = buildScenarios("2026-10-10T08:00:00", "def5678", new Map());
    expect(() => withScenarios({ date: "x" }, rows)).toThrow("record them first");
  });
});
