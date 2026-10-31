import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { newestNvs } from "../lib/nvs-binary.ts";
import { ROOT } from "../lib/paths.ts";
import { Browser, EXE } from "./chromium.ts";

// The registry's pages in a real Chromium (`rule:packaging/the-registry-is-a-novis-program`). A copy
// of `apps/registry` is served by `nvs serve` behind a proxy on a free port, so the `origin` its
// `hosting.json` names is the one the browser sees, and a passkey is CDP's virtual authenticator.
//
// Webhooks and push endpoints must be `https:`, so one TLS receiver stands for both. Its certificate
// comes from a CA `openssl` makes per run, which the copy trusts through `[http.client.tls] roots`.
// Headless Chromium has no push service: `PushManager.prototype.subscribe` is replaced with a stub
// whose keys the test made, the test decrypts the message the registry sent to the receiver, and hands
// it to the service worker with `ServiceWorker.deliverPushMessage`. An operator's `nvs run` gives an
// account a vendor or a verified channel, as the application's own `BrowserTest` does, and saves a
// release as a publish with a token does, without the git host a real publish needs.
//
// Each test has its own browser context, so its cookies, service worker and authenticator are its own.
// A step ends on a page condition, polled, with a bound that only names a stuck step.

// A relative `NVS_BIN`, as CI gives, is resolved from the repository root, because `serve` runs the
// binary from inside the registry's copy.
const NVS = process.env.NVS_BIN ? resolve(ROOT, process.env.NVS_BIN) : newestNvs();
const DIR = join(ROOT, ".agent-tmp", `registry-copy-${process.pid}`);
const APP = join(DIR, "registry");
const enc = new TextEncoder();
const b64url = (b: ArrayBuffer | Uint8Array) => Buffer.from(b instanceof Uint8Array ? b : new Uint8Array(b)).toString("base64url");

/** The longest one step of a page may take. A bound on a stuck step, never a wait a test relies on. */
const STEP_MS = 60_000;

/** One request the TLS receiver took: a webhook message or an encrypted push message. */
type Taken = { path: string; body: Uint8Array<ArrayBuffer> };

/** The receiver of every webhook and push message, on `https://127.0.0.1:<port>`. */
class Receiver {
  private taken: Taken[] = [];
  private waiting: (() => void)[] = [];
  readonly server: ReturnType<typeof Bun.serve>;

  constructor(cert: string, key: string) {
    this.server = Bun.serve({
      port: 0,
      hostname: "127.0.0.1",
      tls: { cert, key },
      fetch: async (req) => {
        const path = new URL(req.url).pathname;
        this.taken.push({ path, body: new Uint8Array(await req.arrayBuffer()) });
        for (const w of this.waiting.splice(0)) w();
        return new Response(null, { status: path.startsWith("/push/") ? 201 : 200 });
      },
    });
  }

  url(path: string) {
    return `https://127.0.0.1:${this.server.port}${path}`;
  }

  /** The first message to `path` that `accept` takes, now or once it arrives. */
  async next<T>(path: string, accept: (t: Taken) => Promise<T | null>): Promise<T> {
    for (let seen = 0; ; ) {
      while (seen < this.taken.length) {
        const t = this.taken[seen++]!;
        if (t.path !== path) continue;
        const got = await accept(t);
        if (got !== null) return got;
      }
      await new Promise<void>((ok) => this.waiting.push(ok));
    }
  }
}

/** The keys of one browser's push subscription, which the stub returns and the test decrypts with. */
type PushKeys = { pair: CryptoKeyPair; publicRaw: Uint8Array<ArrayBuffer>; auth: Uint8Array<ArrayBuffer> };

async function pushKeys(): Promise<PushKeys> {
  const pair = (await crypto.subtle.generateKey({ name: "ECDH", namedCurve: "P-256" }, true, ["deriveBits"])) as CryptoKeyPair;
  const publicRaw = new Uint8Array(await crypto.subtle.exportKey("raw", pair.publicKey));
  return { pair, publicRaw, auth: crypto.getRandomValues(new Uint8Array(16)) };
}

