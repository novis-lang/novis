// Loops forever for `/loop`, and adds 1 MiB to an array forever for `/alloc`.
// Every other path returns `ok`.
export default function (req, res) {
  const path = new URL(req.url, "http://localhost").pathname;
  if (path === "/loop") {
    let turns = 0;
    while (true) turns++;
  }
  if (path === "/alloc") {
    const chunks = [];
    while (true) chunks.push(new Uint8Array(1048576).fill(1));
  }
  res.writeHead(200, { "Content-Type": "text/html; charset=UTF-8" });
  res.end("ok\n");
}
