import { expect, test } from "bun:test";
import { holdOrigin, withOrigin } from "../driver/origin.ts";

const PORT = 18099;

test("the origin answers /ok with ok, any other path with 404, and leaves a second listener alone", async () => {
  const origin = await holdOrigin("127.0.0.1", PORT, 5);
  try {
    expect(origin.line).toStartWith("origin: serving");
    const ok = await fetch(`http://127.0.0.1:${PORT}/ok`);
    expect(ok.status).toBe(200);
    expect(await ok.text()).toBe("ok");
    const missing = await fetch(`http://127.0.0.1:${PORT}/other`);
    expect(missing.status).toBe(404);
    const second = await holdOrigin("127.0.0.1", PORT, 5);
    expect(second.line).toContain("already has a listener");
    second.close();
    expect((await fetch(`http://127.0.0.1:${PORT}/ok`)).status).toBe(200);
  } finally {
    origin.close();
  }
});

test("withOrigin serves while its function runs and closes the port after it throws", async () => {
  const seen = await withOrigin(async () => (await fetch(`http://127.0.0.1:${PORT}/ok`)).status, "127.0.0.1", PORT, 5);
  expect(seen).toBe(200);
  await expect(
    withOrigin(
      async () => {
        throw new Error("a check failed");
      },
      "127.0.0.1",
      PORT,
      5,
    ),
  ).rejects.toThrow("a check failed");
  await expect(fetch(`http://127.0.0.1:${PORT}/ok`)).rejects.toThrow();
});
