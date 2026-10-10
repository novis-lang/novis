import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import {
  type Answer,
  agreement,
  buildRecord,
  CASES,
  COMPARE_DIR,
  checkResult,
  comparable,
  crashResult,
  csrfResult,
  ESCAPE_TERM,
  escapeResult,
  headersResult,
  htmlEntities,
  isolationResult,
  PEERS,
  type PeerResult,
  phpChildren,
  rowLine,
  type RowResult,
  SCENARIOS,
  stackEnv,
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
    expect(stackEnv(go, ctx)).toEqual({ TREE: "cases", CASE: "hello", BACKEND_CPUS: "4", NGINX_CONF: "http.conf", NGINX_PORT: String(go.port) });
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
        expect(existsSync(join(COMPARE_DIR, s.tree, s.case, files[peer.id]!))).toBe(s.fixed?.[peer.id] === undefined);
      }
    }
  });

  test("the rows run in the goal's order, and each id once", () => {
    expect(SCENARIOS.map((s) => s.id)).toEqual(["isolation", "crash", "escape", "taint", "secret", "csrf", "headers"]);
  });

  test("a fixed result is `no` and links the peer's own documentation", () => {
    for (const s of SCENARIOS) {
      for (const r of Object.values(s.fixed ?? {})) {
        expect(r.result).toBe("no");
        expect(String(r.evidence.docs)).toStartWith("https://");
      }
    }
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

  test("isolation is built in only when no request found another's value", () => {
    expect(isolationResult(["0 1", "0 2", "0 3"])).toEqual({ result: "built-in", evidence: { requests: 3, saw_another_request: 0 } });
    expect(isolationResult(["0 1", "1 2", "2 3"])).toEqual({ result: "no", evidence: { requests: 3, saw_another_request: 2 } });
    expect(isolationResult(["10 11"]).result).toBe("no");
  });

  test("a crash is survived when the next request is answered and the backend is still up", () => {
    expect(crashResult(500, 200, true).result).toBe("built-in");
    expect(crashResult(502, 200, true).evidence).toEqual({ crash_status: 502, next_status: 200, backend_running: true });
    expect(crashResult(500, 502, false).result).toBe("no");
    expect(crashResult(500, 200, false).result).toBe("no");
  });

  test("the headers are built in only with all three, and the frame policy may come from either header", () => {
    const all = { "x-content-type-options": "nosniff", "content-security-policy": "frame-ancestors 'none'", "referrer-policy": "no-referrer" };
    expect(headersResult(all).result).toBe("built-in");
    expect(headersResult({ ...all, "content-security-policy": "default-src 'self'", "x-frame-options": "DENY" }).result).toBe("built-in");
    expect(headersResult({ "x-content-type-options": "nosniff" })).toEqual({
      result: "your-code",
      evidence: { nosniff: true, frame_policy: false, referrer_policy: false },
    });
    expect(headersResult({ ...all, "x-content-type-options": "sniff" }).result).toBe("your-code");
  });

  test("the term printed raw needs the developer's escaper", () => {
    expect(escapeResult(`<h1>Results for ${ESCAPE_TERM}</h1>`).result).toBe("your-code");
    expect(escapeResult("<h1>Results for &lt;script&gt;alert(1)&lt;/script&gt;</h1>")).toEqual({ result: "built-in", evidence: { raw_script_in_body: false } });
  });

  test("a row's line names each peer's result in the table's order, with a setting where there is one", () => {
    const r = (result: RowResult["result"], setting?: string): RowResult => (setting === undefined ? { result, evidence: {} } : { result, setting, evidence: {} });
    const byPeer = new Map([["go", r("no")], ["nvs", r("one-setting", "limits.memory")], ["php", r("built-in")], ["bun", r("your-code")]]);
    expect(rowLine("runaway", byPeer)).toBe("runaway: Novis one-setting (limits.memory), PHP built-in, Bun your-code, Go no");
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
    machine: { processor: "cpu", cpus: 16, docker: "28.0.0" },
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
      "machine",
      "peers",
      "images",
      "cases",
      "config_digests",
    ]);
    expect(record.tls).toBe("TLSv1.3");
    expect(record.machine).toEqual(run.machine);
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
});
