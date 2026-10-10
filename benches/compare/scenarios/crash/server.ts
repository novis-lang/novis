// Throws an error for the path `/crash`, and returns `ok` for every other path.
export default {
  fetch(req: Request): Response {
    if (new URL(req.url).pathname === "/crash") throw new Error("the handler failed");
    return new Response("ok\n", { headers: { "Content-Type": "text/html; charset=UTF-8" } });
  },
};
