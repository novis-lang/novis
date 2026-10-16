// Loops forever for `/loop`, and adds 1 MiB to an array forever for `/alloc`.
// Every other path returns `ok`.
export default {
  fetch(req: Request): Response {
    const path = new URL(req.url).pathname;
    if (path === "/loop") {
      let turns = 0;
      while (true) turns++;
    }
    if (path === "/alloc") {
      const chunks: Uint8Array[] = [];
      while (true) chunks.push(new Uint8Array(1048576).fill(1));
    }
    return new Response("ok\n", { headers: { "Content-Type": "text/html; charset=UTF-8" } });
  },
};
