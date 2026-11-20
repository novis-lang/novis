// Runs `?parts=` parts of the same loop and returns the sum of their results.
// With four parts, each part runs in its own `Worker` thread from `part.mjs`.
import { Worker } from "node:worker_threads";
import { part } from "./work.mjs";

function inWorker(seed, turns) {
  const worker = new Worker(new URL("./part.mjs", import.meta.url));
  return new Promise((resolve) => {
    worker.once("message", (result) => {
      resolve(result);
      worker.terminate();
    });
    worker.postMessage({ seed, turns });
  });
}

export default async function (req, res) {
  const query = new URL(req.url, "http://localhost").searchParams;
  const parts = Number(query.get("parts") ?? "1");
  const turns = Number(query.get("turns") ?? "0");
  let sum = 0;
  if (parts === 4) {
    const results = await Promise.all([1, 2, 3, 4].map((seed) => inWorker(seed, turns)));
    sum = results.reduce((a, b) => a + b, 0);
  } else {
    sum = part(1, turns);
  }
  res.writeHead(200, { "Content-Type": "text/html; charset=UTF-8" });
  res.end(String(sum));
}
