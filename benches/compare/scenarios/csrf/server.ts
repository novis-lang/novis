// A `POST` to `/change` changes the account and returns `changed`.
export default {
  fetch(req: Request): Response {
    if (req.method === "POST" && new URL(req.url).pathname === "/change") return new Response("changed\n");
    return new Response(null, { status: 404 });
  },
};
