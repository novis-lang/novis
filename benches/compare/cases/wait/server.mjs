// Waits 50 ms on a timer, then answers. This measures the cost of many
// requests in flight at once, as when a page waits for another service.
import { setTimeout as sleep } from "node:timers/promises";

const text = { "Content-Type": "text/plain; charset=UTF-8" };

export default async function (req, res) {
  if (new URL(req.url, "http://localhost").pathname !== "/wait") {
    res.writeHead(404, text);
    res.end("not found\n");
    return;
  }
  await sleep(50);
  res.writeHead(200, text);
  res.end("waited 50 ms\n");
}
