// `bun nv proxy-validate`: what each kind of file is, which certificates an nginx file gets stubs for,
// the script `nginx -t` runs in, and how Caddy's log becomes its error. Nothing here starts Docker.

import { expect, test } from "bun:test";
import { CADDY_BUILDER_IMAGE, CADDY_IMAGE, CADDY_PLUGINS, caddyMessage, kindOf, NGINX_IMAGE, nginxCertPaths, nginxScript, validate } from "../cmd/proxy-validate.ts";

test("a file's kind comes from its name, and nginx is the rest", () => {
  expect(kindOf("apps/chat/deploy/Caddyfile")).toBe("caddyfile");
  expect(kindOf("site.caddyfile")).toBe("caddyfile");
  expect(kindOf("Dockerfile")).toBe("dockerfile");
  expect(kindOf("deploy/nginx.conf")).toBe("nginx");
});

test("each certificate and key path gets a stub, and a path with a variable does not", () => {
  const text = [
    "ssl_certificate     /etc/letsencrypt/live/shop.example.com/fullchain.pem;",
    "  ssl_certificate_key '/etc/ssl/private/shop.key';",
    "ssl_certificate /etc/ssl/$ssl_server_name.crt;",
    "ssl_client_certificate /etc/ssl/clients.pem;",
  ].join("\n");
  expect(nginxCertPaths(text)).toEqual({
    certs: ["/etc/letsencrypt/live/shop.example.com/fullchain.pem", "/etc/ssl/clients.pem"],
    keys: ["/etc/ssl/private/shop.key"],
  });
});

test("a conf.d file replaces the default site, and a file with an http block replaces nginx.conf", () => {
  const site = nginxScript("server { listen 80; }\n");
  expect(site).toContain("cat > /etc/nginx/conf.d/site.conf");
  expect(site).not.toContain("openssl");
  expect(site.endsWith("nginx -t -q")).toBe(true);
  const whole = nginxScript("events {}\nhttp {\n  server { ssl_certificate /c.pem; ssl_certificate_key /k.pem; }\n}\n");
  expect(whole).toContain("cat > /etc/nginx/nginx.conf");
  expect(whole).toContain("cp /tmp/stub.crt '/c.pem'");
  expect(whole).toContain("cp /tmp/stub.key '/k.pem'");
});

test("Caddy's error is each error line of its JSON log, and its last lines without one", () => {
  const log = [
    '{"level":"info","ts":1,"msg":"using config from file","file":"/tmp/Caddyfile"}',
    '{"level":"error","ts":2,"msg":"adapting config using caddyfile: /tmp/Caddyfile:2: unrecognized directive: reverse_prox"}',
  ].join("\n");
  expect(caddyMessage(log)).toBe("adapting config using caddyfile: /tmp/Caddyfile:2: unrecognized directive: reverse_prox");
  expect(caddyMessage("Error: loading config: bad\n")).toBe("Error: loading config: bad");
  expect(caddyMessage("one\ntwo\n")).toBe("one\ntwo");
});

test("a plugin with no pinned version, or on a file that is not a Caddyfile, fails before Docker starts", async () => {
  expect(await validate("caddyfile", "x {}", ["github.com/example/unknown"])).toEqual({
    problems: ["no pinned version for the Caddy plugin `github.com/example/unknown`: add it to `CADDY_PLUGINS` in tools/nv/cmd/proxy-validate.ts"],
  });
  expect(await validate("nginx", "", Object.keys(CADDY_PLUGINS))).toEqual({ problems: ["only a Caddyfile takes plugins"] });
});

test("every image is pinned by digest, and every plugin by version", () => {
  for (const image of [NGINX_IMAGE, CADDY_IMAGE, CADDY_BUILDER_IMAGE]) expect(image).toMatch(/^[a-z]+@sha256:[0-9a-f]{64}$/);
  for (const v of Object.values(CADDY_PLUGINS)) expect(v).toMatch(/^v\d+\.\d+\.\d+/);
});
