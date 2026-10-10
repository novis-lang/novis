// Runs one part in a `Worker` thread and posts its result back to `server.ts`.
import { part } from "./work.ts";

declare var self: Worker;

self.onmessage = (event: MessageEvent<{ seed: number; turns: number }>) => {
  postMessage(part(event.data.seed, event.data.turns));
};
