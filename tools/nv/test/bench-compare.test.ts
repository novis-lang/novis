import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import {
  type Answer,
  agreement,
  buildRecord,
  buildScenarios,
  CASES,
  COMPARE_DIR,
  CONFIG_CHANGES,
  checkResult,
  comparable,
  configResult,
  coresResult,
  crashResult,
  csrfResult,
  DEPLOY_EXT,
  DEPLOY_PARTS,
  DEPLOY_WAYS,
  deployEntry,
  deployParts,
  deployResult,
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
  RUNAWAY_LIMITS,
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
  UPGRADE_WAYS,
  upgradeCommand,
  upgradeResult,
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
    expect(stackEnv(go, ctx)).toEqual({ TREE: "cases", CASE: "hello", BACKEND_CPUS: "4", BACKEND_MEMORY: "0", NGINX_CONF: "http.conf", NGINX_PORT: String(go.port) });
    expect(stackEnv(go, { ...ctx, memory: "512m" }).BACKEND_MEMORY).toBe("512m");
    const nvs = PEERS.find((p) => p.id === "nvs")!;
    expect(stackEnv(nvs, ctx).NVS_BUILD_COMMIT).toBe("abc1234");
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
        // A row that serves a case serves every peer's program, fixed or not. A generated one is written by the row.
        const file = files[peer.id]!;
        const there = existsSync(join(COMPARE_DIR, s.tree, s.case, file)) || s.generate?.()[file] !== undefined;
        expect(there).toBe(s.tree === "cases" || s.fixed?.[peer.id] === undefined);
      }
    }
  });

  test("the rows run in the goal's order, and each id once", () => {
    expect(SCENARIOS.map((s) => s.id)).toEqual(["escape", "taint", "secret", "live-edit", "config", "deploy", "cores", "crash", "loop", "memory", "csrf", "headers", "waiting", "parallel", "upgrade"]);
  });

  test("every row either probes a stack or times a case, never both, and a timed row names a case", () => {
    for (const s of SCENARIOS) {
      expect(s.probe === undefined).toBe(s.timed !== undefined);
      if (s.timed !== undefined) expect(CASES.some((c) => c.id === s.case)).toBe(true);
    }
  });

  const timed = (rps: number, over: Partial<PeerResult> = {}): PeerResult => ({
    requests_per_sec: rps,
    median_requests_per_sec: rps,
    p50_ms: 1,
    p99_ms: 2,
    errors: 0,
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
    expect(waitingResult(timed(100), 1000, undefined)).toEqual({ result: "built-in", evidence: { in_flight: 1000, kib_per_request: 10.2, failed_requests: 0 } });
    expect(waitingResult(timed(100), 1000, maxChildren).result).toBe("one-setting");
    expect(waitingResult(timed(100, { status_codes: { "200": 290, "503": 10 } }), 1000, undefined).evidence.failed_requests).toBe(10);
    expect(waitingResult(timed(100, { errors: 1 }), 1000, maxChildren).result).toBe("no");
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

  test("a deploy is built in with no command, one-setting naming its command, and no on a failure, a mixed reply or no new version", () => {
    const load = { requests: 900, failed: 0, failures: null, p99_ms: 3 };
    const clean = { replies: 40, failed: 0, mixed: 0, copy_seconds: 2.5, seconds: 1.2 };
    const restart = DEPLOY_WAYS.bun!;
    expect(deployResult(load, clean, undefined, false)).toEqual({
      result: "built-in",
      evidence: { requests: 940, failed_requests: 0, failures: null, mixed_replies: 0, seconds_to_copy: 2.5, seconds_to_new_answer: 1.2, command: null, restarts: 0, version_checked: true },
    });
    expect(deployResult(load, clean, restart, false)).toMatchObject({ result: "one-setting", setting: "a restart", evidence: { restarts: 1 } });
    expect(deployResult(load, clean, undefined, false, LIVE_EDIT_SETTING.php)).toMatchObject({ result: "on-by-default", setting: "opcache.validate_timestamps" });
    expect(DEPLOY_WAYS.php).toBeUndefined();
    expect(deployResult({ ...load, failed: 3, failures: "502 x3" }, clean, restart, false).result).toBe("no");
    expect(deployResult(load, { ...clean, failed: 1 }, undefined, false).evidence.failed_requests).toBe(1);
    expect(deployResult(load, { ...clean, mixed: 2 }, undefined, false).result).toBe("no");
    expect(deployResult(load, { ...clean, seconds: null }, undefined, false).result).toBe("no");
    expect(deployResult(load, clean, DEPLOY_WAYS.go!, true).evidence.version_checked).toBe(false);
    expect(mixedVersions("1121\n")).toBe(true);
    expect(mixedVersions("2222\n")).toBe(false);
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

  test("an upgrade is one-setting naming its step with no failed request, and no otherwise", () => {
    const load = { requests: 900, failed: 0, failures: null, p99_ms: 3 };
    const clean = { replies: 40, failed: 0, mixed: 0, seconds: 0.4 };
    expect(upgradeResult(load, clean, UPGRADE_WAYS.php!.way)).toEqual({
      result: "one-setting",
      setting: "php-fpm reload",
      evidence: { requests: 940, failed_requests: 0, failures: null, p99_ms: 3, seconds_to_answer: 0.4, command: "php-fpm reload", restarts: 0 },
    });
    expect(upgradeResult(load, clean, UPGRADE_WAYS.nvs!.way).evidence.restarts).toBe(1);
    expect(upgradeResult({ ...load, failed: 7, failures: "502 x7" }, clean, UPGRADE_WAYS.nvs!.way).result).toBe("no");
    expect(upgradeResult(load, { ...clean, seconds: null }, UPGRADE_WAYS.bun!.way).result).toBe("no");
    expect(Object.keys(UPGRADE_WAYS).sort()).toEqual(PEERS.map((p) => p.id).sort());
    expect(upgradeCommand("/usr/local/bin/nvs")[2]).toBe('cp "/usr/local/bin/nvs" "/usr/local/bin/nvs.new" && mv "/usr/local/bin/nvs.new" "/usr/local/bin/nvs"');
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

  test("a runaway is stopped when the backend answers it, the load saw no failure, and the backend still runs", () => {
    const load = { requests: 900, failed: 0, failures: null, p99_ms: 12.5 };
    const stopped = { status: 500, seconds: 1.2 };
    const limits = { name: "[limits]", default: false };
    expect(runawayResult(load, stopped, true, undefined).result).toBe("built-in");
    expect(runawayResult(load, { status: 200, seconds: 30.4 }, true, limits)).toEqual({
      result: "one-setting",
      setting: "[limits]",
      evidence: { requests: 900, failed_requests: 0, failures: null, p99_ms: 12.5, runaway_status: 200, runaway_seconds: 30.4, backend_running: true },
    });
    expect(runawayResult(load, stopped, true, { name: "memory_limit", default: true })).toMatchObject({ result: "on-by-default", setting: "memory_limit" });
    expect(runawayResult(load, { status: null, seconds: null }, true, undefined).result).toBe("no");
    expect(runawayResult(load, { status: 502, seconds: 3 }, true, undefined).result).toBe("no");
    expect(runawayResult({ ...load, failed: 1 }, stopped, true, limits).result).toBe("no");
    expect(runawayResult(load, stopped, false, undefined).result).toBe("no");
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
    expect(editedLines(shipped, RUNAWAY_LIMITS.nvs!)).toEqual(['memory = "16M"', 'cpu_time = "1s"']);
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

  test("a crash is survived when the next request is answered and the backend is still up", () => {
    expect(crashResult(500, 200, true).result).toBe("built-in");
    expect(crashResult(502, 200, true).evidence).toEqual({ crash_status: 502, next_status: 200, backend_running: true });
    expect(crashResult(500, 502, false).result).toBe("no");
    expect(crashResult(500, 200, false).result).toBe("no");
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
