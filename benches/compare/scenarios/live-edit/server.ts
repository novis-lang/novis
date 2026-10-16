// Returns the version of this file. The driver changes `1` to `2` while the server runs.
export default {
  fetch(): Response {
    return new Response("version 1\n", { headers: { "Content-Type": "text/html; charset=UTF-8" } });
  },
};
