// A `POST` to `/change` changes the account and returns `changed`.
export default function (req, res) {
  if (req.method === "POST" && new URL(req.url, "http://localhost").pathname === "/change") {
    // This is the type Bun gives a text answer with no `Content-Type`.
    res.writeHead(200, { "Content-Type": "text/plain;charset=utf-8" });
    res.end("changed\n");
    return;
  }
  res.writeHead(404);
  res.end();
}
