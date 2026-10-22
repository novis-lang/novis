// `/hold` keeps 8 MiB for 300 ms and then returns `held`.
// Every other path returns `ok`.
export default {
  async fetch(req: Request): Promise<Response> {
    const headers = { "Content-Type": "text/html; charset=UTF-8" };
    if (new URL(req.url).pathname === "/hold") {
      const held = new Uint8Array(8388608).fill(1);
      await Bun.sleep(300);
      return new Response(held[held.length - 1] === 1 ? "held\n" : "", { headers });
    }
    return new Response("ok\n", { headers });
  },
};
