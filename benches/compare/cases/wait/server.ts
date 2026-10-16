// Waits 50 ms on a timer, then answers. This measures the cost of many
// requests in flight at once, as when a page waits for another service.

const text = { "Content-Type": "text/plain; charset=UTF-8" };

export default {
  async fetch(req: Request): Promise<Response> {
    if (new URL(req.url).pathname !== "/wait") {
      return new Response("not found\n", { status: 404, headers: text });
    }
    await Bun.sleep(50);
    return new Response("waited 50 ms\n", { headers: text });
  },
};