async function hkdf(ikm: ArrayBuffer, salt: Uint8Array<ArrayBuffer>, info: Uint8Array<ArrayBuffer>, bits: number): Promise<ArrayBuffer> {
  const key = await crypto.subtle.importKey("raw", ikm, "HKDF", false, ["deriveBits"]);
  return crypto.subtle.deriveBits({ name: "HKDF", hash: "SHA-256", salt, info }, key, bits);
}

/** The text of one RFC 8291 `aes128gcm` message of one record, as the browser's push service would. */
async function decrypt(body: Uint8Array<ArrayBuffer>, keys: PushKeys): Promise<string> {
  const salt = body.slice(0, 16);
  const idlen = body[20]!;
  const serverRaw = body.slice(21, 21 + idlen);
  const server = await crypto.subtle.importKey("raw", serverRaw, { name: "ECDH", namedCurve: "P-256" }, false, []);
  const shared = await crypto.subtle.deriveBits({ name: "ECDH", public: server }, keys.pair.privateKey, 256);
  const info = new Uint8Array([...enc.encode("WebPush: info\0"), ...keys.publicRaw, ...serverRaw]);
  const ikm = await hkdf(shared, keys.auth, info, 256);
  const cek = await hkdf(ikm, salt, enc.encode("Content-Encoding: aes128gcm\0"), 128);
  const nonce = await hkdf(ikm, salt, enc.encode("Content-Encoding: nonce\0"), 96);
  const aes = await crypto.subtle.importKey("raw", cek, "AES-GCM", false, ["decrypt"]);
  const plain = new Uint8Array(await crypto.subtle.decrypt({ name: "AES-GCM", iv: nonce }, aes, body.slice(21 + idlen)));
  let end = plain.length - 1;
  while (end > 0 && plain[end] === 0) end--;
  return new TextDecoder().decode(plain.slice(0, end));
}

/** `openssl` on the PATH, or else the copy Git for Windows ships, which a PATH with only `git` misses. */
function openssl(): string {
  const found = Bun.which("openssl");
  if (found) return found;
  const git = Bun.which("git");
  if (git) {
    // `git` is `<Git>/cmd/git.exe` or `<Git>/mingw64/bin/git.exe`.
    const up = dirname(dirname(git));
    for (const root of [up, dirname(up)]) {
      for (const p of [join(root, "usr", "bin", "openssl.exe"), join(root, "mingw64", "bin", "openssl.exe")]) {
        if (existsSync(p)) return p;
      }
    }
  }
  throw new Error("openssl is not on the PATH, and Git for Windows has no copy beside git");
}

/** A CA and a certificate for `127.0.0.1` it signed, made with `openssl` under `dir`. */
function certificates(dir: string): { ca: string; cert: string; key: string } {
  const exe = openssl();
  const run = (args: string[]) => {
    const r = Bun.spawnSync([exe, ...args], { cwd: dir, stdout: "pipe", stderr: "pipe" });
    if (r.exitCode !== 0) throw new Error(`openssl ${args.join(" ")}:\n${r.stderr.toString()}`);
  };
  const ec = ["-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:prime256v1", "-noenc", "-days", "2"];
  run(["req", "-x509", ...ec, "-keyout", "ca.key", "-out", "ca.pem", "-subj", "/CN=Test CA", "-addext", "keyUsage=critical,keyCertSign"]);
  run(["req", "-x509", ...ec, "-keyout", "host.key", "-out", "host.pem", "-subj", "/CN=127.0.0.1", "-CA", "ca.pem", "-CAkey", "ca.key", "-addext", "subjectAltName=IP:127.0.0.1", "-addext", "basicConstraints=critical,CA:FALSE"]);
  const read = (f: string) => readFileSync(join(dir, f), "utf8");
  return { ca: read("ca.pem"), cert: read("host.pem"), key: read("host.key") };
}

