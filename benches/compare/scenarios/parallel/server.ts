// Runs `?parts=` parts of the same loop and returns the sum of their results.
// With four parts, each part runs in its own `Worker` thread from `part.ts`.
import { part } from "./work.ts";

function inWorker(seed: number, turns: number): Promise<number> {
  const worker = new Worker(new URL("./part.ts", import.meta.url));
  return new Promise((resolve) => {
    worker.onmessage = (event: MessageEvent<number>) => {
      resolve(event.data);
      worker.terminate();
    };
    worker.postMessage({ seed, turns });
  });
}

export default {
  async fetch(req: Request): Promise<Response> {
    const query = new URL(req.url).searchParams;
    const parts = Number(query.get("parts") ?? "1");
    const turns = Number(query.get("turns") ?? "0");
    let sum = 0;
    if (parts === 4) {
      const results = await Promise.all([1, 2, 3, 4].map((seed) => inWorker(seed, turns)));
      sum = results.reduce((a, b) => a + b, 0);
    } else {
      sum = part(1, turns);
    }
    return new Response(String(sum), { headers: { "Content-Type": "text/html; charset=UTF-8" } });
  },
};
