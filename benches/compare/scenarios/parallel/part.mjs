// Runs one part in a `Worker` thread and posts its result back to `server.mjs`.
import { parentPort } from "node:worker_threads";
import { part } from "./work.mjs";

parentPort.on("message", ({ seed, turns }) => {
  parentPort.postMessage(part(seed, turns));
});