/** A P-256 private key in a PKCS #8 PEM file, the shape `pushKey` names. */
async function vapidPem(): Promise<string> {
  const pair = (await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, ["sign"])) as CryptoKeyPair;
  const der = Buffer.from(await crypto.subtle.exportKey("pkcs8", pair.privateKey)).toString("base64");
  return `-----BEGIN PRIVATE KEY-----\n${der.match(/.{1,64}/g)!.join("\n")}\n-----END PRIVATE KEY-----\n`;
}

/** What `nvs run` runs to give an account a verified webhook or a vendor. A webhook under `.invalid`
 * never has an address, so each message to it fails at once with no network. */
const SCRIPTS: Record<string, string> = {
  "channel.nvs": `<?nvs
autoload 'Registry' from '../src';

array<tainted string> $words = Core\\Cli::arguments();
string $account = Registry\\Accounts::idOf($words[0]) ?? '';
var $channels = new Registry\\Channels(Registry\\Schema::open());
array<string> $added = $channels->add($account, Registry\\Channels::WEBHOOK, 'https://hooks.invalid/' . $account);
if (!$channels->verify($account, $added['id'], $added['code'])) {
    throw new RuntimeError('The webhook is not verified.');
}
`,
  "vendor.nvs": `<?nvs
autoload 'Registry' from '../src';

array<tainted string> $words = Core\\Cli::arguments();
string $account = Registry\\Accounts::idOf($words[0]) ?? '';
string $vendor = Core\\Taint::assertTrusted($words[1], 'the test names the vendor');
if (!new Registry\\Vendors(Registry\\Schema::open())->claim($vendor, $account)) {
    throw new RuntimeError('The vendor is not claimed.');
}
`,
  "release.nvs": `<?nvs
autoload 'Registry' from '../src';

array<tainted string> $words = Core\\Cli::arguments();
string $account = Registry\\Accounts::idOf($words[0]) ?? '';
string $name = Core\\Taint::assertTrusted($words[1], 'the test names the package');
string $version = Core\\Taint::assertTrusted($words[2], 'the test names the version');
string $api = Core\\Taint::assertTrusted($words[3], 'the test wrote the API file');
string $key = Core\\Str::lower($name);
array<string> $parts = Core\\Str::split($version, '.');
int $now = Core\\Time::now()->toEpochSeconds();
var $db = Registry\\Schema::open();
$db->execute(
    'insert into packages (key, name, vendor, repository, directory, created_at) values (?, ?, ?, ?, ?, ?) on conflict (key) do nothing',
    [$key, $name, Core\\Str::before($key, '\\\\') ?? $key, 'https://git.example.com/project.git', '', $now],
);
$db->execute(
    'insert into releases (package, version, major, minor, patch, digest, size, repository, directory, git_commit, account, token, archive, api, '
        . 'published_at, pending_until, dependencies, capabilities) values (?, ?, ?, ?, ?, ?, 10, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
    [
        $key, $version, $parts[0] as int, $parts[1] as int, $parts[2] as int,
        'blake3:' . Core\\Encoding::toHex(Core\\Random::bytes(32)), 'https://git.example.com/project.git', '',
        Core\\Str::repeat('1', 40), $account, 'token', '', $api, $now, $now + 7 * 86400, '{}', '{"required":[],"optional":[]}',
    ],
);
`,
};

let browser: Browser;
let receiver: Receiver;
let proxy: ReturnType<typeof Bun.serve>;
let server: ReturnType<typeof Bun.spawn> | undefined;
let said = "";
let origin = "";

/** Runs `jobs/<script>` in the copy, as the operator would. */
function operator(script: string, ...args: string[]) {
  const r = Bun.spawnSync([NVS!, "--data", join(DIR, "data"), "--no-init", "run", `jobs/${script}`, "--config", "nvs.toml", ...args], {
    cwd: APP,
    stdout: "pipe",
    stderr: "pipe",
  });
  if (r.exitCode !== 0) throw new Error(`jobs/${script}: ${r.stdout}${r.stderr}`);
}

/** Gives `account` the vendor of `name`, such as `Store\Lib`, and saves each version of the package,
 * pending for seven days. Each version's API is `api`. Returns the address of the package page. */
