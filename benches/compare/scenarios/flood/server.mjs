// `/hold` keeps 8 MiB for 300 ms and then returns `held`.
// Every other path returns `ok`.
import { setTimeout as sleep } from "node:timers/promises";

export default async function (req, res) {
  const headers = { "Content-Type": "text/html; charset=UTF-8" };
  if (new URL(req.url, "http://localhost").pathname === "/hold") {
    const held = new Uint8Array(8388608).fill(1);
    await sleep(300);
    res.writeHead(200, headers);
    res.end(held[held.length - 1] === 1 ? "held\n" : "");
    return;
  }
  res.writeHead(200, headers);
  res.end("ok\n");
}