function publish(account: string, name: string, versions: string[], api: object = { classes: [] }): string {
  const vendor = name.split("\\")[0]!;
  operator("vendor.nvs", account, vendor);
  mkdirSync(join(APP, "builds"), { recursive: true });
  for (const version of versions) {
    const file = join(APP, "builds", `${name.replace("\\", "-")}-${version}.json`);
    writeFileSync(file, JSON.stringify(api));
    operator("release.nvs", account, name, version, file);
  }
  return `/packages/${name.replace("\\", "/").toLowerCase()}`;
}

/** Serves the copy and resolves with its address once it prints its `listening on` line. */
async function serve(): Promise<string> {
  server = Bun.spawn([NVS!, "--data", join(DIR, "data"), "--no-init", "serve", "public/index.nvs", "--config", "nvs.toml", "--listen", "127.0.0.1:0"], {
    cwd: APP,
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  const drain = async (s: ReadableStream<Uint8Array>, found?: (addr: string) => void) => {
    const dec = new TextDecoder();
    for await (const chunk of s) {
      said += dec.decode(chunk);
      const addr = /listening on http:\/\/(\S+)/.exec(said)?.[1];
      if (addr && found) found(addr);
    }
  };
  void drain(server.stderr as ReadableStream<Uint8Array>);
  return new Promise((ok, fail) => {
    void drain(server!.stdout as ReadableStream<Uint8Array>, ok).then(() => fail(new Error(`nvs serve ended:\n${said}`)));
  });
}

beforeAll(async () => {
  if (!EXE) return;
  if (!NVS) throw new Error("no built nvs: run `cargo build`");
  mkdirSync(join(APP, "keys"), { recursive: true });
  mkdirSync(join(DIR, "data"), { recursive: true });
  for (const folder of ["src", "public", "jobs", "fixtures"]) cpSync(join(ROOT, "apps", "registry", folder), join(APP, folder), { recursive: true });
  for (const [name, text] of Object.entries(SCRIPTS)) writeFileSync(join(APP, "jobs", name), text);
  const tls = certificates(join(APP, "keys"));
  receiver = new Receiver(tls.cert, tls.key);
  writeFileSync(join(APP, "keys", "push.pem"), await vapidPem());

  let target = "";
  proxy = Bun.serve({
    port: 0,
    hostname: "127.0.0.1",
    async fetch(req) {
      const url = new URL(req.url);
      const headers = new Headers(req.headers);
      headers.delete("accept-encoding");
      const body = req.method === "GET" || req.method === "HEAD" ? undefined : await req.arrayBuffer();
      const r = await fetch(`http://${target}${url.pathname}${url.search}`, { method: req.method, headers, body, redirect: "manual" });
      return new Response(r.body, { status: r.status, headers: r.headers });
    },
  });
  origin = `http://localhost:${proxy.port}`;

  const hosting = JSON.parse(readFileSync(join(ROOT, "apps", "registry", "hosting.json"), "utf8"));
  writeFileSync(join(APP, "hosting.json"), JSON.stringify({ ...hosting, origin, signUpWork: 4, pushKey: "keys/push.pem" }));
  const config = readFileSync(join(ROOT, "apps", "registry", "nvs.toml"), "utf8").replace(/\r\n/g, "\n");
  const changed = config
    .replace('read = ["hosting.json",', 'read = ["keys", "hosting.json",')
    .replace("[capabilities.net]\ninternal = []\n", '[capabilities.net]\ninternal = ["127.0.0.1"]\n');
  if (!changed.includes('"keys"') || !changed.includes('internal = ["127.0.0.1"]')) throw new Error("apps/registry/nvs.toml changed its `read` or `internal` line");
  writeFileSync(join(APP, "nvs.toml"), `${changed}\n[http.client.tls]\nroots = ["keys/ca.pem"]\n`);

  target = await serve();
  browser = await Browser.launch("registry-browser", ["--host-resolver-rules=MAP localhost 127.0.0.1"]);
}, 300_000);

afterAll(async () => {
  await browser?.close();
  server?.kill();
  await server?.exited;
  proxy?.stop(true);
  receiver?.server.stop(true);
  for (let tries = 1; ; tries++) {
    try {
      rmSync(DIR, { recursive: true, force: true });
      return;
    } catch (err) {
      if (tries >= 40) throw err;
      await Bun.sleep(250);
    }
  }
}, 60_000);

/** One tab in a browser context of its own, with a virtual passkey device and a push stub. */
class Page {
  readonly events: { method: string; params: any }[] = [];
  private authenticator = "";

  private constructor(
    private context: string,
    readonly session: string,
    readonly keys: PushKeys,
    readonly endpoint: string,
  ) {}

  static async open(): Promise<Page> {
    const { browserContextId } = await browser.send("Target.createBrowserContext");
    const { targetId } = await browser.send("Target.createTarget", { url: "about:blank", browserContextId });
    const { sessionId } = await browser.send("Target.attachToTarget", { targetId, flatten: true });
    const keys = await pushKeys();
    const page = new Page(browserContextId, sessionId, keys, receiver.url(`/push/${crypto.randomUUID()}`));
    browser.on((method, params, from) => {
      if (from === sessionId) page.events.push({ method, params });
    });
    const subscription = { endpoint: page.endpoint, expirationTime: null, keys: { p256dh: b64url(keys.publicRaw), auth: b64url(keys.auth) } };
    await page.send("Page.enable");
    await page.send("Page.addScriptToEvaluateOnNewDocument", {
      source: `PushManager.prototype.subscribe = async function () {
  const json = ${JSON.stringify(subscription)};
  return { endpoint: json.endpoint, toJSON: () => json };
};`,
    });
    await page.send("Emulation.setFocusEmulationEnabled", { enabled: true });
    await page.send("WebAuthn.enable");
    await page.newDevice();
    return page;
  }

  send(method: string, params: object = {}) {
    return browser.send(method, params, this.session);
  }

  /** Replaces the passkey device with a new one that has no passkeys yet. */
  async newDevice() {
    if (this.authenticator) await this.send("WebAuthn.removeVirtualAuthenticator", { authenticatorId: this.authenticator });
    const { authenticatorId } = await this.send("WebAuthn.addVirtualAuthenticator", {
      options: { protocol: "ctap2", transport: "internal", hasResidentKey: true, hasUserVerification: true, isUserVerified: true, automaticPresenceSimulation: true },
    });
    this.authenticator = authenticatorId;
  }

  /** The value of `expression` in the page, or `undefined` while a navigation replaces it. */
  async eval(expression: string): Promise<any> {
    try {
      return await browser.eval(expression, this.session);
    } catch (err) {
      if (/context|navigat|Cannot find/i.test(String(err))) return undefined;
      throw err;
    }
  }

  /** Waits until `expression` is true. A step that does not end names the status line it stopped at. */
  async until(expression: string) {
    const stuck = Date.now() + STEP_MS;
    while (!(await this.eval(expression))) {
      if (Date.now() > stuck) {
        const status = await this.eval(`document.getElementById("status")?.textContent`);
        throw new Error(`the page never made \`${expression}\` true. The status line says: ${status}\nthe server: ${said}`);
      }
      await Bun.sleep(25);
    }
  }

  /** Opens `path` and waits until it has loaded. */
  async go(path: string) {
    await this.send("Page.navigate", { url: origin + path });
    await this.until(loaded(path));
  }

  /** Fills the form `selector` with `fields`, sends it, and waits until `done` is true. */
  async act(selector: string, fields: Record<string, string>, done: string) {
    const fill = Object.entries(fields)
      .map(([name, value]) => `f.elements[${JSON.stringify(name)}].value = ${JSON.stringify(value)};`)
      .join("\n");
    const sent = await this.eval(`(() => {
  const f = document.querySelector(${JSON.stringify(selector)});
  if (!f) return false;
  document.getElementById("status").textContent = "";
  ${fill}
  window.__sent = true;
  f.requestSubmit();
  return true;
})()`);
    if (!sent) throw new Error(`the page has no ${selector}`);
    await this.until(done);
  }

  /** Ticks the checkbox `name`, which `act` cannot fill. */
  async tick(name: string) {
    await this.eval(`document.querySelector('input[name=${JSON.stringify(name)}]').checked = true`);
  }

  text(): Promise<string> {
    return this.eval("document.body.innerText");
  }

  close() {
    return browser.send("Target.disposeBrowserContext", { browserContextId: this.context });
  }
}

const loaded = (path: string) => `location.pathname === ${JSON.stringify(path)} && document.readyState === "complete"`;
/** Whether the page has loaded again since `act` sent its form. */
const reloaded = (path: string) => `${loaded(path)} && window.__sent !== true`;
/** Whether the page has the element `selector`. */
const has = (selector: string) => `!!document.querySelector(${JSON.stringify(selector)})`;
const shows = (words: string) => `document.body.innerText.includes(${JSON.stringify(words)})`;
/** The state of `version` in the table of versions on a package page. */
const stateOf = (version: string) =>
  `[...document.querySelectorAll("tr")].find((r) => r.cells[0]?.textContent === ${JSON.stringify(version)})?.cells[1].textContent`;
const status = (words: string) => `(document.getElementById("status")?.textContent ?? "").includes(${JSON.stringify(words)})`;
/** Whether the table of push devices and webhooks has a row whose kind starts with `kind` and that counts. */
const counts = (kind: string) =>
  `[...document.querySelectorAll("tr")].some((r) => r.cells[0]?.textContent.startsWith(${JSON.stringify(kind)}) && r.cells[4]?.textContent === "yes")`;

/** Signs up on `/signup` and returns the account's id and its recovery codes. */
async function signUp(page: Page): Promise<{ account: string; codes: string[] }> {
  await page.go("/signup");
  await page.act('form[data-action="signup"]', {}, status("Your account is ready"));
  const shown: string = await page.eval(`document.getElementById("status").textContent`);
  const account = /Its id is (\S+)\.$/.exec(shown)?.[1];
  if (!account) throw new Error(`the status line names no account: ${shown}`);
  const codes: string[] = await page.eval(`[...document.querySelectorAll("#code-list li")].map((li) => li.textContent)`);
  return { account, codes };
}

describe.skipIf(!EXE)("the registry's pages in Chromium", () => {
  test("a visitor signs up with a passkey and sees the account page", async () => {
    const page = await Page.open();
    const { account, codes } = await signUp(page);
    expect(codes.length).toBe(10);
    expect(await page.eval(`document.getElementById("codes").hidden`)).toBe(false);
    await page.go("/account");
    const text = await page.text();
    expect(text).toContain(account);
    expect(text).toContain("You signed in with a passkey.");
    await page.close();
  }, 120_000);

  test("a person signs in with a passkey and with a recovery code", async () => {
    const page = await Page.open();
    const { account, codes } = await signUp(page);
    await page.go("/account");
    await page.act('form[data-action="signout"]', {}, loaded("/"));
    await page.go("/signin");
    await page.act('form[data-action="signin"]', {}, loaded("/account"));
    expect(await page.text()).toContain("You signed in with a passkey.");

    await page.act('form[data-action="signout"]', {}, loaded("/"));
    await page.go("/signin");
    await page.act('form[data-action="recovery"]', { code: codes[0]! }, loaded("/account"));
    const text = await page.text();
    expect(text).toContain(account);
    expect(text).toContain("You did not sign in with a passkey in the last 15 minutes.");

    // A recovery code signs in one time only.
    await page.act('form[data-action="signout"]', {}, loaded("/"));
    await page.go("/signin");
    await page.act('form[data-action="recovery"]', { code: codes[0]! }, `${loaded("/signin")} && document.getElementById("status").textContent !== ""`);
    expect(await page.eval("location.pathname")).toBe("/signin");
    await page.close();
  }, 120_000);

  test("a signed-in person adds a second passkey", async () => {
    const page = await Page.open();
    await signUp(page);
    await page.go("/account");
    await page.newDevice();
    await page.act('form[data-action="add-passkey"]', {}, status("Your account has 2 passkeys."));
    await page.go("/account");
    expect(await page.eval(`document.querySelectorAll('form[data-action="add-passkey"]')[0].previousElementSibling.rows.length`)).toBe(3);

    // The new device signs in alone, so its passkey belongs to the account.
    await page.act('form[data-action="signout"]', {}, loaded("/"));
    await page.go("/signin");
    await page.act('form[data-action="signin"]', {}, loaded("/account"));
    expect(await page.text()).toContain("You signed in with a passkey.");
    await page.close();
  }, 120_000);

  test("a person makes a token, sees it once and revokes it", async () => {
    const page = await Page.open();
    await signUp(page);
    await page.go("/account");
    await page.act('form[data-action="token"]', { scope: "Blog", days: "30" }, status("The token is made. It works for 30 days."));
    const token: string = await page.eval(`document.getElementById("token-text").textContent`);
    expect(token.length).toBeGreaterThan(20);
    expect(await page.eval(`document.getElementById("new-token").hidden`)).toBe(false);

    await page.go("/account");
    expect(await page.eval(`document.getElementById("token-text").textContent`)).toBe("");
    expect(await page.text()).not.toContain(token);
    await page.act('form[data-action="revoke-token"]', {}, `${loaded("/account")} && !document.querySelector('form[data-action="revoke-token"]')`);
    expect(await page.text()).toContain("revoked on");
    await page.close();
  }, 120_000);

  test("a person adds a push device and verifies it from the test push", async () => {
    const page = await Page.open();
    await signUp(page);
    await page.send("ServiceWorker.enable");
    await page.go("/account");
    await page.act('form[data-action="add-device"]', {}, status("This browser is added."));

    const message = await receiver.next(new URL(page.endpoint).pathname, async (t) => {
      const text = await decrypt(t.body, page.keys);
      return JSON.parse(text).code ? text : null;
    });
    const registration = page.events
      .filter((e) => e.method === "ServiceWorker.workerRegistrationUpdated")
      .flatMap((e) => e.params.registrations)
      .find((r: any) => r.scopeURL.startsWith(origin) && !r.isDeleted);
    expect(registration).toBeDefined();
    await page.send("ServiceWorker.deliverPushMessage", { origin, registrationId: registration.registrationId, data: message });

    await page.until(`fetch("/api/channels").then((r) => r.json()).then((j) => j.channels.some((c) => c.verifiedAt !== null))`);
    await page.go("/account");
    expect(await page.eval(counts("Push device"))).toBe(true);
    await page.close();
  }, 120_000);

  test("a person adds a webhook and verifies it with its code", async () => {
    const page = await Page.open();
    await signUp(page);
    const hook = `/hook/${crypto.randomUUID()}`;
    await page.go("/account");
    await page.act('form[data-action="add-webhook"]', { url: receiver.url(hook) }, status("The webhook is added."));
    expect((await page.eval(`document.getElementById("webhook-secret").textContent`)).startsWith("whsec_")).toBe(true);

    const code = await receiver.next(hook, async (t) => (JSON.parse(new TextDecoder().decode(t.body)).code as string) || null);
    await page.go("/account");
    expect(await page.eval(counts("Webhook"))).toBe(false);
    await page.act('form[data-action="verify-channel"]', { code }, `${loaded("/account")} && ${counts("Webhook")}`);
    await page.close();
  }, 120_000);

  test("an owner adds and removes a second owner", async () => {
    const other = await Page.open();
    const second = (await signUp(other)).account;
    await other.close();
    operator("channel.nvs", second);

    const page = await Page.open();
    const { account } = await signUp(page);
    operator("vendor.nvs", account, "Shop");
    await page.go("/account");
    const removeSecond = `form[data-action="remove-owner"][data-id="${second}"]`;
    await page.act('form[data-action="add-owner"]', { account: second }, `${loaded("/account")} && !!document.querySelector(${JSON.stringify(removeSecond)})`);
    await page.act(removeSecond, {}, `${loaded("/account")} && !document.querySelector(${JSON.stringify(removeSecond)})`);
    expect(await page.text()).toContain(`${account} (you)`);
    await page.close();
  }, 120_000);

  test("an owner releases a pending version now", async () => {
    const page = await Page.open();
    const { account } = await signUp(page);
    // A version below 0.1.0 waits seven days. The first version at 0.1.0 or higher waits for its API.
    const path = `${publish(account, "Store\\Lib", ["0.0.1"])}/0.0.1`;
    await page.go(path);
    expect(await page.text()).toContain("pending until");
    await page.act('form[data-action="release"]', {}, `${reloaded(path)} && !${has('form[data-action="release"]')}`);
    expect(await page.eval(`document.querySelector("table td").textContent`)).toBe("released");
    await page.close();
  }, 120_000);

  test("an owner freezes the first stable API after reading it", async () => {
    const page = await Page.open();
    const { account } = await signUp(page);
    const path = `${publish(account, "Wiki\\Lib", ["0.1.0"], { classes: [{ name: "Wiki\\Lib\\Page" }] })}/0.1.0`;
    await page.go(path);
    expect(await page.text()).toContain("waits for an owner");
    await page.act('form[data-action="show-api"]', {}, `!document.getElementById("api").hidden`);
    expect(await page.eval(`document.getElementById("api").textContent`)).toContain("Wiki\\\\Lib\\\\Page");
    await page.tick("read");
    await page.act('form[data-action="stable"]', {}, `${reloaded(path)} && !${has('form[data-action="stable"]')}`);
    expect(await page.eval(`document.querySelector("table td").textContent`)).toBe("released");
    await page.close();
  }, 120_000);

  test("an owner unlists a version and lists it again", async () => {
    const page = await Page.open();
    const { account } = await signUp(page);
    const path = publish(account, "News\\Lib", ["0.0.1", "0.0.2"]);
    await page.go(path);
    await page.act('form[data-action="unlist"][data-version="0.0.1"]', {}, `${reloaded(path)} && ${has('form[data-action="relist"][data-version="0.0.1"]')}`);
    expect(await page.eval(stateOf("0.0.1"))).toBe("unlisted");
    expect(await page.eval(stateOf("0.0.2"))).toStartWith("pending until");
    await page.act('form[data-action="relist"][data-version="0.0.1"]', {}, `${reloaded(path)} && ${has('form[data-action="unlist"][data-version="0.0.1"]')}`);
    expect(await page.eval(stateOf("0.0.1"))).toStartWith("pending until");
    await page.close();
  }, 120_000);

  test("an owner retracts a version and sends an advisory", async () => {
    const page = await Page.open();
    const { account } = await signUp(page);
    const path = publish(account, "Docs\\Lib", ["0.0.1", "0.0.2"]);
    await page.go(path);
    const reason = "It deletes the cache.";
    await page.act('form[data-action="retract"]', { version: "0.0.1", reason, fixedIn: "0.0.2" }, `${reloaded(path)} && ${shows(reason)}`);
    expect(await page.text()).toContain("Version 0.0.2 fixes it.");
    expect(await page.eval(stateOf("0.0.1"))).toBe("retracted");
    const summary = "A crafted header reads other requests.";
    await page.act('form[data-action="advisory"]', { versions: "0.0.1..0.0.2", summary }, `${reloaded(path)} && ${shows(summary)}`);
    expect(await page.text()).toContain("docs/lib-1");
    await page.close();
  }, 120_000);

  test("an owner moves the package to another repository", async () => {
    const page = await Page.open();
    const { account } = await signUp(page);
    const path = publish(account, "Forum\\Lib", ["0.0.1"]);
    await page.go(path);
    const repository = "https://git.example.com/forum/lib.git";
    await page.act('form[data-action="repository"]', { repository, directory: "packages/lib" }, reloaded(path));
    expect(await page.eval(`document.querySelector('input[name="repository"]').value`)).toBe(repository);
    expect(await page.eval(`document.querySelector('input[name="directory"]').value`)).toBe("packages/lib");
    await page.close();
  }, 120_000);
});
